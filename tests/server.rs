//! Integration tests for the API server, driving the router directly
//! against a mockito upstream (no real network).
#![cfg(feature = "server")]
// The mockito server must outlive the requests, whatever clippy thinks
#![allow(clippy::significant_drop_tightening)]

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mockito::{Matcher, Mock, Server, ServerGuard};
use rdap::server::{AppState, ServeOptions, router};
use rdap::{NetOptions, RdapClient};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

const RDAP: &str = "application/rdap+json";

fn client(timeout: Duration) -> Arc<RdapClient> {
    let opts = NetOptions {
        timeout,
        ..NetOptions::default()
    };
    Arc::new(RdapClient::with_options(&opts).unwrap())
}

fn app_with(opts: ServeOptions, timeout: Duration) -> Router {
    router(Arc::new(AppState::new(opts, client(timeout))))
}

/// Router sending every query to `upstream`
fn app(upstream: &ServerGuard) -> Router {
    app_with(options(upstream), Duration::from_secs(5))
}

fn options(upstream: &ServerGuard) -> ServeOptions {
    ServeOptions {
        upstream: Some(format!("{}/", upstream.url()).parse().unwrap()),
        ..ServeOptions::default()
    }
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

async fn send(app: &Router, request: Request<Body>) -> Reply {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(!bytes.is_empty(), "empty body for {status}");
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("non-JSON body for {status}: {e}: {bytes:?}"));
    Reply {
        status,
        headers,
        body,
    }
}

async fn get(app: &Router, uri: &str) -> Reply {
    send(app, Request::get(uri).body(Body::empty()).unwrap()).await
}

async fn post_json(app: &Router, uri: &str, body: impl Into<Body>) -> Reply {
    let request = Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.into())
        .unwrap();
    send(app, request).await
}

fn content_type(reply: &Reply) -> &str {
    reply.headers[header::CONTENT_TYPE].to_str().unwrap()
}

/// Assert an RFC 7480 error object with `status`
fn assert_error(reply: &Reply, status: u16) {
    assert_eq!(reply.status.as_u16(), status, "body: {}", reply.body);
    assert_eq!(reply.body["errorCode"], status, "body: {}", reply.body);
    assert!(reply.body["title"].is_string());
    assert!(reply.body["description"].is_array());
}

fn abuse_entity(email: &str) -> Value {
    json!({
        "objectClassName": "entity",
        "handle": "ABUSE-1",
        "roles": ["abuse"],
        "vcardArray": ["vcard", [
            ["version", {}, "text", "4.0"],
            ["fn", {}, "text", "Abuse desk"],
            ["email", {}, "text", email]
        ]]
    })
}

fn ip_network(start: &str, end: &str, entities: &[Value]) -> String {
    json!({
        "objectClassName": "ip network",
        "rdapConformance": ["rdap_level_0"],
        "handle": format!("{start} - {end}"),
        "startAddress": start,
        "endAddress": end,
        "ipVersion": "v4",
        "name": "TEST-NET",
        "entities": entities
    })
    .to_string()
}

async fn mock_json(server: &mut ServerGuard, path: &str, status: usize, body: &str) -> Mock {
    server
        .mock("GET", path)
        .with_status(status)
        .with_header("content-type", RDAP)
        .with_body(body)
        .create_async()
        .await
}

