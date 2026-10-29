//! RDAP client implementation

use crate::bootstrap::BootstrapClient;
use crate::error::{RdapError, Result};
use crate::ip;
use crate::models::{Domain, RdapObject};
use crate::net::{HttpPool, NetOptions};
use crate::request::{QueryType, RdapRequest};
use serde::Serialize;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

/// RDAP query result with optional registrar data
#[derive(Debug, Clone, Serialize)]
pub struct RdapQueryResult {
    /// Primary result from registry
    pub registry: RdapObject,
    /// URL of the registry RDAP server used
    pub registry_url: Url,
    /// Optional result from registrar (for domain queries)
    pub registrar: Option<RdapObject>,
    /// URL of the registrar RDAP server used (if any)
    pub registrar_url: Option<Url>,
    /// Registry response exactly as received, including fields the models don't cover
    #[serde(skip)]
    pub registry_json: serde_json::Value,
    /// Registrar response exactly as received
    #[serde(skip)]
    pub registrar_json: Option<serde_json::Value>,
    /// A registrar referral was found but fetching it failed, so this
    /// registry-only result may be incomplete (and shouldn't be cached long)
    #[serde(skip)]
    pub registrar_failed: bool,
}

/// RDAP client
pub struct RdapClient {
    http: Arc<HttpPool>,
    bootstrap: BootstrapClient,
    /// Per-request timeout override for RDAP queries (`None` = pool default)
    timeout: Option<Duration>,
    follow_referral: bool,
}

impl RdapClient {
    /// Create a new RDAP client with default network options
    pub fn new() -> Result<Self> {
        Self::with_options(&NetOptions::default())
    }

    /// Create a new RDAP client with custom network options (timeout,
    /// proxies, source address, ...)
    pub fn with_options(opts: &NetOptions) -> Result<Self> {
        let http = Arc::new(HttpPool::new(opts)?);
        let bootstrap = BootstrapClient::with_pool(Arc::clone(&http));

        Ok(Self {
            http,
            bootstrap,
            timeout: None,
            follow_referral: true, // Enable by default
        })
    }

    /// Set timeout for RDAP queries
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Redacted labels of the outbound routes (proxy / source address) in rotation
    pub fn route_labels(&self) -> Vec<String> {
        self.http.route_labels()
    }

    /// Enable or disable following registrar referrals (default: enabled)
    pub const fn with_follow_referral(mut self, follow: bool) -> Self {
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
        // Try the original query first
        let err = match self.query_servers(request).await {
            Ok(result) => return Ok(result),
            Err(e) => e,
        };

        if self.should_retry_with_cidr(request, &err) {
            // For IPv6 host queries that get 400, retry with CIDR prefixes
            // Some RDAP servers (e.g., TWNIC) don't support host-level IPv6 queries
            for prefix_len in &[64u8, 48, 32] {
                if let Some(cidr_query) = self.make_cidr_query(request, *prefix_len) {
                    log::info!(
                        "Retrying with CIDR prefix /{prefix_len}: {}",
                        cidr_query.query
                    );
                    if let Ok(result) = self.query_servers(&cidr_query).await {
                        return Ok(result);
                    }
                }
            }
        }

        // All attempts failed, return the original error
        Err(err)
    }

    /// Check if we should retry an IPv6 query with CIDR notation
    fn should_retry_with_cidr(&self, request: &RdapRequest, error: &RdapError) -> bool {
        if request.query_type != QueryType::Ip {
            return false;
        }
        // Only retry for non-CIDR IPv6 queries that got a 400
        if ip::is_cidr(&request.query) {
            return false;
        }
        if !request.query.contains(':') {
            return false;
        }
        matches!(error, RdapError::ServerError { code: 400, .. })
    }

    /// Create a CIDR query from a host IPv6 address
    fn make_cidr_query(&self, request: &RdapRequest, prefix_len: u8) -> Option<RdapRequest> {
        let addr: IpAddr = request.query.parse().ok()?;
        if let IpAddr::V6(v6) = addr {
            let bits = u128::from(v6);
            let mask = !((1u128 << (128 - prefix_len)) - 1);
            let network = std::net::Ipv6Addr::from(bits & mask);
            let cidr = format!("{network}/{prefix_len}");
            let mut new_request = RdapRequest::new(QueryType::Ip, cidr);
            new_request.server = request.server.clone();
            Some(new_request)
        } else {
            None
        }
    }

