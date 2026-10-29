//! Request handlers

use super::AppState;
use super::error::{ApiError, JSON, RDAP_JSON};
use super::lookup::{Source, parse_query_type};
use crate::client::RdapQueryResult;
use crate::display::find_abuse_contact;
use crate::models::RdapObject;
use crate::request::{QueryType, RdapRequest};
use axum::body::Bytes;
use axum::extract::rejection::{BytesRejection, PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use url::Url;

/// Maximum `/batch` body size
pub const MAX_BATCH_BODY: usize = 64 * 1024;
/// Upstream queries run concurrently per `/batch` request
const BATCH_CONCURRENCY: usize = 8;

type Shared = State<Arc<AppState>>;
type RawPath = Result<Path<String>, PathRejection>;
type RawParams = Result<Query<Params>, QueryRejection>;

/// `?type=` / `?source=`
#[derive(Debug, Default, Deserialize)]
pub struct Params {
    #[serde(rename = "type")]
    query_type: Option<String>,
    source: Option<String>,
}

impl Params {
    fn query_type(&self) -> Result<Option<QueryType>, ApiError> {
        self.query_type.as_deref().map(parse_query_type).transpose()
    }

    fn source(&self) -> Result<Source, ApiError> {
        Source::parse(self.source.as_deref())
    }
}

fn path_value(path: RawPath) -> Result<String, ApiError> {
    path.map(|Path(value)| value)
        .map_err(|e| ApiError::bad_request(format!("Invalid path: {}", e.body_text())))
}

fn params(params: RawParams) -> Result<Params, ApiError> {
    params
        .map(|Query(p)| p)
        .map_err(|e| ApiError::bad_request(format!("Invalid query string: {}", e.body_text())))
}

/// Serialize `value` as a JSON response
fn json_response<T: Serialize>(
    status: StatusCode,
    content_type: &'static str,
    value: &T,
) -> Response {
    match serde_json::to_vec(value) {
        Ok(body) => (
            status,
            [(header::CONTENT_TYPE, HeaderValue::from_static(content_type))],
            body,
        )
            .into_response(),
        Err(e) => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize response: {e}"),
        )
        .into_response(),
    }
}

fn into_response(result: Result<Response, ApiError>) -> Response {
    result.unwrap_or_else(IntoResponse::into_response)
}

/// The upstream JSON to return for a single-object view of a result
const fn pick(result: &RdapQueryResult, source: Source) -> (&'static str, &Value) {
    match (source, &result.registrar_json) {
        (Source::Registrar | Source::Both, Some(registrar)) => ("registrar", registrar),
        _ => ("registry", &result.registry_json),
    }
}

// ---------------------------------------------------------------------------
// RDAP-compatible endpoints (RFC 7482)
// ---------------------------------------------------------------------------

/// Look up `raw` as `query_type` and return the bare RDAP object
async fn rdap_object(
    state: Arc<AppState>,
    raw: &str,
    query_type: QueryType,
    params: RawParams,
) -> Result<Response, ApiError> {
    let source = params_source(params)?;
    let request = state.build_request(raw, Some(query_type))?;
    let result = state.lookup(request).await?;
    let (_, object) = pick(&result, source);
    Ok(json_response(StatusCode::OK, RDAP_JSON, object))
}

/// `?source=` on an RDAP endpoint: only one object can be returned
fn params_source(raw: RawParams) -> Result<Source, ApiError> {
    match params(raw)?.source()? {
        Source::Both => Err(ApiError::bad_request(
            "source=both is only supported on /query",
        )),
        source => Ok(source),
    }
}

async fn rdap_lookup(
    state: Arc<AppState>,
    path: RawPath,
    query_type: QueryType,
    params: RawParams,
) -> Response {
    let result = async {
        let raw = path_value(path)?;
        rdap_object(state, &raw, query_type, params).await
    }
    .await;
    into_response(result.map_err(ApiError::rdap))
}

pub async fn domain(State(state): Shared, path: RawPath, params: RawParams) -> Response {
    // `rdap -s <this server> com` sends a TLD query as /domain/com
    let query_type = match &path {
        Ok(Path(name)) if !name.contains('.') && state.is_tld(name.trim()) => QueryType::Tld,
        _ => QueryType::Domain,
    };
    rdap_lookup(state, path, query_type, params).await
}

