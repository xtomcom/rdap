//! Authentication and request timeout

use super::AppState;
use super::error::ApiError;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;

/// Upper bound on handling one request (a large `/batch` included)
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

/// Require `Authorization: Bearer <token>` when a token is configured
/// (`/healthz` is always open, for load balancers and monitoring)
pub async fn require_token(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(token) = &state.opts.token else {
        // No token on a loopback address: only accept loopback Host names, so a
        // web page can't reach the API through DNS rebinding
        if state.opts.listen.is_loopback() && !is_loopback_host(&request) {
            return ApiError::new(
                StatusCode::FORBIDDEN,
                "Host not allowed: without a token only localhost / loopback Host names are accepted",
            )
            .into_response();
        }
        return next.run(request).await;
    };
    if request.uri().path() == "/healthz" {
        return next.run(request).await;
    }

    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().split_once(' '))
        .is_some_and(|(scheme, credentials)| {
            scheme.eq_ignore_ascii_case("bearer")
                && bool::from(credentials.trim().as_bytes().ct_eq(token.as_bytes()))
        });

    if authorized {
        next.run(request).await
    } else {
        ApiError::unauthorized().into_response()
    }
}

/// Answer 504 instead of hanging when a request takes too long
pub async fn timeout(request: Request, next: Next) -> Response {
    tokio::time::timeout(REQUEST_TIMEOUT, next.run(request))
        .await
        .unwrap_or_else(|_| {
            ApiError::new(StatusCode::GATEWAY_TIMEOUT, "Request timed out").into_response()
        })
}

/// Whether the request's `Host` header names this machine (`localhost`,
/// `*.localhost` or a loopback IP); a missing Host (HTTP/1.0) counts as local
fn is_loopback_host(request: &Request) -> bool {
    let Some(host) = request.headers().get(header::HOST) else {
        return true;
    };
    let Ok(host) = host.to_str() else {
        return false;
    };
    // Strip the port: "[::1]:8888", "127.0.0.1:8888", "localhost"
    let name = host.strip_prefix('[').map_or_else(
        || host.rsplit_once(':').map_or(host, |(name, _)| name),
        |rest| rest.split(']').next().unwrap_or_default(),
    );
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    name == "localhost"
        || name.ends_with(".localhost")
        || name
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}
