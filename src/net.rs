//! Outbound HTTP: every request to bootstrap and RDAP servers goes through
//! [`HttpPool`], which owns proxy / source-address selection and limits.
//!
//! # Routes
//!
//! A *route* is one way out to the internet, backed by its own
//! [`reqwest::Client`] (and therefore its own connection pool):
//!
//! - with `proxies`: one route per proxy. `bind` addresses then apply to the
//!   connection *to the proxy*; proxy `i` uses the `i`-th bind address
//!   (cycling), taken from the addresses of the proxy's family when the proxy
//!   host is an IP literal.
//! - without proxies: one route per `bind` address.
//! - otherwise a single direct route (which honours the usual `*_PROXY`
//!   environment variables, like reqwest does by default).
//!
//! `interface` applies to every route (to the proxy connection when proxied).
//!
//! # Rotation and failover
//!
//! Each request starts on the next route in round-robin order. When it fails
//! before any HTTP status was received (connect error, proxy handshake
//! failure, timeout before the response head) it is retried on the next
//! route, up to `min(routes, 3)` attempts. Once a status was received the
//! request is never retried. A route that just failed is put in a short
//! cooldown during which rotation tries the other routes first.
//!
//! # IPv4 / IPv6 source addresses
//!
//! A bound route only connects to destinations of its own address family:
//! hyper's connector skips addresses of the other family instead of
//! connecting from an unbound socket, so the host's default address never
//! leaks. When the bind addresses mix IPv4 and IPv6, the destination's
//! families are resolved before each request and only routes that can reach
//! it are used.

use crate::error::{RdapError, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use url::Url;

/// Default cap on a single response body (16 MiB)
pub const DEFAULT_MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Default time a failed route is skipped by rotation
pub const DEFAULT_ROUTE_COOLDOWN: Duration = Duration::from_secs(10);

/// Maximum attempts (distinct routes) per request
const MAX_ATTEMPTS: usize = 3;

/// Upper bound for the TCP connect timeout, so dead routes fail over quickly
const MAX_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Supported proxy URL schemes
const PROXY_SCHEMES: [&str; 4] = ["socks5", "socks5h", "http", "https"];

/// Default port for SOCKS proxies without an explicit port
const SOCKS_DEFAULT_PORT: u16 = 1080;

/// Outbound network options
#[derive(Debug, Clone)]
pub struct NetOptions {
    /// Per-request timeout
    pub timeout: Duration,
    /// Accept invalid TLS certificates
    pub insecure: bool,
    /// Network interface to send requests from (e.g. `eth1`)
    pub interface: Option<String>,
    /// Source addresses to send requests from, rotated per request
    pub bind: Vec<IpAddr>,
    /// Proxy URLs (`socks5://`, `socks5h://`, `http://`, `https://`), rotated per request
    pub proxies: Vec<String>,
    /// Maximum concurrent requests per upstream host (0 = unlimited)
    pub max_per_host: usize,
    /// Maximum response body size in bytes (0 = unlimited)
    pub max_body_bytes: usize,
    /// How long a route that just failed is skipped by rotation (zero disables)
    pub route_cooldown: Duration,
}

impl Default for NetOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            insecure: false,
            interface: None,
            bind: Vec::new(),
            proxies: Vec::new(),
            max_per_host: 0,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            route_cooldown: DEFAULT_ROUTE_COOLDOWN,
        }
    }
}

/// A fully read HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Replace the userinfo (credentials) of a proxy URL with `***`.
///
/// Works on unparsable input too, so it is safe to use in error messages.
pub fn redact_proxy_url(raw: &str) -> String {
    let raw = raw.trim();
    let (scheme, rest) = raw
        .split_once("://")
        .map_or((None, raw), |(s, r)| (Some(s), r));
    // Use the last '@' so unencoded '@' or '/' in a password are still hidden
    let rest = rest
        .rfind('@')
        .map_or_else(|| rest.to_owned(), |i| format!("***@{}", &rest[i + 1..]));
    scheme.map_or_else(|| rest.clone(), |s| format!("{s}://{rest}"))
}