pub async fn ip(State(state): Shared, path: RawPath, params: RawParams) -> Response {
    rdap_lookup(state, path, QueryType::Ip, params).await
}

pub async fn autnum(State(state): Shared, path: RawPath, params: RawParams) -> Response {
    rdap_lookup(state, path, QueryType::Autnum, params).await
}

pub async fn nameserver(State(state): Shared, path: RawPath, params: RawParams) -> Response {
    rdap_lookup(state, path, QueryType::Nameserver, params).await
}

pub async fn entity(State(state): Shared, path: RawPath, params: RawParams) -> Response {
    rdap_lookup(state, path, QueryType::Entity, params).await
}

/// RFC 7482 searches: the first recognised parameter selects the search type
async fn rdap_search(
    state: Arc<AppState>,
    params: Result<Query<HashMap<String, String>>, QueryRejection>,
    kinds: &[(&str, QueryType)],
) -> Response {
    let result = async {
        let Query(params) = params.map_err(|e| {
            ApiError::bad_request(format!("Invalid query string: {}", e.body_text()))
        })?;
        let (raw, query_type) = kinds
            .iter()
            .find_map(|(name, qt)| params.get(*name).map(|v| (v, *qt)))
            .ok_or_else(|| {
                ApiError::bad_request(format!(
                    "Missing search parameter; expected one of: {}",
                    kinds.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
                ))
            })?;
        let request = state.build_request(raw, Some(query_type))?;
        let result = state.lookup(request).await?;
        Ok(json_response(
            StatusCode::OK,
            RDAP_JSON,
            &result.registry_json,
        ))
    }
    .await;
    into_response(result.map_err(ApiError::rdap))
}

pub async fn domains(
    State(state): Shared,
    params: Result<Query<HashMap<String, String>>, QueryRejection>,
) -> Response {
    let kinds = [
        ("name", QueryType::DomainSearch),
        ("nsLdhName", QueryType::DomainSearchByNameserver),
        ("nsIp", QueryType::DomainSearchByNameserverIp),
    ];
    rdap_search(state, params, &kinds).await
}

pub async fn nameservers(
    State(state): Shared,
    params: Result<Query<HashMap<String, String>>, QueryRejection>,
) -> Response {
    let kinds = [
        ("name", QueryType::NameserverSearch),
        ("ip", QueryType::NameserverSearchByIp),
    ];
    rdap_search(state, params, &kinds).await
}

pub async fn entities(
    State(state): Shared,
    params: Result<Query<HashMap<String, String>>, QueryRejection>,
) -> Response {
    let kinds = [
        ("fn", QueryType::EntitySearch),
        ("handle", QueryType::EntitySearchByHandle),
    ];
    rdap_search(state, params, &kinds).await
}

/// RDAP help object describing this server
pub async fn help() -> Response {
    let body = serde_json::json!({
        "rdapConformance": ["rdap_level_0"],
        "notices": [{
            "title": "rdap API server",
            "description": [
                concat!("rdap ", env!("CARGO_PKG_VERSION"), " RDAP proxy and API server."),
                "RDAP lookups: /domain/{name} /ip/{addr} /ip/{addr}/{len} /autnum/{asn} /nameserver/{name} /entity/{handle}",
                "RDAP searches: /domains?name= /nameservers?name= /entities?fn=",
                "JSON API: /query/{q}?type=&source=registrar|registry|both /abuse/{q} POST /batch /healthz /version",
            ],
            "links": [{
                "rel": "related",
                "href": "https://github.com/xtomcom/rdap",
                "type": "text/html",
            }],
        }],
    });
    json_response(StatusCode::OK, RDAP_JSON, &body)
}

// ---------------------------------------------------------------------------
// Convenience endpoints
// ---------------------------------------------------------------------------

/// `/query` response body
#[derive(Serialize)]
struct QueryView<'a> {
    query: &'a str,
    #[serde(rename = "type")]
    query_type: String,
    source: &'static str,
    /// Upstream URLs are shown without credentials (e.g. from `-s https://user:pass@...`)
    registry_url: String,
    registrar_url: Option<String>,
    #[serde(flatten)]
    objects: Objects<'a>,
}

