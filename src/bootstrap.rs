//! Bootstrap service discovery

use crate::config::{self, Config, TldOverrides};
use crate::error::{RdapError, Result};
use crate::ip;
use crate::net::{HttpPool, NetOptions};
use crate::request::{QueryType, RdapRequest, parse_asn};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use url::Url;

/// How long a fetched bootstrap registry is reused before refetching
const REGISTRY_TTL: Duration = Duration::from_hours(24);

/// Bootstrap registry file
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BootstrapRegistry {
    version: String,
    publication: Option<String>,
    description: Option<String>,
    services: Vec<Vec<serde_json::Value>>,
}

/// Bootstrap client for service discovery
pub struct BootstrapClient {
    http: Arc<HttpPool>,
    config: Config,
    tld_overrides: TldOverrides,
    /// In-memory registry cache by URL
    registries: Mutex<HashMap<String, CachedRegistry>>,
    /// Serializes registry fetches so concurrent cold misses fetch once
    fetch_lock: tokio::sync::Mutex<()>,
}

/// After a failed fetch, don't retry that URL for this long
const FAILURE_BACKOFF: Duration = Duration::from_secs(30);

/// Record a failed fetch of `url`; returns the stale copy, if any
fn mark_failed(
    registries: &mut HashMap<String, CachedRegistry>,
    url: &str,
) -> Option<Arc<BootstrapRegistry>> {
    let entry = registries.entry(url.to_owned()).or_default();
    entry.failed_at = Some(Instant::now());
    entry.registry.as_ref().map(|(_, stale)| Arc::clone(stale))
}

#[derive(Default)]
struct CachedRegistry {
    /// Last good copy and when it was fetched (kept past the TTL as a fallback)
    registry: Option<(Instant, Arc<BootstrapRegistry>)>,
    /// When the last fetch failed
    failed_at: Option<Instant>,
}

impl BootstrapClient {
    /// Create a new bootstrap client
    pub fn new() -> Result<Self> {
        Ok(Self::with_pool(Arc::new(HttpPool::new(
            &NetOptions::default(),
        )?)))
    }

