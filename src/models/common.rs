//! Common RDAP structures

use serde::{Deserialize, Serialize};

/// Deserialize a field that can be either a single string or an array of strings
fn string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrVec;

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or an array of strings")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Vec<String>, E> {
            Ok(vec![value.to_owned()])
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<String>, A::Error> {
            let mut vec = Vec::new();
            while let Some(val) = seq.next_element()? {
                vec.push(val);
            }
            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

/// Link to related resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    #[serde(default)]
    pub value: Option<String>,

    #[serde(default)]
    pub rel: Option<String>,

    pub href: String,

    #[serde(default, deserialize_with = "string_or_vec")]
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
