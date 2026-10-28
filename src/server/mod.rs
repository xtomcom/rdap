//! Local HTTP API server (`rdap --serve`)

use crate::RdapClient;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

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
    /// Maximum number of cached query results
    pub cache_capacity: u64,
    /// Send permissive CORS headers
    pub cors: bool,
    /// Maximum number of queries in one `/batch` request
    pub max_batch: usize,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            listen: DEFAULT_LISTEN.parse().unwrap(),
            port: DEFAULT_PORT,
            token: None,
            cache_ttl: Duration::from_secs(300),
            cache_capacity: 10_000,
            cors: false,
            max_batch: 100,
        }
    }
}

/// Run the API server until SIGINT/SIGTERM
pub async fn serve(
    opts: ServeOptions,
    client: Arc<RdapClient>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = (opts, client);
    tokio::task::yield_now().await;
    Err("API server not implemented yet".into())
}