/// Validate a proxy URL: scheme socks5/socks5h/http/https, a host, no path.
///
/// SOCKS URLs without a port get port 1080. Credentials go in the userinfo
/// (`socks5h://user:pass@host:1080`), percent-encoded where needed.
pub fn parse_proxy_url(raw: &str) -> Result<Url> {
    let redacted = redact_proxy_url(raw);
    let mut url = Url::parse(raw.trim())
        .map_err(|e| RdapError::Config(format!("invalid proxy URL {redacted}: {e}")))?;
    if !PROXY_SCHEMES.contains(&url.scheme()) {
        return Err(RdapError::Config(format!(
            "unsupported proxy scheme in {redacted} (expected socks5, socks5h, http or https)"
        )));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(RdapError::Config(format!(
            "proxy URL {redacted} has no host"
        )));
    }
    if !matches!(url.path(), "" | "/") || url.query().is_some() || url.fragment().is_some() {
        return Err(RdapError::Config(format!(
            "proxy URL {redacted} must not have a path, query or fragment"
        )));
    }
    if url.port_or_known_default().is_none() {
        // Only SOCKS schemes have no known default port
        let _ = url.set_port(Some(SOCKS_DEFAULT_PORT));
    }
    Ok(url)
}

/// Redacted `scheme://[***@]host:port` label for a parsed proxy URL
fn proxy_label(url: &Url) -> String {
    let auth = if url.username().is_empty() && url.password().is_none() {
        ""
    } else {
        "***@"
    };
    let host = url.host_str().unwrap_or_default();
    let port = url.port_or_known_default().unwrap_or_default();
    format!("{}://{auth}{host}:{port}", url.scheme())
}

/// IP literal host of a URL (`[::1]` and `10.0.0.1` alike), if any
fn host_ip(url: &Url) -> Option<IpAddr> {
    url.host_str()?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse()
        .ok()
}

/// Address families a destination can be reached over
#[derive(Debug, Clone, Copy, Default)]
struct Families {
    v4: bool,
    v6: bool,
}

impl Families {
    const fn add(&mut self, ip: IpAddr) {
        match ip {
            IpAddr::V4(_) => self.v4 = true,
            IpAddr::V6(_) => self.v6 = true,
        }
    }

    const fn contains(self, ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(_) => self.v4,
            IpAddr::V6(_) => self.v6,
        }
    }
}

/// Resolve which address families `url`'s host has (`None` = unknown)
async fn destination_families(url: &Url) -> Option<Families> {
    let mut families = Families::default();
    match url.host()? {
        url::Host::Ipv4(_) => families.v4 = true,
        url::Host::Ipv6(_) => families.v6 = true,
        url::Host::Domain(domain) => {
            let port = url.port_or_known_default().unwrap_or(443);
            for addr in tokio::net::lookup_host((domain, port)).await.ok()? {
                families.add(addr.ip());
            }
        }
    }
    (families.v4 || families.v6).then_some(families)
}

/// One outbound route
struct Route {
    client: reqwest::Client,
    /// Redacted description for logs and errors
    label: String,
    /// Source address of a direct (non-proxied) bound route
    source: Option<IpAddr>,
    is_proxy: bool,
    /// Strings that must never appear in error messages (proxy credentials)
    secrets: Vec<String>,
    /// Pool-relative milliseconds until which rotation skips this route
    cooldown_until: AtomicU64,
}

impl Route {
    /// Convert a reqwest error, keeping proxy credentials out of the message
    fn error(&self, e: reqwest::Error) -> RdapError {
        if e.is_timeout() {
            return RdapError::Timeout;
        }
        if self.is_proxy && e.is_connect() {
            return RdapError::Proxy(self.scrub(&format!("{}: {}", self.label, error_chain(&e))));
        }
        RdapError::Http(e)
    }

    fn scrub(&self, message: &str) -> String {
        self.secrets
            .iter()
            .fold(message.to_owned(), |msg, secret| msg.replace(secret, "***"))
    }
}

