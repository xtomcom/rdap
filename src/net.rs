//! Outbound HTTP: every request to bootstrap and RDAP servers goes through
//! [`HttpPool`], which owns proxy / source-address selection and limits.

use crate::error::{RdapError, Result};
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use url::Url;

/// Default cap on a single response body (16 MiB)
pub const DEFAULT_MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

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
    /// Maximum response body size in bytes
    pub max_body_bytes: usize,
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
        }
    }
}

/// A fully read HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Pool of HTTP clients, one per outbound route (proxy or source address)
pub struct HttpPool {
    clients: Vec<reqwest::Client>,
    next: AtomicUsize,
    max_body_bytes: usize,
}

impl HttpPool {
    /// Build the pool from options
    pub fn new(opts: &NetOptions) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(opts.timeout)
            .user_agent(concat!("rdap-rust/", env!("CARGO_PKG_VERSION")))
            .danger_accept_invalid_certs(opts.insecure)
            .build()?;

        Ok(Self {
            clients: vec![client],
            next: AtomicUsize::new(0),
            max_body_bytes: opts.max_body_bytes,
        })
    }

    /// Number of outbound routes in rotation
    pub const fn routes(&self) -> usize {
        self.clients.len()
    }

    /// GET `url` with the given `Accept` header and read the whole body.
    /// `timeout` overrides the pool's default timeout for this request.
    pub async fn get(
        &self,
        url: &Url,
        accept: &str,
        timeout: Option<Duration>,
    ) -> Result<HttpResponse> {
        let idx = self.next.fetch_add(1, Ordering::Relaxed) % self.clients.len();
        let mut request = self.clients[idx]
            .get(url.as_str())
            .header(reqwest::header::ACCEPT, accept);
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        let response = request.send().await?;

        let status = response.status().as_u16();
        let body = response.bytes().await?;
        if body.len() > self.max_body_bytes {
            return Err(RdapError::Other(format!(
                "Response body exceeds {} bytes",
                self.max_body_bytes
            )));
        }

        Ok(HttpResponse {
            status,
            body: body.to_vec(),
        })
    }
}
