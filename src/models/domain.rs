//! Domain object model

use super::*;
use serde::{Deserialize, Serialize};

/// Domain name registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    #[serde(rename = "objectClassName")]
    pub object_class_name: String,

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

    #[serde(default)]
    pub variants: Vec<Variant>,

    #[serde(default)]
    pub nameservers: Vec<Nameserver>,

    #[serde(rename = "secureDNS", default)]
    pub secure_dns: Option<SecureDNS>,

    #[serde(default)]
    pub entities: Vec<Entity>,

    #[serde(default)]
    pub status: Status,

    #[serde(rename = "publicIds", default)]
    pub public_ids: Vec<PublicId>,

    #[serde(default)]
    pub remarks: Vec<Remark>,

    #[serde(default)]
    pub links: Vec<Link>,

    #[serde(default)]
    pub port43: Option<String>,

    #[serde(default)]
    pub events: Vec<Event>,

    #[serde(default)]
    pub network: Option<Box<IpNetwork>>,

    #[serde(default)]
    pub lang: Option<String>,
}

/// Domain variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    #[serde(default)]
    pub relation: Vec<String>,

    #[serde(rename = "idnTable", default)]
    pub idn_table: Option<String>,

    #[serde(rename = "variantNames", default)]
    pub variant_names: Vec<VariantName>,
}

/// Variant name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantName {
    #[serde(rename = "ldhName", default)]
    pub ldh_name: Option<String>,

    #[serde(rename = "unicodeName", default)]
    pub unicode_name: Option<String>,
}

/// DNSSEC information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureDNS {
    #[serde(rename = "zoneSigned", default)]
    pub zone_signed: Option<bool>,

    #[serde(rename = "delegationSigned", default)]
    pub delegation_signed: Option<bool>,

    #[serde(rename = "maxSigLife", default)]
    pub max_sig_life: Option<u64>,

    #[serde(rename = "dsData", default)]
    pub ds_data: Vec<DSData>,

    #[serde(rename = "keyData", default)]
    pub key_data: Vec<KeyData>,
}

/// DS record data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DSData {
    #[serde(rename = "keyTag", default)]
    pub key_tag: Option<u64>,

    #[serde(default)]
    pub algorithm: Option<u8>,

    #[serde(default)]
    pub digest: Option<String>,

    #[serde(rename = "digestType", default)]
    pub digest_type: Option<u8>,

    #[serde(default)]
    pub events: Vec<Event>,

    #[serde(default)]
    pub links: Vec<Link>,
}

/// DNSKEY data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyData {
    #[serde(default)]
    pub flags: Option<u16>,

    #[serde(default)]
    pub protocol: Option<u8>,

    #[serde(default)]
    pub algorithm: Option<u8>,

    #[serde(rename = "publicKey", default)]
    pub public_key: Option<String>,

    #[serde(default)]
    pub events: Vec<Event>,

    #[serde(default)]
    pub links: Vec<Link>,
}
