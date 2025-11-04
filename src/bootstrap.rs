//! Bootstrap service discovery

use crate::error::{RdapError, Result};
use crate::request::{QueryType, RdapRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use url::Url;

const IANA_BOOTSTRAP_URL: &str = "https://data.iana.org/rdap/";

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
    base_url: Url,
}

impl BootstrapClient {
    /// Create a new bootstrap client
    pub fn new() -> Result<Self> {
        Ok(Self {
            http_client: reqwest::Client::new(),
            base_url: Url::parse(IANA_BOOTSTRAP_URL)?,
        })
    }
    
    /// Lookup RDAP servers for a request
    pub async fn lookup(&self, request: &RdapRequest) -> Result<Vec<Url>> {
        let registry_file = match request.query_type {
            QueryType::Domain => "dns.json",
            QueryType::Ip => {
                if request.query.contains(':') {
                    "ipv6.json"
                } else {
                    "ipv4.json"
                }
            }
            QueryType::Autnum => "asn.json",
            QueryType::Entity => {
                // Service provider registry (experimental)
                return Err(RdapError::Bootstrap(
                    "Entity queries require explicit server (-s/--server)".to_string()
                ));
            }
            _ => {
                return Err(RdapError::Bootstrap(
                    "This query type requires explicit server (-s/--server)".to_string()
                ));
            }
        };
        
        let registry = self.fetch_registry(registry_file).await?;
        let urls = self.match_registry(&registry, request)?;
        
        Ok(urls)
    }
    
    /// Fetch bootstrap registry file
    async fn fetch_registry(&self, filename: &str) -> Result<BootstrapRegistry> {
        let url = self.base_url.join(filename)?;
        
        log::debug!("Fetching bootstrap registry: {}", url);
        
        let response = self.http_client.get(url.as_str()).send().await?;
        
        if !response.status().is_success() {
            return Err(RdapError::Bootstrap(format!(
                "Failed to fetch registry: HTTP {}",
                response.status()
            )));
        }
        
        let registry: BootstrapRegistry = response.json().await?;
        Ok(registry)
    }
    
    /// Match query against registry
    fn match_registry(&self, registry: &BootstrapRegistry, request: &RdapRequest) -> Result<Vec<Url>> {
        match request.query_type {
            QueryType::Domain => self.match_domain(registry, &request.query),
            QueryType::Ip => self.match_ip(registry, &request.query),
            QueryType::Autnum => self.match_asn(registry, &request.query),
            _ => Err(RdapError::Bootstrap("Unsupported query type".to_string())),
        }
    }
    
    /// Match domain name
    fn match_domain(&self, registry: &BootstrapRegistry, domain: &str) -> Result<Vec<Url>> {
        let domain = domain.trim_end_matches('.').to_lowercase();
        
        // Build lookup map
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for service in &registry.services {
            if service.len() >= 2 {
                if let (Some(entries), Some(urls)) = (service[0].as_array(), service[1].as_array()) {
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
    
    /// Match IP address
    fn match_ip(&self, registry: &BootstrapRegistry, ip: &str) -> Result<Vec<Url>> {
        let addr: IpAddr = ip.parse()
            .map_err(|_| RdapError::InvalidQuery(format!("Invalid IP address: {}", ip)))?;
        
        for service in &registry.services {
            if service.len() >= 2 {
                if let (Some(entries), Some(urls)) = (service[0].as_array(), service[1].as_array()) {
                    for entry in entries {
                        if let Some(cidr) = entry.as_str() {
                            if self.ip_in_range(&addr, cidr) {
                                let url_list: Vec<Url> = urls
                                    .iter()
                                    .filter_map(|v| v.as_str().and_then(|s| Url::parse(s).ok()))
                                    .collect();
                                return Ok(url_list);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(vec![])
    }
    
    /// Check if IP is in CIDR range
    fn ip_in_range(&self, addr: &IpAddr, cidr: &str) -> bool {
        // Parse CIDR notation
        if let Some(slash_pos) = cidr.find('/') {
            let ip_part = &cidr[..slash_pos];
            let prefix_len: u8 = cidr[slash_pos + 1..].parse().unwrap_or(0);
            
            if let Ok(network_addr) = ip_part.parse::<IpAddr>() {
                return match (network_addr, addr) {
                    (IpAddr::V4(net), IpAddr::V4(addr)) => {
                        if prefix_len > 32 {
                            return false;
                        }
                        let net_int = u32::from_be_bytes(net.octets());
                        let addr_int = u32::from_be_bytes(addr.octets());
                        let mask = if prefix_len == 0 { 0 } else { !0u32 << (32 - prefix_len) };
                        (net_int & mask) == (addr_int & mask)
                    }
                    (IpAddr::V6(net), IpAddr::V6(addr)) => {
                        if prefix_len > 128 {
                            return false;
                        }
                        let net_int = u128::from_be_bytes(net.octets());
                        let addr_int = u128::from_be_bytes(addr.octets());
                        let mask = if prefix_len == 0 { 0 } else { !0u128 << (128 - prefix_len) };
                        (net_int & mask) == (addr_int & mask)
                    }
                    _ => false, // IPv4 vs IPv6 mismatch
                };
            }
        }
        false
    }
    
    /// Match AS number
    fn match_asn(&self, registry: &BootstrapRegistry, asn_str: &str) -> Result<Vec<Url>> {
        let asn_str = asn_str.trim_start_matches("AS").trim_start_matches("as");
        let asn: u32 = asn_str.parse()
            .map_err(|_| RdapError::InvalidQuery(format!("Invalid AS number: {}", asn_str)))?;
        
        for service in &registry.services {
            if service.len() >= 2 {
                if let (Some(entries), Some(urls)) = (service[0].as_array(), service[1].as_array()) {
                    for entry in entries {
                        if let Some(range_str) = entry.as_str() {
                            if self.asn_in_range(asn, range_str) {
                                let url_list: Vec<Url> = urls
                                    .iter()
                                    .filter_map(|v| v.as_str().and_then(|s| Url::parse(s).ok()))
                                    .collect();
                                return Ok(url_list);
                            }
                        }
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
