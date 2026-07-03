//! Behavior tests running offline against an in-process mock server — no API
//! key or network access is required. This is the default `cargo test` tier;
//! live-API system tests live in `tests/integration.rs`.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ipregistry::{Cache, Client, Error, ErrorCode, InMemoryCache};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

async fn client_for(server: &MockServer) -> Client {
    Client::builder("test-key")
        .base_url(server.uri())
        .retry_interval(Duration::from_millis(1))
        .build()
        .unwrap()
}

/// A realistic single-IP response, including explicit `null` values as
/// returned by the live API for unknown data points.
fn sample_ip_info() -> serde_json::Value {
    json!({
        "ip": "8.8.8.8",
        "type": "IPv4",
        "hostname": null,
        "carrier": {"name": null, "mcc": null, "mnc": null},
        "company": {"domain": "google.com", "name": "Google LLC", "type": "business"},
        "connection": {
            "asn": 15169,
            "domain": "google.com",
            "organization": "Google LLC",
            "route": "8.8.8.0/24",
            "type": "business"
        },
        "currency": {
            "code": "USD",
            "name": "US Dollar",
            "name_native": "US Dollar",
            "plural": "US dollars",
            "plural_native": "US dollars",
            "symbol": "$",
            "symbol_native": "$",
            "format": {
                "decimal_separator": ".",
                "group_separator": ",",
                "negative": {"prefix": "-$", "suffix": ""},
                "positive": {"prefix": "$", "suffix": ""}
            }
        },
        "location": {
            "continent": {"code": "NA", "name": "North America"},
            "country": {
                "area": 9629091.0,
                "borders": ["CA", "MX"],
                "calling_code": "1",
                "capital": "Washington D.C.",
                "code": "US",
                "name": "United States",
                "population": 331002651u32,
                "population_density": 34.4,
                "flag": {
                    "emoji": "🇺🇸",
                    "emoji_unicode": "U+1F1FA U+1F1F8",
                    "emojitwo": "https://cdn.ipregistry.co/flags/emojitwo/us.svg",
                    "noto": "https://cdn.ipregistry.co/flags/noto/us.png",
                    "twemoji": "https://cdn.ipregistry.co/flags/twemoji/us.svg",
                    "wikimedia": "https://cdn.ipregistry.co/flags/wikimedia/us.svg"
                },
                "languages": [{"code": "en", "name": "English", "native": "English"}],
                "tld": ".us"
            },
            "region": {"code": "US-CA", "name": "California"},
            "city": "Mountain View",
            "postal": "94043",
            "latitude": 37.42240,
            "longitude": -122.08421,
            "language": {"code": "en", "name": "English", "native": "English"},
            "in_eu": false
        },
        "security": {
            "is_abuser": false,
            "is_attacker": false,
            "is_bogon": false,
            "is_cloud_provider": false,
            "is_proxy": false,
            "is_relay": false,
            "is_tor": false,
            "is_tor_exit": false,
            "is_anonymous": false,
            "is_threat": false,
            "is_vpn": false
        },
        "time_zone": {
            "id": "America/Los_Angeles",
            "abbreviation": "PDT",
            "current_time": "2024-05-01T10:00:00-07:00",
            "name": "Pacific Daylight Time",
            "offset": -25200,
            "in_daylight_saving": true
        }
    })
}