/// `error: source: source...` in one line
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(err) = source {
        let text = err.to_string();
        if !message.ends_with(&text) {
            message.push_str(": ");
            message.push_str(&text);
        }
        source = err.source();
    }
    message
}

/// Whether a `send()` error happened before any response was received and
/// another route might succeed
fn is_route_failure(e: &reqwest::Error) -> bool {
    e.is_connect() || e.is_timeout()
}

/// Pool of HTTP clients, one per outbound route (proxy or source address)
pub struct HttpPool {
    routes: Vec<Route>,
    next: AtomicUsize,
    max_body_bytes: usize,
    max_per_host: usize,
    host_limits: Mutex<HashMap<String, Arc<Semaphore>>>,
    cooldown: Duration,
    epoch: Instant,
    /// Bound routes mix IPv4 and IPv6 sources: filter routes per destination
    mixed_families: bool,
}

impl HttpPool {
    /// Build the pool from options. Fails on invalid proxy URLs, a missing
    /// interface or options not supported on this platform.
    pub fn new(opts: &NetOptions) -> Result<Self> {
        let proxies = opts
            .proxies
            .iter()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| parse_proxy_url(p).map(|url| (p, url)))
            .collect::<Result<Vec<_>>>()?;

        let interface = opts
            .interface
            .as_deref()
            .map(str::trim)
            .filter(|i| !i.is_empty());
        let via = interface.map_or_else(String::new, |i| format!(" dev {i}"));

        let mut routes = Vec::new();
        if !proxies.is_empty() {
            for (i, (raw, url)) in proxies.iter().enumerate() {
                let label = proxy_label(url);
                let local = proxy_source(&opts.bind, url, i, &label)?;
                let proxy = reqwest::Proxy::all(url.as_str()).map_err(|e| {
                    RdapError::Config(format!("invalid proxy {label}: {}", error_chain(&e)))
                })?;
                let mut secrets = vec![redact_secret(raw, url)];
                secrets.extend(
                    [url.username(), url.password().unwrap_or_default()]
                        .into_iter()
                        .filter(|s| s.len() >= 4)
                        .map(str::to_owned),
                );
                secrets.retain(|s| !s.is_empty());
                let client = base_builder(opts, interface)?
                    .proxy(proxy)
                    .local_address(local)
                    .build()?;
                let source = local.map_or_else(String::new, |ip| format!(" from {ip}"));
                routes.push(Route {
                    client,
                    label: format!("proxy {label}{source}{via}"),
                    source: None,
                    is_proxy: true,
                    secrets,
                    cooldown_until: AtomicU64::new(0),
                });
            }
        } else if !opts.bind.is_empty() {
            for &ip in &opts.bind {
                routes.push(Route {
                    client: base_builder(opts, interface)?.local_address(ip).build()?,
                    label: format!("bind {ip}{via}"),
                    source: Some(ip),
                    is_proxy: false,
                    secrets: Vec::new(),
                    cooldown_until: AtomicU64::new(0),
                });
            }
        } else {
            routes.push(Route {
                client: base_builder(opts, interface)?.build()?,
                label: format!("direct{via}"),
                source: None,
                is_proxy: false,
                secrets: Vec::new(),
                cooldown_until: AtomicU64::new(0),
            });
        }

        let mixed_families = routes
            .iter()
            .any(|r| r.source.is_some_and(|ip| ip.is_ipv4()))
            && routes
                .iter()
                .any(|r| r.source.is_some_and(|ip| ip.is_ipv6()));

