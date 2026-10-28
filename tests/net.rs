//! Outbound networking: rotation, failover, limits and source binding,
//! against local HTTP / SOCKS5 servers only.

use rdap::RdapError;
use rdap::net::{HttpPool, NetOptions};
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

const ACCEPT: &str = "application/rdap+json";

/// Minimal HTTP/1.1 server: every connection gets the same canned response
struct TestServer {
    addr: SocketAddr,
    /// Peer address of every connection, in accept order
    peers: Arc<Mutex<Vec<SocketAddr>>>,
    /// Highest number of requests handled at once
    max_active: Arc<AtomicUsize>,
}

impl TestServer {
    /// `response` is written verbatim after `delay`; `hang` keeps the
    /// connection open afterwards without finishing the body
    async fn start(bind: &str, response: &'static str, delay: Duration, hang: bool) -> Self {
        let listener = TcpListener::bind(bind).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let peers = Arc::new(Mutex::new(Vec::new()));
        let max_active = Arc::new(AtomicUsize::new(0));
        let active = Arc::new(AtomicUsize::new(0));
        {
            let (peers, max_active) = (Arc::clone(&peers), Arc::clone(&max_active));
            tokio::spawn(async move {
                loop {
                    let (mut stream, peer) = listener.accept().await.unwrap();
                    peers.lock().unwrap().push(peer);
                    let (active, max_active) = (Arc::clone(&active), Arc::clone(&max_active));
                    tokio::spawn(async move {
                        read_head(&mut stream).await;
                        let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(now, Ordering::SeqCst);
                        tokio::time::sleep(delay).await;
                        let _ = stream.write_all(response.as_bytes()).await;
                        active.fetch_sub(1, Ordering::SeqCst);
                        if hang {
                            tokio::time::sleep(Duration::from_secs(60)).await;
                        }
                    });
                }
            });
        }
        Self {
            addr,
            peers,
            max_active,
        }
    }

    async fn ok(bind: &str) -> Self {
        Self::start(bind, OK, Duration::ZERO, false).await
    }

    fn url(&self) -> Url {
        Url::parse(&format!("http://{}/ip/192.0.2.1", self.addr)).unwrap()
    }

    fn peer_ips(&self) -> Vec<IpAddr> {
        self.peers
            .lock()
            .unwrap()
            .iter()
            .map(SocketAddr::ip)
            .collect()
    }
}

const OK: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/rdap+json\r\n\
                  Content-Length: 2\r\nConnection: close\r\n\r\n{}";

/// Read until the end of the request head
async fn read_head(stream: &mut TcpStream) {
    let mut head = Vec::new();
    let mut buf = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
}

/// Minimal SOCKS5 proxy (CONNECT only); optional username/password auth
struct Socks5 {
    addr: SocketAddr,
    /// Peer address of every client connection
    peers: Arc<Mutex<Vec<SocketAddr>>>,
}

impl Socks5 {
    async fn start(auth: Option<(&'static str, &'static str)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let peers = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&peers);
        tokio::spawn(async move {
            loop {
                let (stream, peer) = listener.accept().await.unwrap();
                log.lock().unwrap().push(peer);
                tokio::spawn(async move {
                    let _ = Self::handle(stream, auth).await;
                });
            }
        });
        Self { addr, peers }
    }

    async fn handle(mut client: TcpStream, auth: Option<(&str, &str)>) -> std::io::Result<()> {
        let mut head = [0u8; 2];
        client.read_exact(&mut head).await?;
        let mut methods = vec![0u8; usize::from(head[1])];
        client.read_exact(&mut methods).await?;
        if let Some((user, pass)) = auth {
            client.write_all(&[5, 2]).await?;
            let mut ver_len = [0u8; 2];
            client.read_exact(&mut ver_len).await?;
            let mut got_user = vec![0u8; usize::from(ver_len[1])];
            client.read_exact(&mut got_user).await?;
            let mut pass_len = [0u8; 1];
            client.read_exact(&mut pass_len).await?;
            let mut got_pass = vec![0u8; usize::from(pass_len[0])];
            client.read_exact(&mut got_pass).await?;
            let ok = got_user == user.as_bytes() && got_pass == pass.as_bytes();
            client.write_all(&[1, u8::from(!ok)]).await?;
            if !ok {
                return Ok(());
            }
        } else {
            client.write_all(&[5, 0]).await?;
        }

        let mut req = [0u8; 4];
        client.read_exact(&mut req).await?;
        let host = match req[3] {
            1 => {
                let mut ip = [0u8; 4];
                client.read_exact(&mut ip).await?;
                IpAddr::from(ip).to_string()
            }
            4 => {
                let mut ip = [0u8; 16];
                client.read_exact(&mut ip).await?;
                IpAddr::from(ip).to_string()
            }
            _ => {
                let mut len = [0u8; 1];
                client.read_exact(&mut len).await?;
                let mut name = vec![0u8; usize::from(len[0])];
                client.read_exact(&mut name).await?;
                String::from_utf8_lossy(&name).into_owned()
            }
        };
        let mut port = [0u8; 2];
        client.read_exact(&mut port).await?;
        let mut upstream = TcpStream::connect((host.as_str(), u16::from_be_bytes(port))).await?;
        client.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await?;
        tokio::io::copy_bidirectional(&mut client, &mut upstream).await?;
        Ok(())
    }

    fn url(&self) -> String {
        format!("socks5://{}", self.addr)
    }

    fn connections(&self) -> usize {
        self.peers.lock().unwrap().len()
    }
}

