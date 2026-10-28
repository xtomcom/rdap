//! RDAP request types and builders

use crate::error::{RdapError, Result};
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

/// Strip a case-insensitive `AS` prefix from an AS number (`AS13335` -> `13335`)
pub fn strip_as_prefix(query: &str) -> &str {
    match query.get(..2) {
        Some(prefix) if prefix.eq_ignore_ascii_case("AS") => &query[2..],
        _ => query,
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

    /// Build a request from free-form user input (CLI argument or API path).
    ///
    /// Detects the query type unless `query_type` is given, expands shorthand
    /// IPs (`1.1` -> `1.0.0.1`) and rejects input that can't be a valid query.
    pub fn from_user_query<F>(query: &str, query_type: Option<QueryType>, is_tld: F) -> Result<Self>
    where
        F: Fn(&str) -> bool,
    {
        let query = query.trim();
        if query.is_empty() {
            return Err(RdapError::InvalidQuery("Query is empty".to_owned()));
        }
        if query.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(RdapError::InvalidQuery(format!(
                "Query contains whitespace or control characters: {query:?}"
            )));
        }

        let query_type = match query_type {
            Some(qt) => qt,
            None => Self::detect_type_with_tld_check(query, is_tld)?,
        };

        let query = match query_type {
            QueryType::Ip
            | QueryType::DomainSearchByNameserverIp
            | QueryType::NameserverSearchByIp => ip::normalize_ip(query).ok_or_else(|| {
                RdapError::InvalidQuery(format!("Invalid IP address or CIDR: {query}"))
            })?,
            QueryType::Autnum => {
                let asn = strip_as_prefix(query);
                if asn.parse::<u32>().is_err() {
                    return Err(RdapError::InvalidQuery(format!(
                        "Invalid AS number: {query}"
                    )));
                }
                format!("AS{asn}")
            }
            _ => query.to_owned(),
        };

        Ok(Self::new(query_type, query))
    }

    /// Build the full RDAP URL
    pub fn build_url(&self, base_url: &Url) -> Result<Url> {
        let encoded_query = urlencoding::encode(&self.query);
        let path = match self.query_type {
            QueryType::Domain | QueryType::Tld => {
                format!("domain/{encoded_query}")
            }
            QueryType::Ip => format!("ip/{}", Self::checked_ip(&self.query)?),
            QueryType::Autnum => format!("autnum/{}", strip_as_prefix(&self.query)),
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
                let ip = Self::checked_ip(&self.query)?;
                return Ok(base_url.join(&format!("domains?nsIp={ip}"))?);
            }
            QueryType::NameserverSearch => {
                return Ok(base_url.join(&format!("nameservers?name={encoded_query}"))?);
            }
            QueryType::NameserverSearchByIp => {
                let ip = Self::checked_ip(&self.query)?;
                return Ok(base_url.join(&format!("nameservers?ip={ip}"))?);
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

    /// IPs and CIDRs go into the URL unencoded (the `/` must stay), so only
    /// allow characters that can appear in one
    fn checked_ip(query: &str) -> Result<&str> {
        if !query.is_empty()
            && query
                .chars()
                .all(|c| c.is_ascii_hexdigit() || matches!(c, '.' | ':' | '/'))
        {
            Ok(query)
        } else {
            Err(RdapError::InvalidQuery(format!(
                "Invalid IP address or CIDR: {query}"
            )))
        }
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
        let asn = strip_as_prefix(query);
        if asn.len() < query.len() && !asn.is_empty() && asn.chars().all(|c| c.is_ascii_digit()) {
            return Ok(QueryType::Autnum);
        }

        // Pure number -> AS number without the AS prefix
        if !query.is_empty() && query.chars().all(|c| c.is_ascii_digit()) {
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
    fn test_detect_type_non_ascii_does_not_panic() {
        // "aß" uppercases to "ASS" but byte 2 is inside 'ß'
        assert_eq!(RdapRequest::detect_type("aß").unwrap(), QueryType::Domain);
        assert_eq!(RdapRequest::detect_type("AS").unwrap(), QueryType::Domain);
    }

    #[test]
    fn test_from_user_query() {
        let no_tld = |_: &str| false;
        let req = RdapRequest::from_user_query(" 1.1 ", None, no_tld).unwrap();
        assert_eq!(
            (req.query_type, req.query.as_str()),
            (QueryType::Ip, "1.0.0.1")
        );

        let req = RdapRequest::from_user_query("as13335", None, no_tld).unwrap();
        assert_eq!(
            (req.query_type, req.query.as_str()),
            (QueryType::Autnum, "AS13335")
        );

        let req = RdapRequest::from_user_query("google", None, |q| q == "google").unwrap();
        assert_eq!(req.query_type, QueryType::Tld);

        for bad in ["", "   ", "99999999999", "a b.com", "1.1.1.1/abc"] {
            assert!(
                RdapRequest::from_user_query(bad, None, no_tld).is_err(),
                "{bad:?}"
            );
        }
        assert!(RdapRequest::from_user_query("../x", Some(QueryType::Ip), no_tld).is_err());
    }

    #[test]
    fn test_build_url_rejects_path_injection() {
        let base = Url::parse("https://rdap.example/").unwrap();
        let req = RdapRequest::new(QueryType::Ip, "../../admin?x=1");
        assert!(req.build_url(&base).is_err());
        let req = RdapRequest::new(QueryType::Domain, "../admin");
        assert_eq!(
            req.build_url(&base).unwrap().as_str(),
            "https://rdap.example/domain/..%2Fadmin"
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
