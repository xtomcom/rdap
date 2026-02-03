//! Common RDAP structures

use serde::{Deserialize, Serialize};

/// Link to related resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    #[serde(default)]
    pub value: Option<String>,

    #[serde(default)]
    pub rel: Option<String>,

    pub href: String,

    #[serde(default)]
    pub hreflang: Vec<String>,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub media: Option<String>,

    #[serde(rename = "type", default)]
    pub link_type: Option<String>,
}

/// Notice or remark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notice {
    #[serde(default)]
    pub title: Option<String>,

    #[serde(rename = "type", default)]
    pub notice_type: Option<String>,

    #[serde(default)]
    pub description: Vec<String>,

    #[serde(default)]
    pub links: Vec<Link>,
}

/// Event information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "eventAction")]
    pub action: String,

    #[serde(rename = "eventActor", default)]
    pub actor: Option<String>,

    #[serde(rename = "eventDate")]
    pub date: String,

    #[serde(default)]
    pub links: Vec<Link>,
}

/// Public identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicId {
    #[serde(rename = "type")]
    pub id_type: String,

    pub identifier: String,
}

/// Status values
pub type Status = Vec<String>;

/// Remark (same structure as Notice)
pub type Remark = Notice;