/// Listener that accepts connections and drops them (handshake failure)
async fn closing_listener() -> (SocketAddr, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&accepted);
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            count.fetch_add(1, Ordering::SeqCst);
            drop(stream);
        }
    });
    (addr, accepted)
}

/// Listener that accepts connections and never answers
async fn black_hole() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let mut held = Vec::new();
        loop {
            held.push(listener.accept().await.unwrap().0);
        }
    });
    addr
}

fn pool(opts: NetOptions) -> HttpPool {
    HttpPool::new(&opts).unwrap()
}

fn proxies(urls: &[String]) -> NetOptions {
    NetOptions {
        proxies: urls.to_vec(),
        ..NetOptions::default()
    }
}

#[tokio::test]
async fn rotates_proxies_round_robin() {
    let server = TestServer::ok("127.0.0.1:0").await;
    let (a, b) = (Socks5::start(None).await, Socks5::start(None).await);
    let pool = pool(proxies(&[a.url(), b.url()]));
    assert_eq!(pool.routes(), 2);

    for expected in [(1, 0), (1, 1), (2, 1), (2, 2)] {
        let response = pool.get(&server.url(), ACCEPT, None).await.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"{}");
        assert_eq!((a.connections(), b.connections()), expected);
    }
}

#[tokio::test]
async fn socks5_with_credentials() {
    let server = TestServer::ok("127.0.0.1:0").await;
    let proxy = Socks5::start(Some(("alice", "p@ss word"))).await;
    let url = format!("socks5h://alice:p%40ss%20word@{}", proxy.addr);
    let response = pool(proxies(&[url])).get(&server.url(), ACCEPT, None).await;
    assert_eq!(response.unwrap().status, 200);
    assert_eq!(proxy.connections(), 1);
}

#[tokio::test]
async fn fails_over_from_dead_proxy() {
    let server = TestServer::ok("127.0.0.1:0").await;
    let good = Socks5::start(None).await;
    let pool = pool(proxies(&["socks5://127.0.0.1:1".into(), good.url()]));

    // Starts on the dead proxy, succeeds on the second route
    let response = pool.get(&server.url(), ACCEPT, None).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(good.connections(), 1);
}

#[tokio::test]
async fn fails_over_on_timeout_before_response() {
    let server = TestServer::ok("127.0.0.1:0").await;
    let good = Socks5::start(None).await;
    let hole = black_hole().await;
    let pool = pool(NetOptions {
        timeout: Duration::from_millis(500),
        ..proxies(&[format!("socks5://{hole}"), good.url()])
    });

    let response = pool.get(&server.url(), ACCEPT, None).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(good.connections(), 1);
}

#[tokio::test]
async fn failed_route_cools_down() {
    let server = TestServer::ok("127.0.0.1:0").await;
    let good = Socks5::start(None).await;
    let (bad, bad_accepts) = closing_listener().await;
    let pool = pool(proxies(&[format!("socks5://{bad}"), good.url()]));

    for _ in 0..4 {
        let response = pool.get(&server.url(), ACCEPT, None).await.unwrap();
        assert_eq!(response.status, 200);
    }
    // Only the first request tried the broken proxy; later rotations skip it
    assert_eq!(bad_accepts.load(Ordering::SeqCst), 1);
    assert_eq!(good.connections(), 4);
}

