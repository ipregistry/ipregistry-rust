# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2026-11-08

### Added

- New field `Connection::is_anycast` (JSON `connection.is_anycast`): whether the IP address is announced via
  anycast, i.e. simultaneously from multiple locations.
- New field `Security::is_residential_proxy` (JSON `security.is_residential_proxy`): whether the IP address is
  a node of a residential proxy network.

### Changed

- Updated dependencies, including the security fixes in rustls, rustls-webpki, quinn-proto and h2, and lru 0.19.

## [1.0.0] - 2026-07-03

### Added

- Initial release of the official Rust client for the Ipregistry API.
- Single IP lookup (`Client::lookup`), origin lookup (`Client::lookup_origin`),
  and batch lookup (`Client::lookup_batch`) with per-entry results, transparent
  chunking beyond the API's 1024-address limit, and bounded concurrency.
- User-Agent parsing (`Client::parse_user_agents`) and the `is_bot` helper.
- Per-request options: reverse-DNS hostname resolution, field selection, and
  arbitrary query parameters.
- Pluggable response caching via the `Cache` trait, with a built-in
  thread-safe `InMemoryCache` (LRU + TTL).
- Automatic retries with exponential backoff for transient network errors and
  5xx responses; opt-in retries on 429 honoring `Retry-After`.
- Typed errors: `Error` (API / transport / decode), `ApiError`, and
  `ErrorCode`.
- TLS via rustls by default, with an optional `native-tls` feature.
