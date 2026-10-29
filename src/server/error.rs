//! API errors, rendered as RFC 7480 §5.3 / RFC 9083 error objects

use crate::error::RdapError;
use crate::models::ErrorResponse;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value};

/// Media type of RDAP responses
pub const RDAP_JSON: &str = "application/rdap+json";
/// Media type of everything else
pub const JSON: &str = "application/json";

/// An error response: status code plus human-readable description lines
#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    description: Vec<String>,
    /// Rendered as a full RDAP error object (`application/rdap+json`, `rdapConformance`)
    rdap: bool,
}

impl ApiError {
    /// Create an error with one description line
    pub fn new(status: StatusCode, description: impl Into<String>) -> Self {
        Self {
            status,
            description: vec![description.into()],
            rdap: false,
        }
    }

    pub fn bad_request(description: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, description)
    }

    pub fn not_found(description: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, description)
    }

    pub fn bad_gateway(description: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, description)
    }

    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "Missing or invalid bearer token")
    }

    /// Render as an RDAP error object (for the RFC 7482 endpoints)
    pub const fn rdap(mut self) -> Self {
        self.rdap = true;
        self
    }

    pub const fn status(&self) -> StatusCode {
        self.status
    }

    pub fn title(&self) -> &'static str {
        self.status.canonical_reason().unwrap_or("Error")
    }

    /// The JSON error object
    pub fn body(&self) -> Value {
        let mut body = Map::new();
        if self.rdap {
            body.insert(
                "rdapConformance".to_owned(),
                Value::Array(vec![Value::from("rdap_level_0")]),
            );
        }
        body.insert("errorCode".to_owned(), Value::from(self.status.as_u16()));
        body.insert("title".to_owned(), Value::from(self.title()));
        body.insert(
            "description".to_owned(),
            Value::Array(
                self.description
                    .iter()
                    .map(|d| Value::from(d.as_str()))
                    .collect(),
            ),
        );
        Value::Object(body)
    }

    /// Map an error from the RDAP client. `query` is the (normalized) query,
    /// used in descriptions.
    pub fn from_rdap(err: &RdapError, query: &str) -> Self {
        match err {
            RdapError::InvalidQuery(msg) => Self::bad_request(msg.clone()),
            RdapError::NotFound => Self::not_found(format!("No RDAP record for {query}")),
            RdapError::NoServer(_) => Self::not_found(format!("No RDAP server known for {query}")),
            RdapError::NeedsServer(_) => Self::needs_upstream(),
            RdapError::ServerError {
                code,
                title,
                description,
            } => {
                let mut lines = vec![format!("Upstream RDAP server returned {code}: {title}")];
                lines.extend(description.iter().cloned());
                Self {
                    status: upstream_status(*code),
                    description: lines,
                    rdap: false,
                }
            }
            RdapError::Http(e) if e.is_timeout() => Self::timeout(),
            RdapError::Timeout => Self::timeout(),
            RdapError::Http(e) => Self::bad_gateway(format!("Upstream request failed: {e}")),
            RdapError::Json(e) => {
                Self::bad_gateway(format!("Unparsable upstream RDAP response: {e}"))
            }
            RdapError::NoWorkingServers => Self::bad_gateway("No working RDAP servers found"),
            // Details (proxy host, source address) are logged, not shown to clients
            RdapError::Proxy(_) => Self::bad_gateway(
                "Could not reach the upstream RDAP server through the configured proxy",
            ),
            other => Self::bad_gateway(other.to_string()),
        }
    }

    /// Map an RDAP error object an upstream returned with a 2xx status
    pub fn from_error_object(err: &ErrorResponse, query: &str) -> Self {
        let status = err
            .error_code
            .map_or(StatusCode::BAD_GATEWAY, upstream_status);
        let mut description = if status == StatusCode::NOT_FOUND {
            vec![format!("No RDAP record for {query}")]
        } else {
            vec![format!(
                "Upstream RDAP server returned an error object: {}",
                err.title.as_deref().unwrap_or("no title")
            )]
        };
        description.extend(err.description.iter().cloned());
        Self {
            status,
            description,
            rdap: false,
        }
    }

    /// Query type that can't be routed through bootstrap
    pub fn needs_upstream() -> Self {
        Self::new(
            StatusCode::NOT_IMPLEMENTED,
            "This query type needs an explicit upstream RDAP server (start the server with -s/--server)",
        )
    }

    fn timeout() -> Self {
        Self::new(
            StatusCode::GATEWAY_TIMEOUT,
            "Upstream RDAP server timed out",
        )
    }
}

/// Status code to return for an upstream HTTP status / RDAP `errorCode`
const fn upstream_status(code: u16) -> StatusCode {
    match code {
        400 | 422 => StatusCode::BAD_REQUEST,
        404 | 410 => StatusCode::NOT_FOUND,
        429 => StatusCode::TOO_MANY_REQUESTS,
        501 => StatusCode::NOT_IMPLEMENTED,
        504 => StatusCode::GATEWAY_TIMEOUT,
        _ => StatusCode::BAD_GATEWAY,
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let content_type = if self.rdap { RDAP_JSON } else { JSON };
        let mut response = (
            self.status,
            [(header::CONTENT_TYPE, HeaderValue::from_static(content_type))],
            self.body().to_string(),
        )
            .into_response();
        if self.status == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_of(err: &RdapError) -> u16 {
        ApiError::from_rdap(err, "q").status().as_u16()
    }

    #[test]
    fn maps_client_errors() {
        assert_eq!(status_of(&RdapError::InvalidQuery("x".into())), 400);
        assert_eq!(status_of(&RdapError::NotFound), 404);
        // Bootstrap had no server for this TLD / IP / ASN
        assert_eq!(status_of(&RdapError::NoServer("x.example".into())), 404);
        assert_eq!(status_of(&RdapError::NeedsServer("entity".into())), 501);
        assert_eq!(
            status_of(&RdapError::Bootstrap(
                "Failed to fetch registry: HTTP 500".into()
            )),
            502
        );
        assert_eq!(status_of(&RdapError::Timeout), 504);
        assert_eq!(status_of(&RdapError::NoWorkingServers), 502);
        assert_eq!(status_of(&RdapError::Other("proxy failed".into())), 502);
        for (code, expected) in [(429, 429), (410, 404), (500, 502), (403, 502), (400, 400)] {
            let err = RdapError::ServerError {
                code,
                title: "t".into(),
                description: vec![],
            };
            assert_eq!(status_of(&err), expected, "upstream {code}");
        }
    }

    #[test]
    fn error_body_shape() {
        let body = ApiError::not_found("No RDAP record for x").body();
        assert_eq!(body["errorCode"], 404);
        assert_eq!(body["title"], "Not Found");
        assert_eq!(body["description"][0], "No RDAP record for x");
        assert!(body.get("rdapConformance").is_none());
        let body = ApiError::not_found("x").rdap().body();
        assert_eq!(body["rdapConformance"][0], "rdap_level_0");
    }
}
