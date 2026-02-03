//! Search result models

use super::*;
use serde::{Deserialize, Serialize};

/// Domain search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSearchResults {
    #[serde(rename = "rdapConformance", default)]
    pub conformance: Vec<String>,

    #[serde(default)]
    pub notices: Vec<Notice>,

    #[serde(rename = "domainSearchResults", default)]
    pub domains: Vec<Domain>,

    #[serde(default)]
    pub lang: Option<String>,
}

/// Entity search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySearchResults {
    #[serde(rename = "rdapConformance", default)]
    pub conformance: Vec<String>,

    #[serde(default)]
    pub notices: Vec<Notice>,

    #[serde(rename = "entitySearchResults", default)]
    pub entities: Vec<Entity>,

    #[serde(default)]
    pub lang: Option<String>,
}

/// Nameserver search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameserverSearchResults {
    #[serde(rename = "rdapConformance", default)]
    pub conformance: Vec<String>,

    #[serde(default)]
    pub notices: Vec<Notice>,

    #[serde(rename = "nameserverSearchResults", default)]
    pub nameservers: Vec<Nameserver>,

    #[serde(default)]
    pub lang: Option<String>,
}