        Ok(Self {
            routes,
            next: AtomicUsize::new(0),
            max_body_bytes: opts.max_body_bytes,
            max_per_host: opts.max_per_host,
            host_limits: Mutex::new(HashMap::new()),
            cooldown: opts.route_cooldown,
            epoch: Instant::now(),
            mixed_families,
        })
    }

    /// Number of outbound routes in rotation
    pub const fn routes(&self) -> usize {
        self.routes.len()
    }

    /// Redacted descriptions of the routes (for startup logs)
    pub fn route_labels(&self) -> Vec<String> {
        self.routes.iter().map(|r| r.label.clone()).collect()
    }

    /// GET `url` with the given `Accept` header and read the whole body.
    /// `timeout` overrides the pool's default timeout for this request
    /// (per attempt, including reading the body).
    pub async fn get(
        &self,
        url: &Url,
        accept: &str,
        timeout: Option<Duration>,
    ) -> Result<HttpResponse> {
        // Held until the body is fully read
        let _permit = self.host_permit(url).await;
        let order = self.route_order(url).await;
        let attempts = order.len();

        let mut last_error = None;
        for (attempt, idx) in order.into_iter().enumerate() {
            let route = &self.routes[idx];
            let mut request = route
                .client
                .get(url.as_str())
                .header(reqwest::header::ACCEPT, accept);
            if let Some(timeout) = timeout {
                request = request.timeout(timeout);
            }

            match request.send().await {
                Ok(response) => {
                    route.cooldown_until.store(0, Ordering::Relaxed);
                    log::debug!("GET {url} via {}: HTTP {}", route.label, response.status());
                    return self.read_body(response).await.map_err(|e| match e {
                        RdapError::Http(e) => route.error(e),
                        other => other,
                    });
                }
                Err(e) if is_route_failure(&e) => {
                    let error = route.error(e);
                    // The last failure is returned to the caller, which reports it
                    let level = if attempt + 1 < attempts {
                        log::Level::Warn
                    } else {
                        log::Level::Debug
                    };
                    log::log!(
                        level,
                        "GET {url} via {} failed (attempt {}/{attempts}): {error}",
                        route.label,
                        attempt + 1
                    );
                    if self.routes.len() > 1 {
                        let until = self.now_ms().saturating_add(duration_ms(self.cooldown));
                        route.cooldown_until.store(until, Ordering::Relaxed);
                    }
                    last_error = Some(error);
                }
                Err(e) => return Err(route.error(e)),
            }
        }

        Err(last_error.unwrap_or(RdapError::NoWorkingServers))
    }

    /// Read the body, enforcing `max_body_bytes` up front and while streaming
    async fn read_body(&self, mut response: reqwest::Response) -> Result<HttpResponse> {
        let status = response.status().as_u16();
        let limit = self.max_body_bytes;
        let too_large = || RdapError::BodyTooLarge { limit };

        let expected = response.content_length();
        if limit > 0 && expected.is_some_and(|len| len > limit as u64) {
            return Err(too_large());
        }

        let capacity = expected.map_or(0, |len| usize::try_from(len).unwrap_or(0).min(1 << 20));
        let mut body = Vec::with_capacity(capacity);
        while let Some(chunk) = response.chunk().await? {
            if limit > 0 && body.len() + chunk.len() > limit {
                return Err(too_large());
            }
            body.extend_from_slice(&chunk);
        }

        Ok(HttpResponse { status, body })
    }

    /// Wait for a slot for `url`'s host when `max_per_host` is set
    async fn host_permit(&self, url: &Url) -> Option<OwnedSemaphorePermit> {
        if self.max_per_host == 0 {
            return None;
        }
        let key = format!(
            "{}:{}",
            url.host_str().unwrap_or_default(),
            url.port_or_known_default().unwrap_or_default()
        );
        let semaphore = Arc::clone(
            self.host_limits
                .lock()
                .unwrap()
                .entry(key)
                .or_insert_with(|| Arc::new(Semaphore::new(self.max_per_host))),
        );
        // The semaphore is never closed
        semaphore.acquire_owned().await.ok()
    }

    /// Routes to try for one request, in order: round-robin start, routes
    /// that can't reach the destination's address family dropped, routes in
    /// cooldown last, at most `MAX_ATTEMPTS`.
    async fn route_order(&self, url: &Url) -> Vec<usize> {
        let n = self.routes.len();
        let start = self.next.fetch_add(1, Ordering::Relaxed) % n;
        let rotation = (0..n).map(|i| (start + i) % n);

        let families = if self.mixed_families {
            destination_families(url).await
        } else {
            None
        };
        let now = self.now_ms();

        let mut ready = Vec::with_capacity(n);
        let mut cooling = Vec::new();
        for idx in rotation.clone() {
            let route = &self.routes[idx];
            if let (Some(families), Some(source)) = (families, route.source)
                && !families.contains(source)
            {
                continue;
            }
            if route.cooldown_until.load(Ordering::Relaxed) > now {
                cooling.push(idx);
            } else {
                ready.push(idx);
            }
        }
        ready.extend(cooling);
        if ready.is_empty() {
            // No route matches the destination's families: let the error surface
            ready.extend(rotation);
        }
        ready.truncate(MAX_ATTEMPTS);
        ready
    }

    fn now_ms(&self) -> u64 {
        duration_ms(self.epoch.elapsed())
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// The raw userinfo of a proxy URL as written (scrubbed from errors)
fn redact_secret(raw: &str, url: &Url) -> String {
    if url.username().is_empty() && url.password().is_none() {
        return String::new();
    }
    let rest = raw.split_once("://").map_or(raw, |(_, r)| r);
    rest.rfind('@')
        .map_or_else(String::new, |i| rest[..i].to_owned())
}

/// Source address for the connection to proxy number `index`
fn proxy_source(bind: &[IpAddr], proxy: &Url, index: usize, label: &str) -> Result<Option<IpAddr>> {
    if bind.is_empty() {
        return Ok(None);
    }
    // A hostname proxy may resolve to either family: any bind address goes
    let proxy_ip = host_ip(proxy);
    let candidates: Vec<IpAddr> = bind
        .iter()
        .copied()
        .filter(|ip| proxy_ip.is_none_or(|p| p.is_ipv4() == ip.is_ipv4()))
        .collect();
    if candidates.is_empty() {
        return Err(RdapError::Config(format!(
            "no --bind address of the same IP family as proxy {label}"
        )));
    }
    Ok(Some(candidates[index % candidates.len()]))
}

/// Client builder with the options shared by all routes
fn base_builder(opts: &NetOptions, interface: Option<&str>) -> Result<reqwest::ClientBuilder> {
    let builder = reqwest::Client::builder()
        .timeout(opts.timeout)
        .connect_timeout(opts.timeout.min(MAX_CONNECT_TIMEOUT))
        .user_agent(concat!("rdap-rust/", env!("CARGO_PKG_VERSION")))
        .danger_accept_invalid_certs(opts.insecure);
    match interface {
        Some(interface) => with_interface(builder, interface),
        None => Ok(builder),
    }
}

/// Bind all connections to a network interface
#[cfg(any(
    target_os = "android",
    target_os = "fuchsia",
    target_os = "illumos",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
    target_os = "solaris",
    target_os = "tvos",
    target_os = "visionos",
    target_os = "watchos",
))]
fn with_interface(
    builder: reqwest::ClientBuilder,
    interface: &str,
) -> Result<reqwest::ClientBuilder> {
    let name = std::ffi::CString::new(interface)
        .map_err(|_| RdapError::Config(format!("invalid interface name {interface:?}")))?;
    // SAFETY: `name` is a valid NUL-terminated string that outlives the call
    if unsafe { libc::if_nametoindex(name.as_ptr()) } == 0 {
        return Err(RdapError::Config(format!(
            "network interface {interface:?} not found"
        )));
    }
    Ok(builder.interface(interface))
}

