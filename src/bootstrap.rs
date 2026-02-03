//! Bootstrap service discovery

use crate::config::{self, Config, TldOverrides};
use crate::error::{RdapError, Result};
use crate::ip;
use crate::request::{QueryType, RdapRequest};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use url::Url;

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
    http_client: reqwest::Client,
    config: Config,
    tld_overrides: TldOverrides,
}

impl BootstrapClient {
    /// Create a new bootstrap client
    pub fn new() -> Result<Self> {
        let config = Config::load().unwrap_or_default();
        let tld_overrides = config::load_tld_overrides().unwrap_or_default();

        Ok(Self {
            http_client: reqwest::Client::new(),
            config,
            tld_overrides,
        })
    }

    /// Lookup RDAP servers for a request
    pub async fn lookup(&self, request: &RdapRequest) -> Result<Vec<Url>> {
        match request.query_type {
            QueryType::Tld => {
                // TLD queries always go to IANA RDAP
                let url = Url::parse(config::IANA_RDAP_URL)
                    .map_err(|e| RdapError::Bootstrap(format!("Invalid IANA RDAP URL: {}", e)))?;
                Ok(vec![url])
            }
            QueryType::Domain => {
                // Priority: tlds.json first, then bootstrap
                if let Some(url) = config::lookup_tld_override(&self.tld_overrides, &request.query)
                {
                    log::debug!("Found TLD override for {}: {}", request.query, url);
                    return Ok(vec![url]);
                }

                // Fall back to IANA bootstrap
                let registry = self.fetch_registry(&self.config.bootstrap.dns).await?;
                self.match_domain(&registry, &request.query)
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
            QueryType::Entity => Err(RdapError::Bootstrap(
                "Entity queries require explicit server (-s/--server)".to_string(),
            )),
            _ => Err(RdapError::Bootstrap(
                "This query type requires explicit server (-s/--server)".to_string(),
            )),
        }
    }

    /// Fetch bootstrap registry file from URL
    async fn fetch_registry(&self, url: &str) -> Result<BootstrapRegistry> {
        log::debug!("Fetching bootstrap registry: {}", url);

        let response = self.http_client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(RdapError::Bootstrap(format!(
                "Failed to fetch registry: HTTP {}",
                response.status()
            )));
        }

        let registry: BootstrapRegistry = response.json().await?;
        Ok(registry)
    }

    /// Match domain name
    fn match_domain(&self, registry: &BootstrapRegistry, domain: &str) -> Result<Vec<Url>> {
        let domain = domain.trim_end_matches('.').to_lowercase();

        // Build lookup map
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for service in &registry.services {
            if service.len() >= 2
                && let (Some(entries), Some(urls)) = (service[0].as_array(), service[1].as_array())
            {
                let url_strings: Vec<String> = urls
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
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
                return Ok(urls.iter().filter_map(|s| Url::parse(s).ok()).collect());
            }
            parts.remove(0);
        }

        Ok(vec![])
    }

    /// Match IP address (supports standard IPs, shorthand IPs, and CIDR)
    fn match_ip(&self, registry: &BootstrapRegistry, ip_query: &str) -> Result<Vec<Url>> {
        // Normalize the IP (handles shorthand like 1.1 -> 1.0.0.1)
        let normalized = ip::normalize_ip(ip_query)
            .ok_or_else(|| RdapError::InvalidQuery(format!("Invalid IP address: {}", ip_query)))?;

        // Extract the IP part (for CIDR queries like 8.8.8.0/24, we match using the network address)
        let ip_str = ip::extract_ip_from_cidr(&normalized);

        let addr: IpAddr = ip_str
            .parse()
            .map_err(|_| RdapError::InvalidQuery(format!("Invalid IP address: {}", ip_str)))?;

        for service in &registry.services {
            if service.len() >= 2
                && let (Some(entries), Some(urls)) = (service[0].as_array(), service[1].as_array())
            {
                for entry in entries {
                    if let Some(cidr) = entry.as_str()
                        && self.ip_in_network(&addr, cidr)
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
    fn ip_in_network(&self, addr: &IpAddr, cidr: &str) -> bool {
        if let Ok(network) = cidr.parse::<IpNet>() {
            return network.contains(addr);
        }
        false
    }

    /// Match AS number
    fn match_asn(&self, registry: &BootstrapRegistry, asn_str: &str) -> Result<Vec<Url>> {
        // Case-insensitive strip of "AS" prefix
        let asn_str = if asn_str.to_uppercase().starts_with("AS") {
            &asn_str[2..]
        } else {
            asn_str
        };
        let asn: u32 = asn_str
            .parse()
            .map_err(|_| RdapError::InvalidQuery(format!("Invalid AS number: {}", asn_str)))?;

        for service in &registry.services {
            if service.len() >= 2
                && let (Some(entries), Some(urls)) = (service[0].as_array(), service[1].as_array())
            {
                for entry in entries {
                    if let Some(range_str) = entry.as_str()
                        && self.asn_in_range(asn, range_str)
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
    fn asn_in_range(&self, asn: u32, range_str: &str) -> bool {
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
