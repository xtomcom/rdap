//! RDAP client implementation

use crate::bootstrap::BootstrapClient;
use crate::error::{RdapError, Result};
use crate::models::RdapObject;
use crate::request::RdapRequest;
use reqwest::Client;
use std::time::Duration;
use url::Url;

/// RDAP client
pub struct RdapClient {
    http_client: Client,
    bootstrap: BootstrapClient,
    timeout: Duration,
}

impl RdapClient {
    /// Create a new RDAP client
    pub fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("rdap-rust/", env!("CARGO_PKG_VERSION")))
            .build()?;
        
        let bootstrap = BootstrapClient::new()?;
        
        Ok(Self {
            http_client,
            bootstrap,
            timeout: Duration::from_secs(30),
        })
    }
    
    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Execute an RDAP request
    pub async fn query(&self, request: &RdapRequest) -> Result<RdapObject> {
        // Determine RDAP servers
        let urls = if let Some(server) = &request.server {
            vec![server.clone()]
        } else {
            self.bootstrap.lookup(request).await?
        };
        
        if urls.is_empty() {
            return Err(RdapError::Bootstrap("No RDAP servers found".to_string()));
        }
        
        // Try each server
        let mut last_error = None;
        
        for base_url in &urls {
            let url = request.build_url(base_url)?;
            
            log::debug!("Querying RDAP server: {}", url);
            
            match self.fetch_rdap(&url).await {
                Ok(obj) => return Ok(obj),
                Err(RdapError::NotFound) => return Err(RdapError::NotFound),
                Err(e) => {
                    log::warn!("Server {} failed: {}", url, e);
                    last_error = Some(e);
                }
            }
        }
        
        Err(last_error.unwrap_or(RdapError::NoWorkingServers))
    }
    
    /// Fetch RDAP response from URL
    async fn fetch_rdap(&self, url: &Url) -> Result<RdapObject> {
        let response = self
            .http_client
            .get(url.as_str())
            .header("Accept", "application/rdap+json, application/json")
            .send()
            .await?;
        
        let status = response.status();
        
        if status.is_success() {
            let text = response.text().await?;
            let obj = self.parse_response(&text)?;
            Ok(obj)
        } else if status.as_u16() == 404 {
            Err(RdapError::NotFound)
        } else {
            // Try to parse as error response
            let text = response.text().await?;
            if let Ok(err_obj) = serde_json::from_str::<crate::models::ErrorResponse>(&text) {
                Err(RdapError::ServerError {
                    code: err_obj.error_code.unwrap_or(status.as_u16()),
                    title: err_obj.title.unwrap_or_else(|| "Unknown error".to_string()),
                    description: err_obj.description,
                })
            } else {
                Err(RdapError::Other(format!("HTTP error: {}", status)))
            }
        }
    }
    
    /// Parse RDAP JSON response
    fn parse_response(&self, json: &str) -> Result<RdapObject> {
        // First, parse as generic JSON to inspect structure
        let value: serde_json::Value = serde_json::from_str(json)?;
        
        // Detect object type
        if let Some(obj) = value.as_object() {
            // Check for error
            if obj.contains_key("errorCode") {
                return Ok(RdapObject::Error(serde_json::from_value(value)?));
            }
            
            // Check for search results
            if obj.contains_key("domainSearchResults") {
                return Ok(RdapObject::DomainSearch(serde_json::from_value(value)?));
            }
            if obj.contains_key("entitySearchResults") {
                return Ok(RdapObject::EntitySearch(serde_json::from_value(value)?));
            }
            if obj.contains_key("nameserverSearchResults") {
                return Ok(RdapObject::NameserverSearch(serde_json::from_value(value)?));
            }
            
            // Check objectClassName
            if let Some(class_name) = obj.get("objectClassName").and_then(|v| v.as_str()) {
                match class_name {
                    "domain" => return Ok(RdapObject::Domain(serde_json::from_value(value)?)),
                    "entity" => return Ok(RdapObject::Entity(serde_json::from_value(value)?)),
                    "nameserver" => return Ok(RdapObject::Nameserver(serde_json::from_value(value)?)),
                    "autnum" => return Ok(RdapObject::Autnum(serde_json::from_value(value)?)),
                    "ip network" => return Ok(RdapObject::IpNetwork(serde_json::from_value(value)?)),
                    _ => {}
                }
            }
            
            // Default to Help
            Ok(RdapObject::Help(serde_json::from_value(value)?))
        } else {
            Err(RdapError::Json(serde::de::Error::custom("Invalid RDAP response")))
        }
    }
}

impl Default for RdapClient {
    fn default() -> Self {
        Self::new().expect("Failed to create RDAP client")
    }
}