#[tokio::test]
async fn ip_lookup_and_cache_hit() {
    let mut upstream = Server::new_async().await;
    let body = ip_network("1.1.1.0", "1.1.1.255", &[abuse_entity("abuse@example.net")]);
    let mock = upstream
        .mock("GET", "/ip/1.1.1.1")
        .with_header("content-type", RDAP)
        .with_body(&body)
        .expect(1)
        .create_async()
        .await;
    let app = app(&upstream);

    let reply = get(&app, "/ip/1.1.1.1").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(content_type(&reply), RDAP);
    assert_eq!(reply.headers["x-content-type-options"], "nosniff");
    assert_eq!(reply.body["objectClassName"], "ip network");
    assert_eq!(reply.body["startAddress"], "1.1.1.0");

    // Same query through other endpoints: served from the cache
    let reply = get(&app, "/query/1.1.1.1").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(content_type(&reply), "application/json");
    assert_eq!(reply.body["query"], "1.1.1.1");
    assert_eq!(reply.body["type"], "ip");
    assert_eq!(reply.body["source"], "registry");
    assert_eq!(reply.body["registrar_url"], Value::Null);
    assert_eq!(
        reply.body["registry_url"],
        format!("{}/ip/1.1.1.1", upstream.url())
    );
    assert_eq!(reply.body["result"]["name"], "TEST-NET");

    let reply = get(&app, "/abuse/1.1.1.1").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["abuse"], "abuse@example.net");
    assert_eq!(reply.body["type"], "ip");

    mock.assert_async().await;
}

#[tokio::test]
async fn upstream_json_passes_through_unchanged() {
    // Fields the models don't know (RFC 9537 redaction, vendor extensions,
    // nested unknowns) must survive the proxy
    let mut upstream = Server::new_async().await;
    let mut body: Value = serde_json::from_str(&ip_network(
        "192.0.2.0",
        "192.0.2.255",
        &[abuse_entity("abuse@example.net")],
    ))
    .unwrap();
    body["redacted"] = json!([{"name": {"type": "Registrant Name"}, "method": "removal"}]);
    body["vendor_extension"] = json!({"x": 1});
    body["entities"][0]["vendorEntityField"] = json!("kept");
    let _mock = mock_json(&mut upstream, "/ip/192.0.2.1", 200, &body.to_string()).await;
    let app = app(&upstream);

    let reply = get(&app, "/ip/192.0.2.1").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, body);

    let reply = get(&app, "/query/192.0.2.1").await;
    assert_eq!(reply.body["result"], body);
}

#[tokio::test]
async fn upstream_credentials_are_not_exposed() {
    let mut upstream = Server::new_async().await;
    let body = ip_network("192.0.2.0", "192.0.2.255", &[abuse_entity("a@example.net")]);
    let _mock = mock_json(&mut upstream, "/ip/192.0.2.9", 200, &body).await;
    let with_creds = upstream.url().replace("http://", "http://svc:UpPass77@");
    let app = app_with(
        ServeOptions {
            upstream: Some(format!("{with_creds}/").parse().unwrap()),
            ..ServeOptions::default()
        },
        Duration::from_secs(5),
    );

    for uri in ["/query/192.0.2.9", "/abuse/192.0.2.9"] {
        let reply = get(&app, uri).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(
            !reply.body.to_string().contains("UpPass77"),
            "{uri}: {}",
            reply.body
        );
    }
}

#[tokio::test]
async fn shorthand_and_cidr_paths() {
    let mut upstream = Server::new_async().await;
    let cidr = mock_json(
        &mut upstream,
        "/ip/1.1.1.0/24",
        200,
        &ip_network("1.1.1.0", "1.1.1.255", &[]),
    )
    .await;
    let shorthand = mock_json(
        &mut upstream,
        "/ip/1.0.0.1",
        200,
        &ip_network("1.0.0.0", "1.0.0.255", &[]),
    )
    .await;
    let app = app(&upstream);

    let reply = get(&app, "/ip/1.1.1.0/24").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["startAddress"], "1.1.1.0");

    let reply = get(&app, "/query/1.1.1.0/24").await;
    assert_eq!(reply.body["query"], "1.1.1.0/24");

    // `1.1` expands like the CLI does
    let reply = get(&app, "/query/1.1").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["query"], "1.0.0.1");

    cidr.assert_async().await;
    shorthand.assert_async().await;
}

