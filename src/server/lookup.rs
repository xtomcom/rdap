//! Turning user input into RDAP queries and validated, cached results

use super::AppState;
use super::cache::Cached;
use super::error::ApiError;
use crate::client::RdapQueryResult;
use crate::models::RdapObject;
use crate::request::{QueryType, RdapRequest};
use std::sync::Arc;

/// Longest query string accepted (a DNS name is at most 253 characters)
pub const MAX_QUERY_LEN: usize = 512;

/// Every query type, for parsing `?type=`
const QUERY_TYPES: [QueryType; 14] = [
    QueryType::Domain,
    QueryType::Tld,
    QueryType::Ip,
    QueryType::Autnum,
    QueryType::Entity,
    QueryType::Nameserver,
    QueryType::Help,
    QueryType::DomainSearch,
    QueryType::DomainSearchByNameserver,
    QueryType::DomainSearchByNameserverIp,
    QueryType::NameserverSearch,
    QueryType::NameserverSearchByIp,
    QueryType::EntitySearch,
    QueryType::EntitySearchByHandle,
];

/// Parse a `?type=` value (same names as the CLI's `-t`)
pub fn parse_query_type(value: &str) -> Result<QueryType, ApiError> {
    QUERY_TYPES
        .into_iter()
        .find(|qt| qt.to_string().eq_ignore_ascii_case(value.trim()))
        .ok_or_else(|| {
            ApiError::bad_request(format!(
                "Invalid type {value:?}; expected one of: {}",
                QUERY_TYPES.map(|qt| qt.to_string()).join(", ")
            ))
        })
}

/// Which object(s) of a query result to return
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Registrar data when a referral succeeded, else registry data
    Registrar,
    Registry,
    Both,
}

impl Source {
    /// Parse a `source` value; `None` means the default (registrar)
    pub fn parse(value: Option<&str>) -> Result<Self, ApiError> {
        match value.map(str::trim) {
            None => Ok(Self::Registrar),
            Some(v) if v.eq_ignore_ascii_case("registrar") => Ok(Self::Registrar),
            Some(v) if v.eq_ignore_ascii_case("registry") => Ok(Self::Registry),
            Some(v) if v.eq_ignore_ascii_case("both") => Ok(Self::Both),
            Some(v) => Err(ApiError::bad_request(format!(
                "Invalid source {v:?}; expected registrar, registry or both"
            ))),
        }
    }
}

/// Query types bootstrap can't route: they need `-s/--server`
const fn needs_upstream(query_type: QueryType) -> bool {
    !matches!(
        query_type,
        QueryType::Domain
            | QueryType::Tld
            | QueryType::Ip
            | QueryType::Autnum
            | QueryType::Nameserver
    )
}

impl AppState {
    /// Whether `query` is a known TLD (`google` -> query the .google TLD)
    pub fn is_tld(&self, query: &str) -> bool {
        self.tlds.as_ref().is_some_and(|list| list.is_tld(query))
    }

    /// Validate user input and build the request (detecting the type unless given)
    pub fn build_request(
        &self,
        raw: &str,
        query_type: Option<QueryType>,
    ) -> Result<RdapRequest, ApiError> {
        if raw.len() > MAX_QUERY_LEN {
            return Err(ApiError::bad_request(format!(
                "Query is longer than {MAX_QUERY_LEN} bytes"
            )));
        }
        let request = RdapRequest::from_user_query(raw, query_type, |q| self.is_tld(q))
            .map_err(|e| ApiError::from_rdap(&e, raw))?;

        match &self.opts.upstream {
            Some(upstream) => Ok(request.with_server(upstream.clone())),
            None if needs_upstream(request.query_type) => Err(ApiError::needs_upstream()),
            None => Ok(request),
        }
    }

    /// Run a request through the cache
    pub async fn lookup(
        self: &Arc<Self>,
        request: RdapRequest,
    ) -> Result<Arc<RdapQueryResult>, ApiError> {
        let key = (request.query_type.to_string(), request.query.clone());
        let state = Arc::clone(self);
        let fetch = async move { state.fetch(&request).await };
        match self.cache.get_or_fetch(key, fetch).await? {
            Cached::Found(result) => Ok(result),
            Cached::NotFound(err) => Err(err),
        }
    }

