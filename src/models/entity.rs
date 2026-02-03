//! Entity (person/organization) model

use super::*;
use serde::{Deserialize, Serialize};

/// Entity representing a person or organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    #[serde(rename = "objectClassName", default)]
    pub object_class_name: Option<String>,

    #[serde(rename = "rdapConformance", default)]
    pub conformance: Vec<String>,

    #[serde(default)]
    pub notices: Vec<Notice>,

    #[serde(default)]
    pub handle: Option<String>,

    #[serde(rename = "vcardArray", default)]
    pub vcard: Option<VCard>,

    #[serde(default)]
    pub roles: Vec<String>,

    #[serde(rename = "publicIds", default)]
    pub public_ids: Vec<PublicId>,

    #[serde(default)]
    pub entities: Vec<Entity>,

    #[serde(default)]
    pub remarks: Vec<Remark>,

    #[serde(default)]
    pub links: Vec<Link>,

    #[serde(default)]
    pub events: Vec<Event>,

    #[serde(rename = "asEventActor", default)]
    pub as_event_actor: Vec<Event>,

    #[serde(default)]
    pub status: Status,

    #[serde(default)]
    pub port43: Option<String>,

    #[serde(default)]
    pub networks: Vec<IpNetwork>,

    #[serde(default)]
    pub autnums: Vec<Autnum>,

    #[serde(default)]
    pub lang: Option<String>,
}
