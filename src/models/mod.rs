//! RDAP data models

pub mod autnum;
pub mod common;
pub mod domain;
pub mod entity;
pub mod error;
pub mod ip_network;
pub mod nameserver;
pub mod search;
pub mod vcard;

pub use autnum::Autnum;
pub use common::*;
pub use domain::Domain;
pub use entity::Entity;
pub use error::ErrorResponse;
pub use ip_network::IpNetwork;
pub use nameserver::Nameserver;
pub use search::*;
pub use vcard::VCard;

use serde::{Deserialize, Serialize};

/// Top-level RDAP response object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RdapObject {
    Domain(Domain),
    Entity(Entity),
    Nameserver(Nameserver),
    Autnum(Autnum),
    IpNetwork(IpNetwork),
    Error(ErrorResponse),
    DomainSearch(DomainSearchResults),
    EntitySearch(EntitySearchResults),
    NameserverSearch(NameserverSearchResults),
    Help(HelpResponse),
}

/// Help response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpResponse {
    #[serde(rename = "rdapConformance", default)]
    pub conformance: Vec<String>,

    #[serde(default)]
    pub notices: Vec<Notice>,

    #[serde(default)]
    pub lang: Option<String>,
}