#[tokio::test]
async fn all_routes_dead_is_proxy_error_without_credentials() {
    let pool = pool(proxies(&[
        "socks5h://alice:hunter2@127.0.0.1:1".into(),
        "http://bob:swordfish@127.0.0.1:1".into(),
    ]));
    let url = Url::parse("http://192.0.2.1/ip/192.0.2.1").unwrap();
    let err = pool.get(&url, ACCEPT, None).await.unwrap_err();
    assert!(matches!(err, RdapError::Proxy(_)), "{err:?}");
    let message = format!("{err} {err:?}");
    for secret in ["alice", "hunter2", "bob", "swordfish"] {
        assert!(!message.contains(secret), "{message}");
    }
    assert!(message.contains("***@127.0.0.1:1"), "{message}");
}

#[tokio::test]
async fn no_retry_after_http_status() {
    let server = TestServer::start(
        "127.0.0.1:0",
        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        Duration::ZERO,
        false,
    )
    .await;
    let (a, b) = (Socks5::start(None).await, Socks5::start(None).await);
    let pool = pool(proxies(&[a.url(), b.url()]));

    let response = pool.get(&server.url(), ACCEPT, None).await.unwrap();
    assert_eq!(response.status, 503);
    assert_eq!(a.connections() + b.connections(), 1);
}

#[tokio::test]
async fn no_retry_on_body_timeout() {
    let server = TestServer::start(
        "127.0.0.1:0",
        "HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{\"partial\":",
        Duration::ZERO,
        true,
    )
    .await;
    let (a, b) = (Socks5::start(None).await, Socks5::start(None).await);
    let pool = pool(NetOptions {
        timeout: Duration::from_millis(500),
        ..proxies(&[a.url(), b.url()])
    });

    let err = pool.get(&server.url(), ACCEPT, None).await.unwrap_err();
    assert!(matches!(err, RdapError::Timeout), "{err:?}");
    assert_eq!(a.connections() + b.connections(), 1);
}