    /// Try querying all available servers for a request
    async fn query_servers(&self, request: &RdapRequest) -> Result<RdapQueryResult> {
        // Determine RDAP servers
        let urls = if let Some(server) = &request.server {
            vec![server.clone()]
        } else {
            self.bootstrap.lookup(request).await?
        };

        if urls.is_empty() {
            return Err(RdapError::NoServer(request.query.clone()));
        }

        // Try each server
        let mut last_error = None;

        for base_url in &urls {
            let url = request.build_url(base_url)?;

            log::debug!("Querying RDAP server: {url}");

            match self.fetch_rdap_json(&url).await {
                Ok((obj, json)) => {
                    // For domain queries (not TLD), try to follow registrar referral
                    if self.follow_referral
                        && request.query_type == QueryType::Domain
                        && let RdapObject::Domain(ref domain) = obj
                        && let Some(registrar_rdap_url) = self.extract_registrar_rdap_url(domain)
                    {
                        // Skip if referral points to the same server (same host)
                        if Self::is_same_server(&url, &registrar_rdap_url) {
                            log::debug!(
                                "Skipping referral: same server as registry ({})",
                                registrar_rdap_url.host_str().unwrap_or("unknown")
                            );
                            return Ok(RdapQueryResult {
                                registry: obj,
                                registry_url: url,
                                registrar: None,
                                registrar_url: None,
                                registry_json: json,
                                registrar_json: None,
                                registrar_failed: false,
                            });
                        }

                        log::debug!("Following registrar referral: {registrar_rdap_url}");
                        match self.fetch_rdap_json(&registrar_rdap_url).await {
                            Ok((registrar_obj, registrar_json)) => {
                                return Ok(RdapQueryResult {
                                    registry: obj,
                                    registry_url: url,
                                    registrar: Some(registrar_obj),
                                    registrar_url: Some(registrar_rdap_url),
                                    registry_json: json,
                                    registrar_json: Some(registrar_json),
                                    registrar_failed: false,
                                });
                            }
                            Err(e) => {
                                log::warn!("Failed to fetch registrar data: {e}");
                                // Continue with registry-only result
                                return Ok(RdapQueryResult {
                                    registry: obj,
                                    registry_url: url,
                                    registrar: None,
                                    registrar_url: None,
                                    registry_json: json,
                                    registrar_json: None,
                                    registrar_failed: true,
                                });
                            }
                        }
                    }
                    return Ok(RdapQueryResult {
                        registry: obj,
                        registry_url: url,
                        registrar: None,
                        registrar_url: None,
                        registry_json: json,
                        registrar_json: None,
                        registrar_failed: false,
                    });
                }
                Err(RdapError::NotFound) => return Err(RdapError::NotFound),
                Err(e) => {
                    log::warn!("Server {url} failed: {e}");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(RdapError::NoWorkingServers))
    }

    /// Check if two URLs point to the same server (same host and port)
    fn is_same_server(url1: &Url, url2: &Url) -> bool {
        url1.host() == url2.host() && url1.port_or_known_default() == url2.port_or_known_default()
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
        self.fetch_rdap_json(url).await.map(|(obj, _)| obj)
    }

    /// Fetch RDAP response from URL, returning the parsed object and the raw JSON
    async fn fetch_rdap_json(&self, url: &Url) -> Result<(RdapObject, serde_json::Value)> {
        let response = self
            .http
            .get(url, "application/rdap+json, application/json", self.timeout)
            .await?;
        let status = response.status;

        if (200..300).contains(&status) {
            self.parse_response(&response.body)
        } else if status == 404 {
            Err(RdapError::NotFound)
        } else if let Ok(err_obj) =
            serde_json::from_slice::<crate::models::ErrorResponse>(&response.body)
        {
            // Server sent an RDAP error object
            Err(RdapError::ServerError {
                code: err_obj.error_code.unwrap_or(status),
                title: err_obj.title.unwrap_or_else(|| "Unknown error".to_string()),
                description: err_obj.description,
            })
        } else {
            Err(RdapError::ServerError {
                code: status,
                title: format!("HTTP error {status}"),
                description: Vec::new(),
            })
        }
    }

    /// Parse RDAP JSON response into the typed object, keeping the raw JSON
    fn parse_response(&self, json: &[u8]) -> Result<(RdapObject, serde_json::Value)> {
        // First, parse as generic JSON to inspect structure
        let value: serde_json::Value = serde_json::from_slice(json)?;
        let obj = Self::classify(value.clone())?;
        Ok((obj, value))
    }

    /// Turn generic RDAP JSON into the matching `RdapObject` variant
    fn classify(value: serde_json::Value) -> Result<RdapObject> {
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
