//! Autonomous System Number model

use super::*;
use serde::{Deserialize, Serialize};

/// Autonomous System Number information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Autnum {
    #[serde(rename = "objectClassName", default)]
    pub object_class_name: Option<String>,
    
    #[serde(rename = "rdapConformance", default)]
    pub conformance: Vec<String>,
    
    #[serde(default)]
    pub notices: Vec<Notice>,
    
    #[serde(default)]
    pub handle: Option<String>,
    
    #[serde(rename = "startAutnum", default)]
    pub start_autnum: Option<u32>,
    
    #[serde(rename = "endAutnum", default)]
    pub end_autnum: Option<u32>,
    
    #[serde(rename = "ipVersion", default)]
    pub ip_version: Option<String>,
    
    #[serde(default)]
    pub name: Option<String>,
    
    #[serde(rename = "type", default)]
    pub as_type: Option<String>,
    
    #[serde(default)]
    pub status: Status,
    
    #[serde(default)]
    pub country: Option<String>,
    
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
