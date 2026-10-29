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
use axum::http::{HeaderValue, header};
use axum::routing::{get, post};
use cache::ResultCache;
use colored::Colorize;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
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
    /// Maximum number of cached query results
    pub cache_capacity: u64,
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
            cache_capacity: 10_000,
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
}

impl AppState {
    pub fn new(opts: ServeOptions, client: Arc<RdapClient>) -> Self {
        let tlds = TldList::load()
            .map_err(|e| log::warn!("Failed to load TLD list: {e}"))
            .ok();
        let cache = ResultCache::new(opts.cache_ttl, opts.cache_capacity);
        Self {
            opts,
            client,
            tlds,
            cache,
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
        app = app.layer(CorsLayer::permissive());
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
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    println!("Server stopped");
    Ok(())
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
