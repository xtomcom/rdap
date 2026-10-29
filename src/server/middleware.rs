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