    /// Query upstream. `Ok(NotFound)` is a definitive, cacheable "not found";
    /// `Err` is never cached.
    async fn fetch(&self, request: &RdapRequest) -> Result<Cached, ApiError> {
        let outcome = match self.client.query_with_referral(request).await {
            Ok(result) => check_result(request, result),
            Err(e) => Err(ApiError::from_rdap(&e, &request.query)),
        };
        match outcome {
            Ok(result) => Ok(Cached::Found(Arc::new(result))),
            Err(err) if err.status() == axum::http::StatusCode::NOT_FOUND => {
                Ok(Cached::NotFound(err))
            }
            Err(err) => Err(err),
        }
    }
}

/// Reject results that aren't an answer to the query: RDAP error objects
/// sent with a 2xx status, empty searches, objects of the wrong class.
/// Unusable registrar data is dropped (registry-only result).
fn check_result(
    request: &RdapRequest,
    mut result: RdapQueryResult,
) -> Result<RdapQueryResult, ApiError> {
    check_object(request.query_type, &result.registry, &request.query)?;

    if let Some(registrar) = &result.registrar
        && let Err(err) = check_object(QueryType::Domain, registrar, &request.query)
    {
        log::warn!(
            "Ignoring registrar data for {}: {}",
            request.query,
            err.body()
        );
        result.registrar = None;
        result.registrar_url = None;
        result.registrar_json = None;
    }
    Ok(result)
}

fn check_object(query_type: QueryType, object: &RdapObject, query: &str) -> Result<(), ApiError> {
    let no_results = || ApiError::not_found(format!("No RDAP search results for {query}"));
    let matches = match object {
        RdapObject::Error(err) => return Err(ApiError::from_error_object(err, query)),
        RdapObject::DomainSearch(s) if s.domains.is_empty() => return Err(no_results()),
        RdapObject::EntitySearch(s) if s.entities.is_empty() => return Err(no_results()),
        RdapObject::NameserverSearch(s) if s.nameservers.is_empty() => return Err(no_results()),
        RdapObject::Domain(_) => matches!(query_type, QueryType::Domain | QueryType::Tld),
        RdapObject::IpNetwork(_) => query_type == QueryType::Ip,
        RdapObject::Autnum(_) => query_type == QueryType::Autnum,
        RdapObject::Entity(_) => query_type == QueryType::Entity,
        RdapObject::Nameserver(_) => query_type == QueryType::Nameserver,
        RdapObject::Help(_) => query_type == QueryType::Help,
        RdapObject::DomainSearch(_) => matches!(
            query_type,
            QueryType::DomainSearch
                | QueryType::DomainSearchByNameserver
                | QueryType::DomainSearchByNameserverIp
        ),
        RdapObject::NameserverSearch(_) => matches!(
            query_type,
            QueryType::NameserverSearch | QueryType::NameserverSearchByIp
        ),
        RdapObject::EntitySearch(_) => matches!(
            query_type,
            QueryType::EntitySearch | QueryType::EntitySearchByHandle
        ),
    };
    if matches {
        Ok(())
    } else {
        Err(ApiError::bad_gateway(format!(
            "Upstream RDAP server returned an unexpected response for {query_type} query {query}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_types_and_sources() {
        assert_eq!(parse_query_type("ip").unwrap(), QueryType::Ip);
        assert_eq!(
            parse_query_type("Domain-Search").unwrap(),
            QueryType::DomainSearch
        );
        assert_eq!(parse_query_type("bogus").unwrap_err().status(), 400);
        assert_eq!(Source::parse(None).unwrap(), Source::Registrar);
        assert_eq!(Source::parse(Some("BOTH")).unwrap(), Source::Both);
        assert_eq!(Source::parse(Some("x")).unwrap_err().status(), 400);
    }
}