/// Bind all connections to a network interface (unsupported here)
#[cfg(not(any(
    target_os = "android",
    target_os = "fuchsia",
    target_os = "illumos",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
    target_os = "solaris",
    target_os = "tvos",
    target_os = "visionos",
    target_os = "watchos",
)))]
#[allow(clippy::needless_pass_by_value)]
fn with_interface(
    _builder: reqwest::ClientBuilder,
    interface: &str,
) -> Result<reqwest::ClientBuilder> {
    Err(RdapError::Config(format!(
        "--interface {interface} is not supported on this platform (use --bind instead)"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_proxy_url() {
        assert_eq!(
            redact_proxy_url("socks5h://user:pass@10.0.0.1:1080"),
            "socks5h://***@10.0.0.1:1080"
        );
        assert_eq!(
            redact_proxy_url("http://u:p@ss/w@rd@proxy:3128"),
            "http://***@proxy:3128"
        );
        assert_eq!(redact_proxy_url("socks5://host:1080"), "socks5://host:1080");
        assert_eq!(redact_proxy_url("user:secret@host"), "***@host");
    }

    #[test]
    fn test_parse_proxy_url() {
        for ok in [
            "socks5://127.0.0.1:1080",
            "socks5h://user:pass@proxy.example:1080",
            "http://proxy.example:3128",
            "https://proxy.example",
            "socks5://[::1]:1080",
        ] {
            assert!(parse_proxy_url(ok).is_ok(), "{ok}");
        }
        assert_eq!(
            parse_proxy_url("socks5h://10.0.0.1").unwrap().port(),
            Some(SOCKS_DEFAULT_PORT)
        );
        for bad in [
            "ftp://proxy:21",
            "socks4://proxy:1080",
            "proxy.example:1080",
            "socks5://",
            "http://proxy:3128/path",
            "not a url",
        ] {
            assert!(parse_proxy_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn test_parse_proxy_url_error_hides_credentials() {
        for bad in ["ftp://alice:hunter2@proxy:21", "socks5://alice:hunter2@"] {
            let err = parse_proxy_url(bad).unwrap_err().to_string();
            assert!(!err.contains("hunter2") && !err.contains("alice"), "{err}");
        }
    }

    #[test]
    fn test_proxy_label() {
        let url = parse_proxy_url("socks5h://alice:hunter2@10.0.0.1:1080").unwrap();
        assert_eq!(proxy_label(&url), "socks5h://***@10.0.0.1:1080");
        assert_eq!(
            redact_secret("socks5h://alice:hunter2@10.0.0.1:1080", &url),
            "alice:hunter2"
        );
    }

    #[test]
    fn test_proxy_source_family() {
        let v4: IpAddr = "192.0.2.1".parse().unwrap();
        let v6: IpAddr = "2001:db8::1".parse().unwrap();
        let bind = [v4, v6];
        let proxy4 = parse_proxy_url("socks5://10.0.0.1:1080").unwrap();
        let proxy6 = parse_proxy_url("socks5://[2001:db8::2]:1080").unwrap();
        let named = parse_proxy_url("socks5://proxy.example:1080").unwrap();
        assert_eq!(proxy_source(&bind, &proxy4, 1, "p").unwrap(), Some(v4));
        assert_eq!(proxy_source(&bind, &proxy6, 0, "p").unwrap(), Some(v6));
        assert_eq!(proxy_source(&bind, &named, 1, "p").unwrap(), Some(v6));
        assert_eq!(proxy_source(&[], &named, 0, "p").unwrap(), None);
        assert!(proxy_source(&[v4], &proxy6, 0, "p").is_err());
    }

    #[test]
    fn test_invalid_proxy_rejected_at_startup() {
        let opts = NetOptions {
            proxies: vec!["socks5://ok:1080".into(), "gopher://bob:s3cret@x".into()],
            ..NetOptions::default()
        };
        let err = HttpPool::new(&opts).err().unwrap();
        assert!(matches!(err, RdapError::Config(_)));
        assert!(!err.to_string().contains("s3cret"));
    }

    #[test]
    fn test_route_count() {
        let bind = vec!["127.0.0.1".parse().unwrap(), "127.0.0.2".parse().unwrap()];
        let opts = NetOptions {
            bind: bind.clone(),
            ..NetOptions::default()
        };
        assert_eq!(HttpPool::new(&opts).unwrap().routes(), 2);
        let opts = NetOptions {
            bind,
            proxies: vec!["socks5://127.0.0.1:1080".into(), " ".into()],
            ..NetOptions::default()
        };
        let pool = HttpPool::new(&opts).unwrap();
        assert_eq!(pool.routes(), 1);
        assert_eq!(
            pool.route_labels(),
            ["proxy socks5://127.0.0.1:1080 from 127.0.0.1"]
        );
        assert_eq!(HttpPool::new(&NetOptions::default()).unwrap().routes(), 1);
    }
}
