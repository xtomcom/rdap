//! IP Network model

use super::*;
use serde::{Deserialize, Serialize};

/// IP Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpNetwork {
    #[serde(rename = "objectClassName", default)]
    pub object_class_name: Option<String>,
    
    #[serde(rename = "rdapConformance", default)]
    pub conformance: Vec<String>,
    
    #[serde(default)]
    pub notices: Vec<Notice>,
    
    #[serde(default)]
    pub handle: Option<String>,
    
    #[serde(rename = "startAddress", default)]
    pub start_address: Option<String>,
    
    #[serde(rename = "endAddress", default)]
    pub end_address: Option<String>,
    
    #[serde(rename = "ipVersion", default)]
    pub ip_version: Option<String>,
    
    #[serde(default)]
    pub name: Option<String>,
    
    #[serde(rename = "type", default)]
    pub network_type: Option<String>,
    
    #[serde(default)]
    pub country: Option<String>,
    
    #[serde(rename = "parentHandle", default)]
    pub parent_handle: Option<String>,
    
    #[serde(default)]
    pub status: Status,
    
    #[serde(default)]
    pub entities: Vec<Entity>,
    
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
