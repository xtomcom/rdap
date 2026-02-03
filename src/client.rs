//! RDAP client implementation

use crate::bootstrap::BootstrapClient;
use crate::error::{RdapError, Result};
use crate::models::{Domain, RdapObject};
use crate::request::{QueryType, RdapRequest};
use reqwest::Client;
use std::time::Duration;
use url::Url;

/// RDAP query result with optional registrar data
#[derive(Debug, Clone)]
pub struct RdapQueryResult {
    /// Primary result from registry
    pub registry: RdapObject,
    /// URL of the registry RDAP server used
    pub registry_url: Url,
    /// Optional result from registrar (for domain queries)
    pub registrar: Option<RdapObject>,
    /// URL of the registrar RDAP server used (if any)
    pub registrar_url: Option<Url>,
}

/// RDAP client
pub struct RdapClient {
    http_client: Client,
    bootstrap: BootstrapClient,
    timeout: Duration,
    follow_referral: bool,
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
            follow_referral: true, // Enable by default
        })
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable or disable following registrar referrals (default: enabled)
    pub fn with_follow_referral(mut self, follow: bool) -> Self {
        self.follow_referral = follow;
        self
    }

    /// Execute an RDAP request (simple query, returns single object)
    pub async fn query(&self, request: &RdapRequest) -> Result<RdapObject> {
        let result = self.query_with_referral(request).await?;
        // If we have registrar data, prefer it; otherwise use registry data
        Ok(result.registrar.unwrap_or(result.registry))
    }

    /// Execute an RDAP request with registrar referral support
    pub async fn query_with_referral(&self, request: &RdapRequest) -> Result<RdapQueryResult> {
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
                Ok(obj) => {
                    // For domain queries (not TLD), try to follow registrar referral
                    if self.follow_referral
                        && request.query_type == QueryType::Domain
                        && let RdapObject::Domain(ref domain) = obj
                        && let Some(registrar_rdap_url) = self.extract_registrar_rdap_url(domain)
                    {
                        log::debug!("Following registrar referral: {}", registrar_rdap_url);
                        match self.fetch_rdap(&registrar_rdap_url).await {
                            Ok(registrar_obj) => {
                                return Ok(RdapQueryResult {
                                    registry: obj,
                                    registry_url: url,
                                    registrar: Some(registrar_obj),
                                    registrar_url: Some(registrar_rdap_url),
                                });
                            }
                            Err(e) => {
                                log::warn!("Failed to fetch registrar data: {}", e);
                                // Continue with registry-only result
                            }
                        }
                    }
                    return Ok(RdapQueryResult {
                        registry: obj,
                        registry_url: url,
                        registrar: None,
                        registrar_url: None,
                    });
                }
                Err(RdapError::NotFound) => return Err(RdapError::NotFound),
                Err(e) => {
                    log::warn!("Server {} failed: {}", url, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(RdapError::NoWorkingServers))
    }

    /// Extract registrar RDAP URL from domain response
    fn extract_registrar_rdap_url(&self, domain: &Domain) -> Option<Url> {
        // Look for a link with rel="related" and type containing "rdap"
        // This is the standard way registries indicate the registrar's RDAP server
        for link in &domain.links {
            if let Some(ref rel) = link.rel
                && rel == "related"
            {
                // Check if it's an RDAP link
                if let Some(ref link_type) = link.link_type
                    && (link_type.contains("rdap") || link_type.contains("json"))
                    && let Ok(url) = Url::parse(&link.href)
                {
                    return Some(url);
                }
                // Also try if href looks like an RDAP URL
                if link.href.contains("/domain/")
                    && let Ok(url) = Url::parse(&link.href)
                {
                    return Some(url);
                }
            }
        }

        // Also check entities for registrar with RDAP link
        for entity in &domain.entities {
            if entity.roles.iter().any(|r| r == "registrar") {
                for link in &entity.links {
                    if let Some(ref rel) = link.rel {
                        // Look for related links that point to domain RDAP
                        if rel == "related"
                            && link.href.contains("/domain/")
                            && let Ok(url) = Url::parse(&link.href)
                        {
                            return Some(url);
                        }
                    }
                }
            }
        }

        None
    }

    /// Fetch RDAP response from URL
    pub async fn fetch_rdap(&self, url: &Url) -> Result<RdapObject> {
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
                    "nameserver" => {
                        return Ok(RdapObject::Nameserver(serde_json::from_value(value)?));
                    }
                    "autnum" => return Ok(RdapObject::Autnum(serde_json::from_value(value)?)),
                    "ip network" => {
                        return Ok(RdapObject::IpNetwork(serde_json::from_value(value)?));
                    }
                    _ => {}
                }
            }

            // Default to Help
            Ok(RdapObject::Help(serde_json::from_value(value)?))
        } else {
            Err(RdapError::Json(serde::de::Error::custom(
                "Invalid RDAP response",
            )))
        }
    }
}

impl Default for RdapClient {
    fn default() -> Self {
        Self::new().expect("Failed to create RDAP client")
    }
}
