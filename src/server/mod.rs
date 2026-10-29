//! Local HTTP API server (`rdap --serve`)
//!
//! RDAP-compatible lookups (`/domain/{name}`, `/ip/{addr}`, ...) so any RDAP
//! client can use the server, plus JSON convenience endpoints (`/query`,
//! `/abuse`, `/batch`). Use [`router`] to embed or test the API and
//! [`serve`] to run it.

mod cache;
mod error;
mod lookup;
mod middleware;
mod routes;

pub use error::ApiError;

use crate::RdapClient;
use crate::config::TldList;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, header};
use axum::routing::{get, post};
use cache::ResultCache;
use colored::Colorize;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use url::Url;

/// Default listen address
pub const DEFAULT_LISTEN: &str = "127.0.0.1";
/// Default listen port
pub const DEFAULT_PORT: u16 = 8888;

/// API server options
#[derive(Debug, Clone)]
pub struct ServeOptions {
    /// Address to listen on
    pub listen: IpAddr,
    /// Port to listen on
    pub port: u16,
    /// Bearer token required on every endpoint except `/healthz` (`None` = no auth)
    pub token: Option<String>,
    /// How long query results are cached (zero disables the cache)
    pub cache_ttl: Duration,
    /// Approximate memory budget of the result cache, in bytes
    pub cache_bytes: u64,
    /// Send permissive CORS headers
    pub cors: bool,
    /// Maximum number of queries in one `/batch` request
    pub max_batch: usize,
    /// Send every query to this RDAP server instead of using bootstrap.
    /// Required for entity lookups and searches.
    pub upstream: Option<Url>,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            listen: DEFAULT_LISTEN.parse().unwrap(),
            port: DEFAULT_PORT,
            token: None,
            cache_ttl: Duration::from_secs(300),
            cache_bytes: 256 * 1024 * 1024,
            cors: false,
            max_batch: 100,
            upstream: None,
        }
    }
}

/// Shared server state
pub struct AppState {
    opts: ServeOptions,
    client: Arc<RdapClient>,
    tlds: Option<TldList>,
    cache: ResultCache,
    /// Upstream lookups running for `/batch` requests, across all clients
    batch_slots: Arc<Semaphore>,
}

/// Batch lookups running at once across all `/batch` requests, so one large
/// batch can't monopolize the per-host upstream slots
const BATCH_GLOBAL_CONCURRENCY: usize = 32;

impl AppState {
    pub fn new(opts: ServeOptions, client: Arc<RdapClient>) -> Self {
        let tlds = TldList::load()
            .map_err(|e| log::warn!("Failed to load TLD list: {e}"))
            .ok();
        let cache = ResultCache::new(opts.cache_ttl, opts.cache_bytes);
        Self {
            opts,
            client,
            tlds,
            cache,
            batch_slots: Arc::new(Semaphore::new(BATCH_GLOBAL_CONCURRENCY)),
        }
    }
}

/// Build the API router
pub fn router(state: Arc<AppState>) -> Router {
    let cors = state.opts.cors;
    let mut app = Router::new()
        // RDAP-compatible (RFC 7482)
        .route("/domain/{name}", get(routes::domain))
        .route("/ip/{*addr}", get(routes::ip))
        .route("/autnum/{asn}", get(routes::autnum))
        .route("/nameserver/{name}", get(routes::nameserver))
        .route("/entity/{handle}", get(routes::entity))
        .route("/domains", get(routes::domains))
        .route("/nameservers", get(routes::nameservers))
        .route("/entities", get(routes::entities))
        .route("/help", get(routes::help))
        // Convenience API
        .route("/query/{*q}", get(routes::query))
        .route("/abuse/{*q}", get(routes::abuse))
        .route(
            "/batch",
            post(routes::batch).layer(DefaultBodyLimit::max(routes::MAX_BATCH_BODY)),
        )
        .route("/healthz", get(routes::healthz))
        .route("/version", get(routes::version))
        .fallback(routes::not_found)
        .method_not_allowed_fallback(routes::method_not_allowed)
        .layer(axum::middleware::from_fn_with_state(
            Arc::clone(&state),
            middleware::require_token,
        ))
        .layer(axum::middleware::from_fn(middleware::timeout))
        .layer(CompressionLayer::new())
        .with_state(state);
    if cors {
        // Not `permissive()`: its `Allow-Headers: *` doesn't cover Authorization,
        // so browsers would fail the preflight for token-authenticated requests
        app = app.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
                .max_age(Duration::from_secs(3600)),
        );
    }
    app.layer(SetResponseHeaderLayer::overriding(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ))
}