#[tokio::test]
async fn lookup_sends_expected_request_and_decodes_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/8.8.8.8"))
        .and(header("Authorization", "ApiKey test-key"))
        .and(header("Accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server).await;
    let info = client.lookup(ip("8.8.8.8")).await.unwrap();

    assert_eq!(info.ip, Some(ip("8.8.8.8")));
    assert_eq!(info.ip_type, Some(ipregistry::model::IpType::V4));
    assert_eq!(info.hostname, None, "null hostname decodes to None");
    assert_eq!(
        info.carrier.name, "",
        "null carrier name decodes to default"
    );
    assert_eq!(info.connection.asn, Some(15169));
    assert_eq!(
        info.company.company_type,
        Some(ipregistry::model::CompanyType::Business)
    );
    assert_eq!(info.location.country.code, "US");
    assert_eq!(info.location.country.borders, vec!["CA", "MX"]);
    assert_eq!(info.location.latitude, Some(37.42240));
    assert_eq!(info.location.country.flag.emoji, "🇺🇸");
    assert_eq!(info.currency.format.negative.prefix, "-$");
    assert!(!info.security.is_vpn);
    assert_eq!(info.time_zone.offset, -25200);
}

#[tokio::test]
async fn lookup_sends_default_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header(
            "User-Agent",
            format!("IpregistryClient/Rust/{}", env!("CARGO_PKG_VERSION")).as_str(),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .await
        .lookup(ip("8.8.8.8"))
        .await
        .unwrap();
}

#[tokio::test]
async fn lookup_options_map_to_query_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/8.8.8.8"))
        .and(query_param("hostname", "true"))
        .and(query_param("fields", "location,security"))
        .and(query_param("extra", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .await
        .lookup(ip("8.8.8.8"))
        .hostname(true)
        .fields("location,security")
        .param("extra", "1")
        .await
        .unwrap();
}

#[tokio::test]
async fn sparse_response_decodes_to_defaults() {
    // A `fields` selection omits everything else from the payload.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "location": {"country": {"name": "United States"}}
        })))
        .mount(&server)
        .await;

    let info = client_for(&server)
        .await
        .lookup(ip("8.8.8.8"))
        .fields("location.country.name")
        .await
        .unwrap();

    assert_eq!(info.location.country.name, "United States");
    assert_eq!(info.ip, None);
    assert_eq!(info.connection.asn, None);
    assert_eq!(info.location.city, "");
}

#[tokio::test]
async fn api_errors_are_typed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "code": "INVALID_API_KEY",
            "message": "the API key is invalid",
            "resolution": "check your key"
        })))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .await
        .lookup(ip("8.8.8.8"))
        .await
        .unwrap_err();

    let api = err.as_api().expect("expected an API error");
    assert_eq!(api.code, Some(ErrorCode::InvalidApiKey));
    assert_eq!(api.message, "the API key is invalid");
    assert_eq!(api.resolution, "check your key");
    assert_eq!(api.status, Some(403));
}

#[tokio::test]
async fn unrecognizable_error_bodies_fall_back_to_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404).set_body_string("<html>not json</html>"))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .await
        .lookup(ip("8.8.8.8"))
        .await
        .unwrap_err();

    let api = err.as_api().expect("expected an API error");
    assert_eq!(api.code, None);
    assert_eq!(api.message, "unexpected HTTP status 404");
}

#[tokio::test]
async fn server_errors_are_retried_by_default() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(1)
        .mount(&server)
        .await;

    let info = client_for(&server)
        .await
        .lookup(ip("8.8.8.8"))
        .await
        .unwrap();
    assert_eq!(info.ip, Some(ip("8.8.8.8")));
}

#[tokio::test]
async fn server_error_retries_can_be_disabled() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let err = Client::builder("test-key")
        .base_url(server.uri())
        .retry_on_server_error(false)
        .build()
        .unwrap()
        .lookup(ip("8.8.8.8"))
        .await
        .unwrap_err();

    assert_eq!(err.as_api().unwrap().status, Some(500));
}

#[tokio::test]
async fn retries_stop_after_max_retries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .expect(3) // initial attempt + 2 retries
        .mount(&server)
        .await;

    let err = Client::builder("test-key")
        .base_url(server.uri())
        .max_retries(2)
        .retry_interval(Duration::from_millis(1))
        .build()
        .unwrap()
        .lookup(ip("8.8.8.8"))
        .await
        .unwrap_err();

    assert_eq!(err.as_api().unwrap().status, Some(503));
}

#[tokio::test]
async fn too_many_requests_is_not_retried_by_default() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "code": "TOO_MANY_REQUESTS",
            "message": "rate limit exceeded",
            "resolution": "slow down"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client_for(&server)
        .await
        .lookup(ip("8.8.8.8"))
        .await
        .unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::TooManyRequests));
}

#[tokio::test]
async fn too_many_requests_retry_honors_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder("test-key")
        .base_url(server.uri())
        .retry_on_too_many_requests(true)
        .retry_interval(Duration::from_millis(1))
        .build()
        .unwrap();

    let start = Instant::now();
    let info = client.lookup(ip("8.8.8.8")).await.unwrap();
    assert_eq!(info.ip, Some(ip("8.8.8.8")));
    assert!(
        start.elapsed() >= Duration::from_secs(1),
        "Retry-After should take precedence over the 1ms retry interval"
    );
}

#[tokio::test]
async fn transport_errors_surface_after_retries() {
    // Nothing listens on this address, so every attempt fails at the
    // transport level.
    let err = Client::builder("test-key")
        .base_url("http://127.0.0.1:9")
        .max_retries(1)
        .retry_interval(Duration::from_millis(1))
        .build()
        .unwrap()
        .lookup(ip("8.8.8.8"))
        .await
        .unwrap_err();

    assert!(matches!(err, Error::Transport(_)), "got: {err:?}");
}

