//! RDAP request types and builders

use crate::error::Result;
use std::fmt;
use url::Url;

/// RDAP query types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// Domain name query
    Domain,
    /// IP address query
    Ip,
    /// Autonomous System Number query
    Autnum,
    /// Entity query
    Entity,
    /// Nameserver query
    Nameserver,
    /// Help query
    Help,
    /// Domain search
    DomainSearch,
    /// Domain search by nameserver
    DomainSearchByNameserver,
    /// Domain search by nameserver IP
    DomainSearchByNameserverIp,
    /// Nameserver search
    NameserverSearch,
    /// Nameserver search by IP
    NameserverSearchByIp,
    /// Entity search
    EntitySearch,
    /// Entity search by handle
    EntitySearchByHandle,
}

impl fmt::Display for QueryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            QueryType::Domain => "domain",
            QueryType::Ip => "ip",
            QueryType::Autnum => "autnum",
            QueryType::Entity => "entity",
            QueryType::Nameserver => "nameserver",
            QueryType::Help => "help",
            QueryType::DomainSearch => "domain-search",
            QueryType::DomainSearchByNameserver => "domain-search-by-nameserver",
            QueryType::DomainSearchByNameserverIp => "domain-search-by-nameserver-ip",
            QueryType::NameserverSearch => "nameserver-search",
            QueryType::NameserverSearchByIp => "nameserver-search-by-ip",
            QueryType::EntitySearch => "entity-search",
            QueryType::EntitySearchByHandle => "entity-search-by-handle",
        };
        write!(f, "{}", s)
    }
}

/// RDAP request
#[derive(Debug, Clone)]
pub struct RdapRequest {
    pub query_type: QueryType,
    pub query: String,
    pub server: Option<Url>,
}

impl RdapRequest {
    /// Create a new RDAP request
    pub fn new(query_type: QueryType, query: impl Into<String>) -> Self {
        Self {
            query_type,
            query: query.into(),
            server: None,
        }
    }
    
    /// Set the RDAP server URL
    pub fn with_server(mut self, server: Url) -> Self {
        self.server = Some(server);
        self
    }
    
    /// Build the full RDAP URL
    pub fn build_url(&self, base_url: &Url) -> Result<Url> {
        let path = match self.query_type {
            QueryType::Domain => format!("domain/{}", urlencoding::encode(&self.query)),
            QueryType::Ip => format!("ip/{}", self.query),
            QueryType::Autnum => {
                let asn = self.query.trim_start_matches("AS").trim_start_matches("as");
                format!("autnum/{}", asn)
            }
            QueryType::Entity => format!("entity/{}", urlencoding::encode(&self.query)),
            QueryType::Nameserver => format!("nameserver/{}", urlencoding::encode(&self.query)),
            QueryType::Help => "help".to_string(),
            QueryType::DomainSearch => {
                return Ok(base_url.join(&format!("domains?name={}", urlencoding::encode(&self.query)))?);
            }
            QueryType::DomainSearchByNameserver => {
                return Ok(base_url.join(&format!("domains?nsLdhName={}", urlencoding::encode(&self.query)))?);
            }
            QueryType::DomainSearchByNameserverIp => {
                return Ok(base_url.join(&format!("domains?nsIp={}", self.query))?);
            }
            QueryType::NameserverSearch => {
                return Ok(base_url.join(&format!("nameservers?name={}", urlencoding::encode(&self.query)))?);
            }
            QueryType::NameserverSearchByIp => {
                return Ok(base_url.join(&format!("nameservers?ip={}", self.query))?);
            }
            QueryType::EntitySearch => {
                return Ok(base_url.join(&format!("entities?fn={}", urlencoding::encode(&self.query)))?);
            }
            QueryType::EntitySearchByHandle => {
                return Ok(base_url.join(&format!("entities?handle={}", urlencoding::encode(&self.query)))?);
            }
        };
        
        Ok(base_url.join(&path)?)
    }
    
    /// Detect query type from string
    pub fn detect_type(query: &str) -> Result<QueryType> {
        // Check for AS number
        if query.to_uppercase().starts_with("AS") {
            if query[2..].chars().all(|c| c.is_ascii_digit()) {
                return Ok(QueryType::Autnum);
            }
        }
        
        // Check for pure number (AS number without AS prefix)
        if query.chars().all(|c| c.is_ascii_digit()) {
            return Ok(QueryType::Autnum);
        }
        
        // Check for IP address (simple heuristic)
        if query.contains(':') || query.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Ok(QueryType::Ip);
        }
        
        // Default to domain
        Ok(QueryType::Domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_type() {
        assert_eq!(RdapRequest::detect_type("example.com").unwrap(), QueryType::Domain);
        assert_eq!(RdapRequest::detect_type("192.0.2.1").unwrap(), QueryType::Ip);
        assert_eq!(RdapRequest::detect_type("2001:db8::1").unwrap(), QueryType::Ip);
        assert_eq!(RdapRequest::detect_type("AS15169").unwrap(), QueryType::Autnum);
        assert_eq!(RdapRequest::detect_type("15169").unwrap(), QueryType::Autnum);
    }
}