    /// Create a bootstrap client that sends requests through `http`
    pub fn with_pool(http: Arc<HttpPool>) -> Self {
        let config = Config::load().unwrap_or_default();
        let tld_overrides = config::load_tld_overrides().unwrap_or_default();

        Self {
            http,
            config,
            tld_overrides,
            registries: Mutex::new(HashMap::new()),
            fetch_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Lookup RDAP servers for a request
    pub async fn lookup(&self, request: &RdapRequest) -> Result<Vec<Url>> {
        match request.query_type {
            QueryType::Tld => {
                // TLD queries always go to IANA RDAP
                let url = Url::parse(config::IANA_RDAP_URL)
                    .map_err(|e| RdapError::Bootstrap(format!("Invalid IANA RDAP URL: {e}")))?;
                Ok(vec![url])
            }
            // A nameserver object lives at the registry of its own name's TLD
            // (ns1.google.com -> .com), like a domain
            QueryType::Domain | QueryType::Nameserver => {
                // Priority: tlds.json first, then bootstrap
                if let Some(url) = config::lookup_tld_override(&self.tld_overrides, &request.query)
                {
                    log::debug!("Found TLD override for {}: {url}", request.query);
                    return Ok(vec![url]);
                }

                // Fall back to IANA bootstrap
                let registry = self.fetch_registry(&self.config.bootstrap.dns).await?;
                Ok(self.match_domain(&registry, &request.query))
            }
            QueryType::Ip => {
                let bootstrap_url = if request.query.contains(':') {
                    &self.config.bootstrap.ipv6
                } else {
                    &self.config.bootstrap.ipv4
                };
                let registry = self.fetch_registry(bootstrap_url).await?;
                self.match_ip(&registry, &request.query)
            }
            QueryType::Autnum => {
                let registry = self.fetch_registry(&self.config.bootstrap.asn).await?;
                self.match_asn(&registry, &request.query)
            }
            other => Err(RdapError::NeedsServer(other.to_string())),
        }
    }

    /// Fetch bootstrap registry file from URL, reusing a cached copy for
    /// `REGISTRY_TTL`. Concurrent misses share one fetch; after a failure the
    /// URL isn't refetched for `FAILURE_BACKOFF`, and a stale copy is used
    /// instead of failing when there is one.
    async fn fetch_registry(&self, url: &str) -> Result<Arc<BootstrapRegistry>> {
        if let Some(answer) = self.cached_registry(url) {
            return answer;
        }
        let _guard = self.fetch_lock.lock().await;
        // Another task may have fetched (or failed) while we waited
        if let Some(answer) = self.cached_registry(url) {
            return answer;
        }

        match self.download_registry(url).await {
            Ok(registry) => {
                self.registries.lock().unwrap().insert(
                    url.to_owned(),
                    CachedRegistry {
                        registry: Some((Instant::now(), Arc::clone(&registry))),
                        failed_at: None,
                    },
                );
                Ok(registry)
            }
            Err(e) => {
                let stale = mark_failed(&mut self.registries.lock().unwrap(), url);
                if let Some(stale) = stale {
                    log::warn!(
                        "Failed to refresh bootstrap registry {url}, using cached copy: {e}"
                    );
                    return Ok(stale);
                }
                Err(e)
            }
        }
    }

    /// A cached answer for `url`: fresh copy, or (within the failure backoff)
    /// the stale copy / an error. `None` means "fetch it".
    fn cached_registry(&self, url: &str) -> Option<Result<Arc<BootstrapRegistry>>> {
        let (registry, failed_at) = self
            .registries
            .lock()
            .unwrap()
            .get(url)
            .map(|entry| (entry.registry.clone(), entry.failed_at))?;
        if let Some((fetched_at, registry)) = &registry
            && fetched_at.elapsed() < REGISTRY_TTL
        {
            return Some(Ok(Arc::clone(registry)));
        }
        if failed_at.is_some_and(|t| t.elapsed() < FAILURE_BACKOFF) {
            return Some(registry.as_ref().map_or_else(
                || {
                    Err(RdapError::Bootstrap(format!(
                        "Bootstrap registry {url} is unavailable (fetch failed recently)"
                    )))
                },
                |(_, stale)| Ok(Arc::clone(stale)),
            ));
        }
        None
    }

    async fn download_registry(&self, url: &str) -> Result<Arc<BootstrapRegistry>> {
        log::debug!("Fetching bootstrap registry: {url}");

        let parsed = Url::parse(url)
            .map_err(|e| RdapError::Bootstrap(format!("Invalid bootstrap URL {url}: {e}")))?;
        let response = self.http.get(&parsed, "application/json", None).await?;

        if !(200..300).contains(&response.status) {
            return Err(RdapError::Bootstrap(format!(
                "Failed to fetch registry: HTTP {}",
                response.status
            )));
        }

        Ok(Arc::new(serde_json::from_slice(&response.body)?))
    }

    /// Match domain name
    fn match_domain(&self, registry: &BootstrapRegistry, domain: &str) -> Vec<Url> {
        let domain = domain.trim_end_matches('.').to_lowercase();

        // Build lookup map
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for service in &registry.services {
            if service.len() >= 2
                && let (Some(entries), Some(urls)) = (service[0].as_array(), service[1].as_array())
            {
                let url_strings: Vec<String> = urls
                    .iter()
                    .filter_map(|v| v.as_str().map(std::borrow::ToOwned::to_owned))
                    .collect();

                for entry in entries {
                    if let Some(tld) = entry.as_str() {
                        map.insert(tld.to_lowercase(), url_strings.clone());
                    }
                }
            }
        }

        // Try to match from most specific to least specific
        let mut parts: Vec<&str> = domain.split('.').collect();

        while !parts.is_empty() {
            let test_domain = parts.join(".");
            if let Some(urls) = map.get(&test_domain) {
                return urls.iter().filter_map(|s| Url::parse(s).ok()).collect();
            }
            parts.remove(0);
        }

        vec![]
    }

    /// Match IP address (supports standard IPs, shorthand IPs, and CIDR)
    fn match_ip(&self, registry: &BootstrapRegistry, ip_query: &str) -> Result<Vec<Url>> {
        // Normalize the IP (handles shorthand like 1.1 -> 1.0.0.1)
        let normalized = ip::normalize_ip(ip_query)
            .ok_or_else(|| RdapError::InvalidQuery(format!("Invalid IP address: {ip_query}")))?;

        // Extract the IP part (for CIDR queries like 8.8.8.0/24, we match using the network address)
        let ip_str = ip::extract_ip_from_cidr(&normalized);

        let addr: IpAddr = ip_str
            .parse()
            .map_err(|_| RdapError::InvalidQuery(format!("Invalid IP address: {ip_str}")))?;

        for service in &registry.services {
            if service.len() >= 2
                && let (Some(entries), Some(urls)) = (service[0].as_array(), service[1].as_array())
            {
                for entry in entries {
                    if let Some(cidr) = entry.as_str()
                        && Self::ip_in_network(&addr, cidr)
                    {
                        let url_list: Vec<Url> = urls
                            .iter()
                            .filter_map(|v| v.as_str().and_then(|s| Url::parse(s).ok()))
                            .collect();
                        return Ok(url_list);
                    }
                }
            }
        }

        Ok(vec![])
    }

    /// Check if IP is in CIDR network using ipnet
    fn ip_in_network(addr: &IpAddr, cidr: &str) -> bool {
        if let Ok(network) = cidr.parse::<IpNet>() {
            return network.contains(addr);
        }
        false
    }

    /// Match AS number
    fn match_asn(&self, registry: &BootstrapRegistry, asn_str: &str) -> Result<Vec<Url>> {
        let asn = parse_asn(asn_str)
            .ok_or_else(|| RdapError::InvalidQuery(format!("Invalid AS number: {asn_str}")))?;

        for service in &registry.services {
            if service.len() >= 2
                && let (Some(entries), Some(urls)) = (service[0].as_array(), service[1].as_array())
            {
                for entry in entries {
                    if let Some(range_str) = entry.as_str()
                        && Self::asn_in_range(asn, range_str)
                    {
                        let url_list: Vec<Url> = urls
                            .iter()
                            .filter_map(|v| v.as_str().and_then(|s| Url::parse(s).ok()))
                            .collect();
                        return Ok(url_list);
                    }
                }
            }
        }

        Ok(vec![])
    }

    /// Check if AS number is in range
    fn asn_in_range(asn: u32, range_str: &str) -> bool {
        if let Some(dash_pos) = range_str.find('-') {
            // Range: "1000-2000"
            if let (Ok(start), Ok(end)) = (
                range_str[..dash_pos].parse::<u32>(),
                range_str[dash_pos + 1..].parse::<u32>(),
            ) {
                return asn >= start && asn <= end;
            }
        } else {
            // Single AS: "1000"
            if let Ok(single) = range_str.parse::<u32>() {
                return asn == single;
            }
        }
        false
    }
}