#[tokio::test]
async fn autnum_and_abuse_null() {
    let mut upstream = Server::new_async().await;
    let body = json!({
        "objectClassName": "autnum",
        "handle": "AS64512",
        "startAutnum": 64512,
        "endAutnum": 64512,
        "name": "PRIVATE"
    })
    .to_string();
    let mock = mock_json(&mut upstream, "/autnum/64512", 200, &body).await;
    let app = app(&upstream);

    for uri in ["/autnum/64512", "/autnum/AS64512"] {
        let reply = get(&app, uri).await;
        assert_eq!(reply.status, StatusCode::OK, "{uri}");
        assert_eq!(reply.body["handle"], "AS64512");
    }

    let reply = get(&app, "/query/AS64512").await;
    assert_eq!(reply.body["query"], "AS64512");
    assert_eq!(reply.body["type"], "autnum");

    // Object found, no abuse contact: 200 with null, not an error
    let reply = get(&app, "/abuse/64512").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["abuse"], Value::Null);
    assert_eq!(
        reply.body["source_url"],
        format!("{}/autnum/64512", upstream.url())
    );

    mock.assert_async().await;
}

fn domain(name: &str, links: &[Value], entities: &[Value]) -> String {
    json!({
        "objectClassName": "domain",
        "ldhName": name,
        "links": links,
        "entities": entities
    })
    .to_string()
}

#[tokio::test]
async fn domain_with_registrar_referral() {
    let mut upstream = Server::new_async().await;
    // A different host name than 127.0.0.1, or the client skips the referral
    let referral = format!(
        "http://localhost:{}/registrar/domain/example.com",
        upstream.socket_address().port()
    );
    let link = json!({"rel": "related", "href": referral, "type": RDAP});
    let registry = mock_json(
        &mut upstream,
        "/domain/example.com",
        200,
        &domain("EXAMPLE.COM", &[link], &[]),
    )
    .await;
    let registrar = mock_json(
        &mut upstream,
        "/registrar/domain/example.com",
        200,
        &domain(
            "example.com",
            &[],
            &[json!({
                "objectClassName": "entity",
                "roles": ["registrar"],
                "entities": [abuse_entity("abuse@registrar.example")]
            })],
        ),
    )
    .await;
    let app = app(&upstream);

    // RDAP endpoint prefers registrar data
    let reply = get(&app, "/domain/example.com").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["ldhName"], "example.com");
    let reply = get(&app, "/domain/example.com?source=registry").await;
    assert_eq!(reply.body["ldhName"], "EXAMPLE.COM");
    assert_error(&get(&app, "/domain/example.com?source=both").await, 400);

    let reply = get(&app, "/query/example.com").await;
    assert_eq!(reply.body["type"], "domain");
    assert_eq!(reply.body["source"], "registrar");
    assert_eq!(reply.body["registrar_url"], referral);
    assert_eq!(reply.body["result"]["ldhName"], "example.com");

    let reply = get(&app, "/query/example.com?source=both").await;
    assert_eq!(reply.body["source"], "both");
    assert_eq!(reply.body["registry"]["ldhName"], "EXAMPLE.COM");
    assert_eq!(reply.body["registrar"]["ldhName"], "example.com");
    assert!(reply.body.get("result").is_none());

    // Abuse found in nested registrar entity
    let reply = get(&app, "/abuse/example.com").await;
    assert_eq!(reply.body["abuse"], "abuse@registrar.example");
    assert_eq!(reply.body["source_url"], referral);

    registry.assert_async().await;
    registrar.assert_async().await;
}

#[tokio::test]
async fn referral_failure_gives_registry_only_result() {
    let mut upstream = Server::new_async().await;
    let referral = format!(
        "http://localhost:{}/registrar/domain/broken.com",
        upstream.socket_address().port()
    );
    let link = json!({"rel": "related", "href": referral, "type": RDAP});
    mock_json(
        &mut upstream,
        "/domain/broken.com",
        200,
        &domain(
            "BROKEN.COM",
            &[link],
            &[abuse_entity("abuse@registry.example")],
        ),
    )
    .await;
    mock_json(&mut upstream, "/registrar/domain/broken.com", 500, "{}").await;
    let app = app(&upstream);

    let reply = get(&app, "/query/broken.com?source=both").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["registry"]["ldhName"], "BROKEN.COM");
    assert_eq!(reply.body["registrar"], Value::Null);
    assert_eq!(reply.body["registrar_url"], Value::Null);

    let reply = get(&app, "/domain/broken.com").await;
    assert_eq!(reply.body["ldhName"], "BROKEN.COM");

    // Registry fallback for abuse
    let reply = get(&app, "/abuse/broken.com").await;
    assert_eq!(reply.body["abuse"], "abuse@registry.example");
}