/// Run the API server until SIGINT/SIGTERM
pub async fn serve(
    opts: ServeOptions,
    client: Arc<RdapClient>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::new(opts.listen, opts.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to listen on {addr}: {e}"))?;
    let local = listener.local_addr()?;

    if !opts.listen.is_loopback() && opts.token.is_none() {
        eprintln!(
            "{} listening on {} without a token: anyone who can reach this address can use \
             the server. Set RDAP_API_TOKEN (or --token) to require authentication.",
            "WARNING:".bright_red().bold(),
            local.ip()
        );
    }
    if let Some(upstream) = &opts.upstream {
        let upstream = crate::net::without_credentials(upstream);
        println!("Sending all queries to {}", upstream.as_str().cyan());
    }
    // Labels are credential-redacted
    println!("Outbound routes: {}", client.route_labels().join(", "));
    println!(
        "rdap API server listening on {}",
        format!("http://{local}").bright_green()
    );

    let app = router(Arc::new(AppState::new(opts, client)));
    serve_connections(listener, app, shutdown_signal()).await;
    println!("Server stopped");
    Ok(())
}

/// Clients must send complete request headers within this time, also for the
/// next request on a keep-alive connection (slowloris / idle connections)
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);
/// Open client connections at once (further clients wait in the accept
/// queue), lowered to half the open-file limit so outbound requests keep fds
const MAX_CONNECTIONS: usize = 1024;
/// Open connections per client IP, so one client can't take every slot
const MAX_CONNECTIONS_PER_IP: usize = 64;

fn max_connections() -> usize {
    #[cfg(unix)]
    {
        let mut limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: getrlimit only writes to the struct we pass
        if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &raw mut limit) } == 0 {
            let half = usize::try_from(limit.rlim_cur / 2).unwrap_or(usize::MAX);
            return half.clamp(16, MAX_CONNECTIONS);
        }
    }
    MAX_CONNECTIONS
}

/// Open connections per client IP; a slot is released when its guard drops
#[derive(Clone, Default)]
struct PeerSlots(Arc<std::sync::Mutex<std::collections::HashMap<IpAddr, usize>>>);

struct PeerSlot {
    slots: PeerSlots,
    ip: IpAddr,
}

impl PeerSlots {
    fn try_acquire(&self, ip: IpAddr) -> Option<PeerSlot> {
        let mut counts = self.0.lock().unwrap();
        let count = counts.entry(ip).or_insert(0);
        if *count >= MAX_CONNECTIONS_PER_IP {
            return None;
        }
        *count += 1;
        drop(counts);
        Some(PeerSlot {
            slots: self.clone(),
            ip,
        })
    }
}

impl Drop for PeerSlot {
    fn drop(&mut self) {
        let mut counts = self.slots.0.lock().unwrap();
        if let Some(count) = counts.get_mut(&self.ip) {
            *count -= 1;
            if *count == 0 {
                counts.remove(&self.ip);
            }
        }
    }
}
/// How long shutdown waits for in-flight requests
const SHUTDOWN_GRACE: Duration = Duration::from_secs(30);

/// Accept loop with header-read timeout, connection cap and graceful
/// shutdown (`axum::serve` has neither of the first two)
async fn serve_connections(
    listener: tokio::net::TcpListener,
    app: Router,
    shutdown: impl std::future::Future<Output = ()>,
) {
    use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
    use hyper_util::server::conn::auto::Builder;
    use hyper_util::server::graceful::GracefulShutdown;
    use hyper_util::service::TowerToHyperService;

    // HTTP/1 only: protocol auto-detection reads the first bytes without any
    // timeout, so a silent client could hold a connection forever
    let mut builder = Builder::new(TokioExecutor::new()).http1_only();
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(HEADER_READ_TIMEOUT);

    let graceful = GracefulShutdown::new();
    let slots = Arc::new(Semaphore::new(max_connections()));
    let peers = PeerSlots::default();
    let mut shutdown = std::pin::pin!(shutdown);

    loop {
        let permit = tokio::select! {
            permit = Arc::clone(&slots).acquire_owned() => permit.expect("semaphore never closed"),
            () = &mut shutdown => break,
        };
        let (stream, peer) = tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok(accepted) => accepted,
                Err(e) => {
                    // e.g. EMFILE: back off instead of spinning
                    log::warn!("Failed to accept connection: {e}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            },
            () = &mut shutdown => break,
        };
        let Some(peer_slot) = peers.try_acquire(peer.ip()) else {
            log::debug!("Too many connections from {}; closing", peer.ip());
            continue;
        };

        let service = TowerToHyperService::new(app.clone());
        let connection = builder
            .serve_connection(TokioIo::new(stream), service)
            .into_owned();
        let connection = graceful.watch(connection);
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::debug!("Connection from {peer} ended: {e}");
            }
            drop((permit, peer_slot));
        });
    }

    drop(listener);
    tokio::select! {
        () = graceful.shutdown() => {}
        () = tokio::time::sleep(SHUTDOWN_GRACE) => {
            log::warn!("Shutdown grace period elapsed; dropping open connections");
        }
    }
}

/// Resolve on SIGINT (Ctrl-C) or SIGTERM
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            log::error!("Failed to listen for Ctrl-C: {e}");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                log::error!("Failed to listen for SIGTERM: {e}");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