/// `"result"` alone, or (`source=both`) `"registry"` and `"registrar"`
#[derive(Serialize)]
#[serde(untagged)]
enum Objects<'a> {
    One {
        result: &'a Value,
    },
    Both {
        registry: &'a Value,
        registrar: Option<&'a Value>,
    },
}

impl<'a> QueryView<'a> {
    fn new(request: &'a RdapRequest, result: &'a RdapQueryResult, source: Source) -> Self {
        let (source, objects) = if source == Source::Both {
            (
                "both",
                Objects::Both {
                    registry: &result.registry_json,
                    registrar: result.registrar_json.as_ref(),
                },
            )
        } else {
            let (name, object) = pick(result, source);
            (name, Objects::One { result: object })
        };
        Self {
            query: &request.query,
            query_type: request.query_type.to_string(),
            source,
            registry_url: public_url(&result.registry_url),
            registrar_url: result.registrar_url.as_ref().map(public_url),
            objects,
        }
    }
}

pub async fn query(State(state): Shared, path: RawPath, params: RawParams) -> Response {
    let result = async {
        let raw = path_value(path)?;
        let params = self::params(params)?;
        let source = params.source()?;
        let request = state.build_request(&raw, params.query_type()?)?;
        let result = state.lookup(request.clone()).await?;
        Ok(json_response(
            StatusCode::OK,
            JSON,
            &QueryView::new(&request, &result, source),
        ))
    }
    .await;
    into_response(result)
}

/// `/abuse` response body
#[derive(Serialize)]
struct AbuseView<'a> {
    query: &'a str,
    #[serde(rename = "type")]
    query_type: String,
    abuse: Option<String>,
    source_url: String,
}

/// Upstream URL as shown to API clients: without credentials
fn public_url(url: &Url) -> String {
    crate::net::without_credentials(url).into()
}

/// Abuse email in an object's entities
fn abuse_in(object: &RdapObject) -> Option<String> {
    match object {
        RdapObject::Domain(d) => find_abuse_contact(&d.entities),
        RdapObject::IpNetwork(n) => find_abuse_contact(&n.entities),
        RdapObject::Autnum(a) => find_abuse_contact(&a.entities),
        RdapObject::Nameserver(n) => find_abuse_contact(&n.entities),
        RdapObject::Entity(e) => find_abuse_contact(std::slice::from_ref(e)),
        _ => None,
    }
}

/// Abuse email and the URL it came from: registrar data first, then registry
fn find_abuse(result: &RdapQueryResult) -> (Option<String>, &Url) {
    if let (Some(registrar), Some(url)) = (&result.registrar, &result.registrar_url)
        && let Some(email) = abuse_in(registrar)
    {
        return (Some(email), url);
    }
    if let Some(email) = abuse_in(&result.registry) {
        return (Some(email), &result.registry_url);
    }
    let url = match (&result.registrar, &result.registrar_url) {
        (Some(_), Some(url)) => url,
        _ => &result.registry_url,
    };
    (None, url)
}

pub async fn abuse(State(state): Shared, path: RawPath, params: RawParams) -> Response {
    let result = async {
        let raw = path_value(path)?;
        let request = state.build_request(&raw, self::params(params)?.query_type()?)?;
        if !matches!(
            request.query_type,
            QueryType::Domain
                | QueryType::Tld
                | QueryType::Ip
                | QueryType::Autnum
                | QueryType::Nameserver
                | QueryType::Entity
        ) {
            return Err(ApiError::bad_request(format!(
                "Abuse lookup is not supported for {} queries",
                request.query_type
            )));
        }
        let result = state.lookup(request.clone()).await?;
        let (abuse, source_url) = find_abuse(&result);
        Ok(json_response(
            StatusCode::OK,
            JSON,
            &AbuseView {
                query: &request.query,
                query_type: request.query_type.to_string(),
                abuse,
                source_url: public_url(source_url),
            },
        ))
    }
    .await;
    into_response(result)
}

#[derive(Deserialize)]
struct BatchRequest {
    queries: Vec<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default, rename = "type")]
    query_type: Option<String>,
}

#[derive(Serialize)]
struct BatchItem<'a> {
    query: &'a str,
    ok: bool,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<QueryView<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct BatchResponse<'a> {
    results: Vec<BatchItem<'a>>,
}

type Outcome = Result<Arc<RdapQueryResult>, ApiError>;