#[tokio::test]
async fn registrar_error_object_is_dropped() {
    let mut upstream = Server::new_async().await;
    let referral = format!(
        "http://localhost:{}/registrar/domain/gone.com",
        upstream.socket_address().port()
    );
    let link = json!({"rel": "related", "href": referral, "type": RDAP});
    mock_json(
        &mut upstream,
        "/domain/gone.com",
        200,
        &domain("GONE.COM", &[link], &[]),
    )
    .await;
    // 200 carrying an RDAP error object
    mock_json(
        &mut upstream,
        "/registrar/domain/gone.com",
        200,
        r#"{"errorCode": 404, "title": "Not Found"}"#,
    )
    .await;
    let app = app(&upstream);

    let reply = get(&app, "/query/gone.com").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["source"], "registry");
    assert_eq!(reply.body["result"]["ldhName"], "GONE.COM");
    assert_eq!(reply.body["registrar_url"], Value::Null);
}

#[tokio::test]
async fn not_found_cases() {
    let mut upstream = Server::new_async().await;
    let missing = upstream
        .mock("GET", "/domain/missing.com")
        .with_status(404)
        .expect_at_least(1)
        // The client may retry once; the cache must absorb the repeats
        .expect_at_most(2)
        .create_async()
        .await;
    mock_json(
        &mut upstream,
        "/domain/errobj.com",
        200,
        r#"{"errorCode": 404, "title": "Not Found", "description": ["nope"]}"#,
    )
    .await;
    upstream
        .mock("GET", "/domains")
        .match_query(Matcher::UrlEncoded("name".into(), "nothing*".into()))
        .with_header("content-type", RDAP)
        .with_body(r#"{"rdapConformance": ["rdap_level_0"], "domainSearchResults": []}"#)
        .create_async()
        .await;
    let app = app(&upstream);

    for _ in 0..3 {
        let reply = get(&app, "/domain/missing.com").await;
        assert_error(&reply, 404);
        assert_eq!(content_type(&reply), RDAP);
        assert_eq!(reply.body["rdapConformance"][0], "rdap_level_0");
        assert_eq!(
            reply.body["description"][0],
            "No RDAP record for missing.com"
        );
    }
    missing.assert_async().await;

    let reply = get(&app, "/query/missing.com").await;
    assert_error(&reply, 404);
    assert_eq!(content_type(&reply), "application/json");

    // 200 with an RDAP error object is not a success
    assert_error(&get(&app, "/domain/errobj.com").await, 404);
    assert_error(&get(&app, "/abuse/errobj.com").await, 404);

    // Empty search results
    assert_error(&get(&app, "/domains?name=nothing*").await, 404);
    assert_error(&get(&app, "/query/nothing*?type=domain-search").await, 404);

    // Unknown route
    assert_error(&get(&app, "/nope").await, 404);
    assert_error(&get(&app, "/").await, 404);
}

#[tokio::test]
async fn search_results() {
    let mut upstream = Server::new_async().await;
    upstream
        .mock("GET", "/domains")
        .match_query(Matcher::UrlEncoded("name".into(), "exa*.com".into()))
        .with_header("content-type", RDAP)
        .with_body(json!({"domainSearchResults": [{"objectClassName": "domain", "ldhName": "example.com"}]}).to_string())
        .create_async()
        .await;
    let app = app(&upstream);

    let reply = get(&app, "/domains?name=exa*.com").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.body["domainSearchResults"][0]["ldhName"],
        "example.com"
    );
    assert_error(&get(&app, "/domains").await, 400);
}

