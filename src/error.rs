//! Error types for the RDAP client

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RdapError>;

#[derive(Error, Debug)]
pub enum RdapError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Bootstrap error: {0}")]
    Bootstrap(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Object not found (404)")]
    NotFound,

    #[error("No working RDAP servers found")]
    NoWorkingServers,

    #[error("RDAP server error {code}: {title}")]
    ServerError {
        code: u16,
        title: String,
        description: Vec<String>,
    },

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Timeout")]
    Timeout,

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("{0}")]
    Other(String),
}
