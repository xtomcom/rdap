//! Nameserver model

use super::*;
use serde::{Deserialize, Serialize};

/// Nameserver information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nameserver {
    #[serde(rename = "objectClassName", default)]
    pub object_class_name: Option<String>,

    #[serde(rename = "rdapConformance", default)]
    pub conformance: Vec<String>,

    #[serde(default)]
    pub notices: Vec<Notice>,

    #[serde(default)]
    pub handle: Option<String>,

    #[serde(rename = "ldhName", default)]
    pub ldh_name: Option<String>,

    #[serde(rename = "unicodeName", default)]
    pub unicode_name: Option<String>,

    #[serde(rename = "ipAddresses", default)]
    pub ip_addresses: Option<IpAddressSet>,

    #[serde(default)]
    pub entities: Vec<Entity>,

    #[serde(default)]
    pub status: Status,

    #[serde(default)]
    pub remarks: Vec<Remark>,

    #[serde(default)]
    pub links: Vec<Link>,

    #[serde(default)]
    pub port43: Option<String>,

    #[serde(default)]
    pub events: Vec<Event>,

    #[serde(default)]
    pub lang: Option<String>,
}

/// IP address set for nameserver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpAddressSet {
    #[serde(default)]
    pub v4: Vec<String>,

    #[serde(default)]
    pub v6: Vec<String>,
}