#[tokio::test]
async fn upstream_error_mapping() {
    let mut upstream = Server::new_async().await;
    mock_json(
        &mut upstream,
        "/ip/192.0.2.1",
        429,
        r#"{"errorCode": 429, "title": "Slow down"}"#,
    )
    .await;
    mock_json(&mut upstream, "/ip/192.0.2.2", 500, "internal error").await;
    mock_json(&mut upstream, "/ip/192.0.2.3", 200, "this is not json").await;
    // Valid JSON, wrong object class for an IP query
    mock_json(
        &mut upstream,
        "/ip/192.0.2.4",
        200,
        r#"{"objectClassName": "domain", "ldhName": "x"}"#,
    )
    .await;
    mock_json(&mut upstream, "/ip/192.0.2.5", 200, "{}").await;
    let app = app(&upstream);

    assert_error(&get(&app, "/ip/192.0.2.1").await, 429);
    assert_error(&get(&app, "/ip/192.0.2.2").await, 502);
    assert_error(&get(&app, "/ip/192.0.2.3").await, 502);
    assert_error(&get(&app, "/ip/192.0.2.4").await, 502);
    assert_error(&get(&app, "/query/192.0.2.5").await, 502);
}

#[tokio::test]
async fn upstream_timeout_is_504() {
    let mut upstream = Server::new_async().await;
    upstream
        .mock("GET", "/ip/192.0.2.9")
        .with_header("content-type", RDAP)
        .with_chunked_body(|w| {
            std::thread::sleep(Duration::from_millis(800));
            w.write_all(b"{}")
        })
        .create_async()
        .await;
    let app = app_with(options(&upstream), Duration::from_millis(200));

    assert_error(&get(&app, "/ip/192.0.2.9").await, 504);
}

#[tokio::test]
async fn upstream_unreachable_is_502() {
    // Nothing listens on port 9 of localhost
    let opts = ServeOptions {
        upstream: Some("http://127.0.0.1:9/".parse().unwrap()),
        ..ServeOptions::default()
    };
    let app = app_with(opts, Duration::from_secs(2));
    assert_error(&get(&app, "/ip/192.0.2.1").await, 502);
}

#[tokio::test]
async fn hostile_input_is_rejected() {
    let upstream = Server::new_async().await;
    let app = app(&upstream);

    let long = format!("/query/{}.com", "a".repeat(10_000));
    for uri in [
        "/ip/..%2F..%2Fadmin",
        "/ip/1.1.1.1%2F..",
        "/ip/not-an-ip",
        "/ip/1.1.1.0/abc",
        "/autnum/99999999999",
        "/autnum/AS-1",
        "/domain/%FF%FE",
        "/query/%FF",
        "/query/a%20b.com",
        "/query/%00",
        "/query/1.1.1.1?type=bogus",
        "/query/1.1.1.1?source=nobody",
        "/query/1.1.1.1?type=ip&type=domain",
        long.as_str(),
    ] {
        let reply = get(&app, uri).await;
        assert_error(&reply, 400);
    }
}

#[tokio::test]
async fn path_tricks_do_not_escape() {
    let mut upstream = Server::new_async().await;
    // `..` stays inside the encoded domain segment
    let mock = upstream
        .mock("GET", "/domain/..%2Fadmin")
        .with_status(404)
        .expect_at_least(1)
        .create_async()
        .await;
    let app = app(&upstream);

    assert_error(&get(&app, "/domain/..%2Fadmin").await, 404);
    mock.assert_async().await;
}