#[tokio::test]
async fn timeouts_surface_as_transport_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(sample_ip_info())
                .set_delay(Duration::from_secs(5)),
        )
        .mount(&server)
        .await;

    let err = Client::builder("test-key")
        .base_url(server.uri())
        .timeout(Duration::from_millis(50))
        .max_retries(0)
        .build()
        .unwrap()
        .lookup(ip("8.8.8.8"))
        .await
        .unwrap_err();

    assert!(matches!(err, Error::Transport(_)), "got: {err:?}");
}

#[tokio::test]
async fn cache_serves_repeated_lookups_locally() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/8.8.8.8"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder("test-key")
        .base_url(server.uri())
        .cache(InMemoryCache::new())
        .build()
        .unwrap();

    let first = client.lookup(ip("8.8.8.8")).await.unwrap();
    let second = client.lookup(ip("8.8.8.8")).await.unwrap();
    assert_eq!(first, second);
}

#[tokio::test]
async fn cache_keys_include_request_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/8.8.8.8"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(2) // one per distinct parameter set
        .mount(&server)
        .await;

    let client = Client::builder("test-key")
        .base_url(server.uri())
        .cache(InMemoryCache::new())
        .build()
        .unwrap();

    client.lookup(ip("8.8.8.8")).await.unwrap();
    client.lookup(ip("8.8.8.8")).hostname(true).await.unwrap();
    client.lookup(ip("8.8.8.8")).await.unwrap(); // cached
}

#[tokio::test]
async fn shared_cache_handle_can_invalidate() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(2)
        .mount(&server)
        .await;

    let cache = Arc::new(InMemoryCache::new());
    let client = Client::builder("test-key")
        .base_url(server.uri())
        .cache(Arc::clone(&cache))
        .build()
        .unwrap();

    client.lookup(ip("8.8.8.8")).await.unwrap();
    assert_eq!(cache.len(), 1);
    cache.invalidate_all();
    client.lookup(ip("8.8.8.8")).await.unwrap();
}

#[tokio::test]
async fn origin_lookup_hits_root_and_parses_user_agent() {
    let server = MockServer::start().await;
    let mut body = sample_ip_info();
    body["user_agent"] = json!({
        "header": "curl/8.0.1",
        "name": "curl",
        "type": "library",
        "version": "8.0.1",
        "version_major": "8",
        "device": {"brand": null, "name": null, "type": null},
        "engine": {"name": null, "type": null, "version": null, "version_major": null},
        "os": {"name": null, "type": null, "version": null}
    });
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(2) // origin lookups are never cached
        .mount(&server)
        .await;

    let client = Client::builder("test-key")
        .base_url(server.uri())
        .cache(InMemoryCache::new())
        .build()
        .unwrap();

    let origin = client.lookup_origin().await.unwrap();
    assert_eq!(origin.info.ip, Some(ip("8.8.8.8")));
    let ua = origin.user_agent.expect("user agent should be present");
    assert_eq!(ua.name, "curl");
    assert_eq!(ua.device.brand, "", "null device fields decode to defaults");

    client.lookup_origin().await.unwrap();
}

#[tokio::test]
async fn batch_lookup_preserves_order_and_per_entry_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(json!(["8.8.8.8", "192.168.0.1"])))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [
                sample_ip_info(),
                {
                    "code": "RESERVED_IP_ADDRESS",
                    "message": "the IP address is reserved",
                    "resolution": "use a public address"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let results = client_for(&server)
        .await
        .lookup_batch([ip("8.8.8.8"), ip("192.168.0.1")])
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].as_ref().unwrap().ip, Some(ip("8.8.8.8")));
    let err = results[1].as_ref().unwrap_err();
    assert_eq!(err.code, Some(ErrorCode::ReservedIpAddress));
}

/// Echoes a minimal successful result for every IP address in the request
/// body, so chunked batch requests can be verified end to end.
struct EchoBatch;

impl Respond for EchoBatch {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let ips: Vec<String> = serde_json::from_slice(&request.body).unwrap();
        assert!(ips.len() <= 2, "chunks should respect max_batch_size");
        let results: Vec<serde_json::Value> = ips.iter().map(|ip| json!({"ip": ip})).collect();
        ResponseTemplate::new(200).set_body_json(json!({ "results": results }))
    }
}

