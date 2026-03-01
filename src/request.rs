//! RDAP request types and builders

use crate::error::Result;
use crate::ip;
use std::fmt;
use url::Url;

/// RDAP query types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// Domain name query
    Domain,
    /// TLD (top-level domain) query - queries IANA for TLD info
    Tld,
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
            Self::Domain => "domain",
            Self::Tld => "tld",
            Self::Ip => "ip",
            Self::Autnum => "autnum",
            Self::Entity => "entity",
            Self::Nameserver => "nameserver",
            Self::Help => "help",
            Self::DomainSearch => "domain-search",
            Self::DomainSearchByNameserver => "domain-search-by-nameserver",
            Self::DomainSearchByNameserverIp => "domain-search-by-nameserver-ip",
            Self::NameserverSearch => "nameserver-search",
            Self::NameserverSearchByIp => "nameserver-search-by-ip",
            Self::EntitySearch => "entity-search",
            Self::EntitySearchByHandle => "entity-search-by-handle",
        };
        write!(f, "{s}")
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
        let encoded_query = urlencoding::encode(&self.query);
        let path = match self.query_type {
            QueryType::Domain | QueryType::Tld => {
                format!("domain/{encoded_query}")
            }
            QueryType::Ip => format!("ip/{}", self.query),
            QueryType::Autnum => {
                // Case-insensitive strip of "AS" prefix
                let asn = if self.query.to_uppercase().starts_with("AS") {
                    &self.query[2..]
                } else {
                    &self.query
                };
                format!("autnum/{asn}")
            }
            QueryType::Entity => format!("entity/{encoded_query}"),
            QueryType::Nameserver => format!("nameserver/{encoded_query}"),
            QueryType::Help => "help".to_owned(),
            QueryType::DomainSearch => {
                return Ok(base_url.join(&format!("domains?name={encoded_query}"))?);
            }
            QueryType::DomainSearchByNameserver => {
                return Ok(base_url.join(&format!("domains?nsLdhName={encoded_query}"))?);
            }
            QueryType::DomainSearchByNameserverIp => {
                return Ok(base_url.join(&format!("domains?nsIp={}", self.query))?);
            }
            QueryType::NameserverSearch => {
                return Ok(base_url.join(&format!("nameservers?name={encoded_query}"))?);
            }
            QueryType::NameserverSearchByIp => {
                return Ok(base_url.join(&format!("nameservers?ip={}", self.query))?);
            }
            QueryType::EntitySearch => {
                return Ok(base_url.join(&format!("entities?fn={encoded_query}"))?);
            }
            QueryType::EntitySearchByHandle => {
                return Ok(base_url.join(&format!("entities?handle={encoded_query}"))?);
            }
        };

        Ok(base_url.join(&path)?)
    }

    /// Detect query type from string
    pub fn detect_type(query: &str) -> Result<QueryType> {
        Self::detect_type_with_tld_check(query, |_| false)
    }

    /// Detect query type from string with TLD check
    pub fn detect_type_with_tld_check<F>(query: &str, is_tld: F) -> Result<QueryType>
    where
        F: Fn(&str) -> bool,
    {
        // Check for AS number
        if query.to_uppercase().starts_with("AS") && query[2..].chars().all(|c| c.is_ascii_digit())
        {
            return Ok(QueryType::Autnum);
        }

        // Check for pure number (AS number without AS prefix)
        // But not if it looks like an IP (e.g., large numbers that could be IPs)
        if query.chars().all(|c| c.is_ascii_digit()) {
            // Numbers > 4_294_967_295 can't be AS numbers or IPs
            if let Ok(n) = query.parse::<u64>()
                && n <= 4_294_967_295
                && n <= 4_294_967_294
            {
                // Could be AS number or IP in numeric form
                // Treat as AS number if it's a reasonable AS number range
                return Ok(QueryType::Autnum);
            }
            return Ok(QueryType::Autnum);
        }

        // Check for IP address or CIDR (using ip module)
        if ip::is_ip_like(query) {
            return Ok(QueryType::Ip);
        }

        // Check if it's a single word that's a valid TLD (no dots)
        if !query.contains('.') && is_tld(query) {
            return Ok(QueryType::Tld);
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
        assert_eq!(
            RdapRequest::detect_type("example.com").unwrap(),
            QueryType::Domain
        );
        assert_eq!(
            RdapRequest::detect_type("192.0.2.1").unwrap(),
            QueryType::Ip
        );
        assert_eq!(
            RdapRequest::detect_type("2001:db8::1").unwrap(),
            QueryType::Ip
        );
        assert_eq!(
            RdapRequest::detect_type("AS15169").unwrap(),
            QueryType::Autnum
        );
        assert_eq!(
            RdapRequest::detect_type("15169").unwrap(),
            QueryType::Autnum
        );
    }

    #[test]
    fn test_build_url_ipv6() {
        let base = Url::parse("https://rdap.apnic.net/").unwrap();
        let req = RdapRequest::new(QueryType::Ip, "2001:db8::1");
        let url = req.build_url(&base).unwrap();
        assert_eq!(url.as_str(), "https://rdap.apnic.net/ip/2001:db8::1");
    }

    #[test]
    fn test_build_url_ipv4() {
        let base = Url::parse("https://rdap.arin.net/registry/").unwrap();
        let req = RdapRequest::new(QueryType::Ip, "8.8.8.8");
        let url = req.build_url(&base).unwrap();
        assert_eq!(url.as_str(), "https://rdap.arin.net/registry/ip/8.8.8.8");
    }
}