#[tokio::test]
async fn needs_explicit_upstream() {
    // No upstream: entity lookups and searches can't be bootstrapped
    let app = app_with(ServeOptions::default(), Duration::from_secs(2));
    assert_error(&get(&app, "/entity/ABC-1").await, 501);
    assert_error(&get(&app, "/entities?fn=Jane*").await, 501);
    assert_error(&get(&app, "/query/x?type=entity").await, 501);
    assert_error(&get(&app, "/query/exa*?type=domain-search").await, 501);
    // Local endpoints still work
    assert_eq!(get(&app, "/healthz").await.status, StatusCode::OK);
}

#[tokio::test]
async fn entity_with_upstream() {
    let mut upstream = Server::new_async().await;
    let body = abuse_entity("abuse@example.net").to_string();
    mock_json(&mut upstream, "/entity/ABUSE-1", 200, &body).await;
    let app = app(&upstream);

    let reply = get(&app, "/entity/ABUSE-1").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["handle"], "ABUSE-1");
}

#[tokio::test]
async fn local_endpoints() {
    let app = app_with(ServeOptions::default(), Duration::from_secs(2));

    let reply = get(&app, "/healthz").await;
    assert_eq!(reply.body, json!({"status": "ok"}));

    let reply = get(&app, "/version").await;
    assert_eq!(reply.body["name"], "rdap");
    assert_eq!(reply.body["version"], env!("CARGO_PKG_VERSION"));

    let reply = get(&app, "/help").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(content_type(&reply), RDAP);
    assert_eq!(reply.body["rdapConformance"][0], "rdap_level_0");
    assert!(reply.body["notices"].is_array());
}