#[tokio::test]
async fn per_request_timeout_override() {
    let server = TestServer::start("127.0.0.1:0", OK, Duration::from_secs(5), false).await;
    let pool = pool(NetOptions::default());
    let started = std::time::Instant::now();
    let err = pool
        .get(&server.url(), ACCEPT, Some(Duration::from_millis(300)))
        .await
        .unwrap_err();
    assert!(matches!(err, RdapError::Timeout), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[tokio::test]
async fn body_limit_content_length() {
    let server = TestServer::start(
        "127.0.0.1:0",
        "HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n",
        Duration::ZERO,
        true,
    )
    .await;
    let pool = pool(NetOptions {
        max_body_bytes: 10,
        ..NetOptions::default()
    });
    // Rejected from the header alone: the body never arrives
    let err = pool.get(&server.url(), ACCEPT, None).await.unwrap_err();
    assert!(
        matches!(err, RdapError::BodyTooLarge { limit: 10 }),
        "{err:?}"
    );
}

const CHUNKED: &str = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n\
                       8\r\n01234567\r\n8\r\n89abcdef\r\n8\r\nghijklmn\r\n0\r\n\r\n";

#[tokio::test]
async fn body_limit_streamed() {
    let server = TestServer::start("127.0.0.1:0", CHUNKED, Duration::ZERO, false).await;
    let limited = pool(NetOptions {
        max_body_bytes: 20,
        ..NetOptions::default()
    });
    let err = limited.get(&server.url(), ACCEPT, None).await.unwrap_err();
    assert!(
        matches!(err, RdapError::BodyTooLarge { limit: 20 }),
        "{err:?}"
    );

    let roomy = pool(NetOptions {
        max_body_bytes: 24,
        ..NetOptions::default()
    });
    let response = roomy.get(&server.url(), ACCEPT, None).await.unwrap();
    assert_eq!(response.body, b"0123456789abcdefghijklmn");
}

#[tokio::test]
async fn per_host_limit() {
    let server = TestServer::start("127.0.0.1:0", OK, Duration::from_millis(150), false).await;
    let pool = Arc::new(pool(NetOptions {
        max_per_host: 2,
        ..NetOptions::default()
    }));

    let tasks: Vec<_> = (0..6)
        .map(|_| {
            let (pool, url) = (Arc::clone(&pool), server.url());
            tokio::spawn(async move { pool.get(&url, ACCEPT, None).await.unwrap().status })
        })
        .collect();
    for task in tasks {
        assert_eq!(task.await.unwrap(), 200);
    }
    assert_eq!(server.max_active.load(Ordering::SeqCst), 2);
}

/// Linux routes all of 127.0.0.0/8 to loopback, so any of it can be a source
#[cfg(target_os = "linux")]
#[tokio::test]
async fn rotates_bind_addresses() {
    let server = TestServer::ok("127.0.0.1:0").await;
    let pool = pool(NetOptions {
        bind: vec!["127.0.0.1".parse().unwrap(), "127.0.0.2".parse().unwrap()],
        ..NetOptions::default()
    });
    for _ in 0..4 {
        pool.get(&server.url(), ACCEPT, None).await.unwrap();
    }
    let expected: Vec<IpAddr> = ["127.0.0.1", "127.0.0.2", "127.0.0.1", "127.0.0.2"]
        .iter()
        .map(|ip| ip.parse().unwrap())
        .collect();
    assert_eq!(server.peer_ips(), expected);
}

/// `--bind` with `--proxy` binds the connection to the proxy
#[cfg(target_os = "linux")]
#[tokio::test]
async fn bind_applies_to_proxy_connection() {
    let server = TestServer::ok("127.0.0.1:0").await;
    let proxy = Socks5::start(None).await;
    let pool = pool(NetOptions {
        bind: vec!["127.0.0.3".parse().unwrap()],
        ..proxies(&[proxy.url()])
    });
    pool.get(&server.url(), ACCEPT, None).await.unwrap();
    let peers: Vec<IpAddr> = proxy
        .peers
        .lock()
        .unwrap()
        .iter()
        .map(SocketAddr::ip)
        .collect();
    assert_eq!(peers, ["127.0.0.3".parse::<IpAddr>().unwrap()]);
}

/// With mixed-family bind addresses, only routes of the destination's family
/// are used (and a v4 source never connects to a v6 destination unbound)
#[tokio::test]
async fn mixed_family_bind_uses_matching_routes() {
    let Ok(listener) = std::net::TcpListener::bind("[::1]:0") else {
        eprintln!("IPv6 loopback unavailable, skipping");
        return;
    };
    drop(listener);
    let server6 = TestServer::ok("[::1]:0").await;
    let server4 = TestServer::ok("127.0.0.1:0").await;
    let pool = pool(NetOptions {
        bind: vec!["127.0.0.1".parse().unwrap(), "::1".parse().unwrap()],
        ..NetOptions::default()
    });
    for _ in 0..3 {
        assert_eq!(
            pool.get(&server6.url(), ACCEPT, None).await.unwrap().status,
            200
        );
        assert_eq!(
            pool.get(&server4.url(), ACCEPT, None).await.unwrap().status,
            200
        );
    }
    assert!(
        server6
            .peer_ips()
            .iter()
            .all(|ip| *ip == "::1".parse::<IpAddr>().unwrap())
    );
    assert!(
        server4
            .peer_ips()
            .iter()
            .all(|ip| *ip == "127.0.0.1".parse::<IpAddr>().unwrap())
    );

    // A v4-only route can't reach a v6 destination: error, no unbound connection
    let v4_only = HttpPool::new(&NetOptions {
        bind: vec!["127.0.0.1".parse().unwrap()],
        ..NetOptions::default()
    })
    .unwrap();
    let before = server6.peer_ips().len();
    assert!(v4_only.get(&server6.url(), ACCEPT, None).await.is_err());
    assert_eq!(server6.peer_ips().len(), before);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn interface_binding() {
    let server = TestServer::ok("127.0.0.1:0").await;
    let pool = pool(NetOptions {
        interface: Some("lo".into()),
        ..NetOptions::default()
    });
    assert_eq!(
        pool.get(&server.url(), ACCEPT, None).await.unwrap().status,
        200
    );

    let err = HttpPool::new(&NetOptions {
        interface: Some("nonexistent0".into()),
        ..NetOptions::default()
    })
    .err()
    .unwrap();
    assert!(matches!(err, RdapError::Config(_)), "{err:?}");
}