#[tokio::test]
async fn large_batches_are_chunked_and_reassembled_in_order() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(EchoBatch)
        .expect(3) // 5 addresses at 2 per chunk
        .mount(&server)
        .await;

    let ips: Vec<IpAddr> = (1..=5).map(|i| ip(&format!("1.1.1.{i}"))).collect();
    let results = Client::builder("test-key")
        .base_url(server.uri())
        .max_batch_size(2)
        .batch_concurrency(2)
        .build()
        .unwrap()
        .lookup_batch(ips.clone())
        .await
        .unwrap();

    assert_eq!(results.len(), 5);
    for (i, entry) in results.iter().enumerate() {
        assert_eq!(entry.as_ref().unwrap().ip, Some(ips[i]));
    }
}

#[tokio::test]
async fn batch_lookup_serves_cached_entries_and_requests_the_rest() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/8.8.8.8"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(1)
        .mount(&server)
        .await;
    // Only the cache miss goes into the batch request.
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(json!(["1.1.1.1"])))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"ip": "1.1.1.1"}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder("test-key")
        .base_url(server.uri())
        .cache(InMemoryCache::new())
        .build()
        .unwrap();

    client.lookup(ip("8.8.8.8")).await.unwrap();

    let results = client
        .lookup_batch([ip("8.8.8.8"), ip("1.1.1.1")])
        .await
        .unwrap();
    assert_eq!(results[0].as_ref().unwrap().ip, Some(ip("8.8.8.8")));
    assert_eq!(results[1].as_ref().unwrap().ip, Some(ip("1.1.1.1")));
}

#[tokio::test]
async fn batch_results_populate_the_cache() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [sample_ip_info()]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder("test-key")
        .base_url(server.uri())
        .cache(InMemoryCache::new())
        .build()
        .unwrap();

    client.lookup_batch([ip("8.8.8.8")]).await.unwrap();
    // Served from the cache: no GET mock is mounted, so a request would 404.
    let info = client.lookup(ip("8.8.8.8")).await.unwrap();
    assert_eq!(info.ip, Some(ip("8.8.8.8")));
}

#[tokio::test]
async fn whole_batch_failures_return_a_single_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "code": "INVALID_API_KEY",
            "message": "the API key is invalid",
            "resolution": "check your key"
        })))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .await
        .lookup_batch([ip("8.8.8.8"), ip("1.1.1.1")])
        .await
        .unwrap_err();

    assert_eq!(err.code(), Some(&ErrorCode::InvalidApiKey));
}

#[tokio::test]
async fn missing_batch_results_are_reported_per_entry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [sample_ip_info()]
        })))
        .mount(&server)
        .await;

    let results = client_for(&server)
        .await
        .lookup_batch([ip("8.8.8.8"), ip("1.1.1.1")])
        .await
        .unwrap();

    assert!(results[0].is_ok());
    let err = results[1].as_ref().unwrap_err();
    assert_eq!(err.message, "missing result for requested IP address");
}

#[tokio::test]
async fn parse_user_agents_posts_to_user_agent_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/user_agent"))
        .and(body_json(json!(["curl/8.0.1", "bogus"])))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [
                {
                    "header": "curl/8.0.1",
                    "name": "curl",
                    "type": "library",
                    "version": "8.0.1",
                    "version_major": "8",
                    "device": {"brand": null, "name": null, "type": null},
                    "engine": {"name": null, "type": null, "version": null, "version_major": null},
                    "os": {"name": null, "type": null, "version": null}
                },
                {
                    "code": "BAD_REQUEST",
                    "message": "unparsable user agent",
                    "resolution": "provide a valid User-Agent header"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let results = client_for(&server)
        .await
        .parse_user_agents(["curl/8.0.1", "bogus"])
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    let ua = results[0].as_ref().unwrap();
    assert_eq!(ua.name, "curl");
    assert_eq!(ua.agent_type, "library");
    assert_eq!(
        results[1].as_ref().unwrap_err().code,
        Some(ErrorCode::BadRequest)
    );
}

#[tokio::test]
async fn client_clones_share_the_cache_and_are_task_safe() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_ip_info()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder("test-key")
        .base_url(server.uri())
        .cache(InMemoryCache::new())
        .build()
        .unwrap();

    let clone = client.clone();
    tokio::spawn(async move { clone.lookup(ip("8.8.8.8")).await })
        .await
        .unwrap()
        .unwrap();

    // Served from the cache populated by the spawned clone.
    client.lookup(ip("8.8.8.8")).await.unwrap();
}