#[tokio::test]
async fn wrong_method_is_405() {
    let app = app_with(ServeOptions::default(), Duration::from_secs(2));

    let reply = send(&app, Request::post("/healthz").body(Body::empty()).unwrap()).await;
    assert_error(&reply, 405);
    assert!(reply.headers.contains_key(header::ALLOW));

    let reply = send(
        &app,
        Request::builder()
            .method(Method::DELETE)
            .uri("/ip/1.1.1.1")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_error(&reply, 405);

    let reply = get(&app, "/batch").await;
    assert_error(&reply, 405);
}

#[tokio::test]
async fn auth() {
    let opts = ServeOptions {
        token: Some("s3cret".into()),
        ..ServeOptions::default()
    };
    let app = app_with(opts, Duration::from_secs(2));
    let with_auth = |value: &str| {
        Request::get("/version")
            .header(header::AUTHORIZATION, value)
            .body(Body::empty())
            .unwrap()
    };

    let reply = get(&app, "/version").await;
    assert_error(&reply, 401);
    assert_eq!(reply.headers[header::WWW_AUTHENTICATE], "Bearer");

    for wrong in [
        "Bearer nope",
        "Bearer s3cre",
        "Bearer s3cret2",
        "Basic s3cret",
        "s3cret",
        "",
    ] {
        assert_error(&send(&app, with_auth(wrong)).await, 401);
    }
    // Unknown routes are protected too
    assert_error(&get(&app, "/nope").await, 401);

    let reply = send(&app, with_auth("Bearer s3cret")).await;
    assert_eq!(reply.status, StatusCode::OK);
    let reply = send(&app, with_auth("bearer s3cret")).await;
    assert_eq!(reply.status, StatusCode::OK);

    // /healthz never requires the token
    assert_eq!(get(&app, "/healthz").await.status, StatusCode::OK);
}

#[tokio::test]
async fn batch_order_dedup_and_errors() {
    let mut upstream = Server::new_async().await;
    let one = upstream
        .mock("GET", "/ip/1.1.1.1")
        .with_header("content-type", RDAP)
        .with_body(ip_network("1.1.1.0", "1.1.1.255", &[]))
        .expect(1)
        .create_async()
        .await;
    let asn = upstream
        .mock("GET", "/autnum/13335")
        .with_header("content-type", RDAP)
        .with_body(json!({"objectClassName": "autnum", "handle": "AS13335"}).to_string())
        .expect(1)
        .create_async()
        .await;
    upstream
        .mock("GET", "/domain/missing.com")
        .with_status(404)
        .create_async()
        .await;
    // Cache off: dedup must not rely on it
    let opts = ServeOptions {
        cache_ttl: Duration::ZERO,
        ..options(&upstream)
    };
    let app = app_with(opts, Duration::from_secs(5));

    let body = json!({
        "queries": ["1.1.1.1", "AS13335", "missing.com", "not an ip", "1.1.1.1", "13335"],
        "source": "registry"
    });
    let reply = post_json(&app, "/batch", body.to_string()).await;
    assert_eq!(reply.status, StatusCode::OK);
    let results = reply.body["results"].as_array().unwrap();
    assert_eq!(results.len(), 6);

    let queries: Vec<&str> = results
        .iter()
        .map(|r| r["query"].as_str().unwrap())
        .collect();
    assert_eq!(
        queries,
        [
            "1.1.1.1",
            "AS13335",
            "missing.com",
            "not an ip",
            "1.1.1.1",
            "13335"
        ]
    );
    let statuses: Vec<u64> = results
        .iter()
        .map(|r| r["status"].as_u64().unwrap())
        .collect();
    assert_eq!(statuses, [200, 200, 404, 400, 200, 200]);

    assert_eq!(results[0]["ok"], true);
    assert_eq!(results[0]["data"]["type"], "ip");
    assert_eq!(results[0]["data"]["source"], "registry");
    assert_eq!(results[0]["data"]["result"]["startAddress"], "1.1.1.0");
    assert_eq!(results[5]["data"]["query"], "AS13335");
    assert_eq!(results[2]["ok"], false);
    assert_eq!(results[2]["error"]["errorCode"], 404);
    assert!(results[2].get("data").is_none());
    assert_eq!(results[3]["error"]["errorCode"], 400);

    one.assert_async().await;
    asn.assert_async().await;
}

#[tokio::test]
async fn batch_limits_and_bad_bodies() {
    let opts = ServeOptions {
        max_batch: 3,
        ..ServeOptions::default()
    };
    let app = app_with(opts, Duration::from_secs(2));

    let too_many = json!({"queries": ["a.com", "b.com", "c.com", "d.com"]});
    assert_error(&post_json(&app, "/batch", too_many.to_string()).await, 413);

    let huge = format!(r#"{{"queries": ["{}"]}}"#, "a".repeat(70 * 1024));
    assert_error(&post_json(&app, "/batch", huge).await, 413);

    for bad in [
        "",
        "not json",
        "[]",
        r#"{"queries": "1.1.1.1"}"#,
        r#"{"queries": [1, 2]}"#,
        r#"{"queries": ["1.1.1.1"], "source": "nobody"}"#,
        r#"{"queries": ["1.1.1.1"], "type": "bogus"}"#,
    ] {
        assert_error(&post_json(&app, "/batch", bad).await, 400);
    }

    let reply = post_json(&app, "/batch", r#"{"queries": []}"#).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, json!({"results": []}));

    // Per-item errors don't fail the batch (entity needs an upstream: 501)
    let reply = post_json(&app, "/batch", r#"{"queries": ["X-1"], "type": "entity"}"#).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["results"][0]["status"], 501);
}

#[tokio::test]
async fn gzip_when_accepted() {
    let app = app_with(ServeOptions::default(), Duration::from_secs(2));
    let request = Request::get("/help")
        .header(header::ACCEPT_ENCODING, "gzip")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
}

#[tokio::test]
async fn cors_only_when_enabled() {
    let request = || {
        Request::get("/healthz")
            .header(header::ORIGIN, "https://example.org")
            .body(Body::empty())
            .unwrap()
    };
    let app = app_with(ServeOptions::default(), Duration::from_secs(2));
    let reply = send(&app, request()).await;
    assert!(
        !reply
            .headers
            .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN)
    );

    let opts = ServeOptions {
        cors: true,
        ..ServeOptions::default()
    };
    let app = app_with(opts, Duration::from_secs(2));
    let reply = send(&app, request()).await;
    assert!(
        reply
            .headers
            .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN)
    );
}
