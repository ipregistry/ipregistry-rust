//! System tests running against the live Ipregistry API. They are marked
//! `#[ignore]` so the default `cargo test` run stays offline; run them with:
//!
//! ```sh
//! IPREGISTRY_API_KEY=YOUR_API_KEY cargo test --test integration -- --ignored
//! ```
//!
//! A valid API key is required; each successful lookup consumes credits.

use std::net::IpAddr;

use ipregistry::{Client, ErrorCode};

fn integration_client() -> Client {
    let key = std::env::var("IPREGISTRY_API_KEY")
        .expect("set IPREGISTRY_API_KEY to run integration tests");
    Client::new(key)
}

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

#[tokio::test]
#[ignore = "requires IPREGISTRY_API_KEY and consumes credits"]
async fn lookup() {
    let client = integration_client();
    let info = client.lookup(ip("8.8.8.8")).await.unwrap();

    assert_eq!(info.ip, Some(ip("8.8.8.8")));
    assert!(
        !info.location.country.code.is_empty(),
        "expected a non-empty country code"
    );
    assert!(
        info.connection.asn.is_some(),
        "expected an ASN for a well-known address"
    );
}

#[tokio::test]
#[ignore = "requires IPREGISTRY_API_KEY and consumes credits"]
async fn lookup_ipv6() {
    let client = integration_client();
    let info = client.lookup(ip("2001:4860:4860::8888")).await.unwrap();
    assert_eq!(info.ip, Some(ip("2001:4860:4860::8888")));
    assert_eq!(info.ip_type, Some(ipregistry::model::IpType::V6));
}

#[tokio::test]
#[ignore = "requires IPREGISTRY_API_KEY and consumes credits"]
async fn lookup_with_options() {
    let client = integration_client();
    let info = client
        .lookup(ip("8.8.8.8"))
        .fields("location.country.name")
        .await
        .unwrap();

    assert!(!info.location.country.name.is_empty());
    assert_eq!(info.ip, None, "unselected fields should be absent");
}

#[tokio::test]
#[ignore = "requires IPREGISTRY_API_KEY and consumes credits"]
async fn lookup_origin() {
    let client = integration_client();
    let origin = client.lookup_origin().await.unwrap();
    assert!(origin.info.ip.is_some(), "expected a non-empty origin IP");
}

#[tokio::test]
#[ignore = "requires IPREGISTRY_API_KEY and consumes credits"]
async fn lookup_batch() {
    let client = integration_client();
    // The private address surfaces as a per-entry error.
    let ips = [ip("8.8.8.8"), ip("1.1.1.1"), ip("192.168.0.1")];
    let results = client.lookup_batch(ips).await.unwrap();

    assert_eq!(results.len(), ips.len());
    assert!(
        results[0].is_ok(),
        "entry 0 should succeed: {:?}",
        results[0]
    );
    assert!(
        results[1].is_ok(),
        "entry 1 should succeed: {:?}",
        results[1]
    );
    let err = results[2]
        .as_ref()
        .expect_err("entry 2 (reserved IP) should fail");
    assert_eq!(err.code, Some(ErrorCode::ReservedIpAddress));
}

#[tokio::test]
#[ignore = "requires IPREGISTRY_API_KEY and consumes credits"]
async fn parse_user_agents() {
    let client = integration_client();
    let results = client
        .parse_user_agents([
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/120.0 Safari/537.36",
        ])
        .await
        .unwrap();

    let ua = results[0].as_ref().unwrap();
    assert!(!ua.name.is_empty(), "expected a non-empty user-agent name");
}