pub async fn batch(State(state): Shared, body: Result<Bytes, BytesRejection>) -> Response {
    let result = async {
        let body = body.map_err(|e| {
            if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
                ApiError::new(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    format!("Request body is larger than {MAX_BATCH_BODY} bytes"),
                )
            } else {
                ApiError::bad_request(format!("Invalid request body: {}", e.body_text()))
            }
        })?;
        let batch: BatchRequest = serde_json::from_slice(&body).map_err(|e| {
            ApiError::bad_request(format!(
                "Invalid batch body (expected {{\"queries\": [\"...\"]}}): {e}"
            ))
        })?;
        if batch.queries.len() > state.opts.max_batch {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!(
                    "Batch has {} queries; the maximum is {}",
                    batch.queries.len(),
                    state.opts.max_batch
                ),
            ));
        }
        let source = Source::parse(batch.source.as_deref())?;
        let query_type = batch
            .query_type
            .as_deref()
            .map(parse_query_type)
            .transpose()?;

        // Validate every query; identical normalized queries run once
        let mut unique: Vec<RdapRequest> = Vec::new();
        let mut seen: HashMap<(String, String), usize> = HashMap::new();
        let slots: Vec<Result<usize, ApiError>> = batch
            .queries
            .iter()
            .map(|raw| {
                let request = state.build_request(raw, query_type)?;
                let key = (request.query_type.to_string(), request.query.clone());
                Ok(match seen.entry(key) {
                    Entry::Occupied(e) => *e.get(),
                    Entry::Vacant(e) => {
                        unique.push(request);
                        *e.insert(unique.len() - 1)
                    }
                })
            })
            .collect();

        // A JoinSet aborts the lookups when this request is dropped (client gone,
        // request timeout), so abandoned batches stop using upstream capacity
        let semaphore = Arc::new(Semaphore::new(BATCH_CONCURRENCY));
        let mut tasks = JoinSet::new();
        for (i, request) in unique.iter().cloned().enumerate() {
            let state = Arc::clone(&state);
            let semaphore = Arc::clone(&semaphore);
            tasks.spawn(async move {
                let _permit = semaphore.acquire_owned().await;
                let _global = Arc::clone(&state.batch_slots).acquire_owned().await;
                (i, state.lookup(request).await)
            });
        }
        let mut outcomes: Vec<Option<Outcome>> = vec![None; unique.len()];
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok((i, outcome)) => outcomes[i] = Some(outcome),
                Err(e) => log::error!("Batch query task failed: {e}"),
            }
        }
        let outcomes: Vec<Outcome> = outcomes
            .into_iter()
            .map(|outcome| {
                outcome.unwrap_or_else(|| {
                    Err(ApiError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Query task failed",
                    ))
                })
            })
            .collect();

        let results = batch
            .queries
            .iter()
            .zip(&slots)
            .map(|(raw, slot)| {
                let outcome = slot.as_ref().map_err(Clone::clone).and_then(|&i| {
                    match (&outcomes[i], &unique[i]) {
                        (Ok(result), request) => Ok((request, result.as_ref())),
                        (Err(err), _) => Err(err.clone()),
                    }
                });
                match outcome {
                    Ok((request, result)) => BatchItem {
                        query: raw,
                        ok: true,
                        status: StatusCode::OK.as_u16(),
                        data: Some(QueryView::new(request, result, source)),
                        error: None,
                    },
                    Err(err) => BatchItem {
                        query: raw,
                        ok: false,
                        status: err.status().as_u16(),
                        data: None,
                        error: Some(err.body()),
                    },
                }
            })
            .collect();
        Ok(json_response(
            StatusCode::OK,
            JSON,
            &BatchResponse { results },
        ))
    }
    .await;
    into_response(result)
}

pub async fn healthz() -> Response {
    json_response(StatusCode::OK, JSON, &serde_json::json!({ "status": "ok" }))
}

pub async fn version() -> Response {
    json_response(
        StatusCode::OK,
        JSON,
        &serde_json::json!({ "name": "rdap", "version": env!("CARGO_PKG_VERSION") }),
    )
}

pub async fn not_found() -> ApiError {
    ApiError::not_found("Unknown endpoint")
}

pub async fn method_not_allowed() -> ApiError {
    ApiError::new(StatusCode::METHOD_NOT_ALLOWED, "Method not allowed")
}
