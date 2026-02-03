//! Error response model

use super::*;
use serde::{Deserialize, Serialize};

/// RDAP error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    #[serde(rename = "rdapConformance", default)]
    pub conformance: Vec<String>,

    #[serde(default)]
    pub notices: Vec<Notice>,

    #[serde(rename = "errorCode", default)]
    pub error_code: Option<u16>,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub description: Vec<String>,

    #[serde(default)]
    pub lang: Option<String>,
}
