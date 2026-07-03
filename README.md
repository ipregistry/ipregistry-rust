[<img src="https://cdn.ipregistry.co/icons/favicon-96x96.png" alt="Ipregistry" width="64"/>](https://ipregistry.co/)
# Ipregistry Rust Client Library

[![License](http://img.shields.io/:license-apache-blue.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/ipregistry.svg)](https://crates.io/crates/ipregistry)
[![docs.rs](https://img.shields.io/docsrs/ipregistry)](https://docs.rs/ipregistry)
[![CI](https://github.com/ipregistry/ipregistry-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/ipregistry/ipregistry-rust/actions/workflows/ci.yml)

This is the official Rust client library for the [Ipregistry](https://ipregistry.co) IP geolocation and threat data
API, allowing you to look up your own IP address or specified ones. Responses return multiple data points including
carrier, company, currency, location, time zone, threat information, and more. The library can also parse raw
User-Agent strings.

## Getting Started

You'll need an Ipregistry API key, which you can get along with 100,000 free lookups by signing up for a free account
at [https://ipregistry.co](https://ipregistry.co).

### Installation

```bash
cargo add ipregistry
```

The client is asynchronous and runs on the [Tokio](https://tokio.rs) runtime. TLS uses
[rustls](https://github.com/rustls/rustls) by default; switch to the platform-native TLS library with
`cargo add ipregistry --no-default-features -F native-tls`.

### Quick start

#### Single IP lookup

```rust
use std::net::IpAddr;
use ipregistry::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("YOUR_API_KEY");

    // Look up data for a given IPv4 or IPv6 address. On the server side,
    // parse the client IP from the request headers.
    let ip: IpAddr = "54.85.132.205".parse()?;
    let info = client.lookup(ip).await?;
    println!("{}", info.location.country.name);

    Ok(())
}
```

IP addresses are passed as [`std::net::IpAddr`](https://doc.rust-lang.org/std/net/enum.IpAddr.html) values, so an
invalid address is caught by `.parse()` at the boundary of your program instead of costing an API call.

#### Origin IP lookup

To look up the IP address the request is sent from — no argument needed — use `lookup_origin`. It returns a
`RequesterIpInfo`, which additionally carries parsed User-Agent data:

```rust
let origin = client.lookup_origin().await?;
println!("{:?} {}", origin.info.ip, origin.info.location.country.name);
```

#### Batch IP lookup

`lookup_batch` resolves many IP addresses in a single request. Each entry independently succeeds or fails (for example
on a reserved address), so entries are plain `Result` values:

```rust
let ips: Vec<IpAddr> = ["73.2.2.2", "8.8.8.8", "2001:67c:2e8:22::c100:68b"]
    .iter()
    .map(|s| s.parse().unwrap())
    .collect();

for entry in client.lookup_batch(ips).await? {
    match entry {
        Ok(info) => println!("{}", info.location.country.name),
        Err(err) => eprintln!("entry failed: {err}"),
    }
}
```

The Ipregistry API accepts up to 1024 IP addresses per request. `lookup_batch` transparently splits larger inputs into
several requests, dispatched with bounded concurrency, and reassembles the results in input order — so you can pass an
arbitrarily long list without hitting `TOO_MANY_IPS`. Tune the behavior when needed:

```rust
let client = Client::builder("YOUR_API_KEY")
    .max_batch_size(1024)     // addresses per request (max/default: 1024)
    .batch_concurrency(4)     // concurrent sub-requests (default: 4; 1 = sequential)
    .build()?;
```

Only cache misses are sent to the API; if a whole sub-request fails (network or API error), `lookup_batch` returns that
error, whereas an individual bad address surfaces as a per-entry error as shown above.

## Options

Lookup requests are refined by chaining methods before awaiting them; each maps to an Ipregistry query parameter:

```rust
let info = client
    .lookup(ip)
    .hostname(true)                            // resolve reverse-DNS hostname
    .fields("location.country.name,security")  // select only these fields
    .await?;
```

| Method                  | Description                                                                                                                 |
|-------------------------|-----------------------------------------------------------------------------------------------------------------------------|
| `.hostname(bool)`       | Enable reverse-DNS hostname resolution (disabled by default).                                                              |
| `.fields(expression)`   | Restrict the response to the given [fields](https://ipregistry.co/docs/filtering-selecting-fields), reducing payload size. |
| `.param(name, value)`   | Set an arbitrary query parameter not covered by a dedicated method.                                                         |

## Caching

Although the client has built-in support for in-memory caching, it is **disabled by default** to ensure data freshness.

To enable caching, pass an `InMemoryCache` when building the client:

```rust
use ipregistry::{Client, InMemoryCache};

let client = Client::builder("YOUR_API_KEY")
    .cache(InMemoryCache::new())
    .build()?;
```

The in-memory cache is thread-safe and supports size- and time-based eviction (LRU with a TTL):

```rust
use std::time::Duration;

let cache = InMemoryCache::builder()
    .max_size(8192)                    // maximum number of entries (default 4096)
    .ttl(Duration::from_secs(600))     // entry lifetime (default 10 minutes)
    .build();

let client = Client::builder("YOUR_API_KEY").cache(cache).build()?;
```

Origin (requester) lookups are never cached, because the requester IP is only known from the response. Batch lookups
transparently serve already-cached entries and only request the remainder from the API.

You can provide your own cache implementation by satisfying the `Cache` trait:

```rust
pub trait Cache: Send + Sync {
    fn get(&self, key: &str) -> Option<IpInfo>;
    fn set(&self, key: &str, value: IpInfo);
    fn invalidate(&self, key: &str);
    fn invalidate_all(&self);
}
```

`Cache` is also implemented for `Arc<C>`, so you can keep a handle to the cache (for example to invalidate entries)
after handing it to a client.

## Retries

Failed requests are automatically retried with an exponential backoff. By default, up to 3 retries are performed on
transient network errors and 5xx server responses.

Because Ipregistry does not rate limit by default (rate limiting is opt-in per API key), retries on
_429 Too Many Requests_ responses are **disabled by default**. Enable them if your API key is configured with a rate
limit and you want the client to wait and retry (honoring the `Retry-After` header when present):

```rust
use std::time::Duration;

let client = Client::builder("YOUR_API_KEY")
    .max_retries(3)                            // 0 disables retries entirely
    .retry_interval(Duration::from_secs(1))    // base backoff (interval * 2^attempt)
    .retry_on_server_error(true)               // retry on 5xx (default: true)
    .retry_on_too_many_requests(true)          // retry on 429 (default: false)
    .build()?;
```

## Timeouts and concurrency

By default the client uses a 15-second per-request timeout. Adjust it with `.timeout(...)`, or supply your own
[`reqwest::Client`](https://docs.rs/reqwest) for full control over connection pooling, proxying, TLS, or
instrumentation:

```rust
let http = ipregistry::reqwest::Client::builder()
    /* custom transport, proxy, TLS, timeout, ... */
    .build()?;

let client = Client::builder("YOUR_API_KEY").http_client(http).build()?;
```

When you supply your own HTTP client, its own timeout settings apply and `.timeout(...)` is ignored.

A `Client` is cheap to clone (clones share the same connection pool and cache) and safe to share across threads and
tasks, so spawn concurrent lookups freely — there is no separate blocking API. As usual with futures, dropping a
lookup future cancels the request; to bound an individual call, combine it with
[`tokio::time::timeout`](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html).

## Errors

All operations return `ipregistry::Error`, which separates failures reported by the API from client-side failures:

- **`Error::Api(ApiError)`** — the API reported a failure (e.g. insufficient credits, throttling, invalid input). It
  carries a typed `ErrorCode` (unrecognized codes are preserved in `ErrorCode::Other`), a `message`, a `resolution`,
  and the HTTP `status`.
- **`Error::Transport(reqwest::Error)`** — a network error, timeout, or TLS failure.
- **`Error::Decode(serde_json::Error)`** — a successful response could not be decoded.

```rust
use ipregistry::{Error, ErrorCode};

match client.lookup(ip).await {
    Ok(info) => { /* use info */ }
    Err(Error::Api(err)) if err.code == Some(ErrorCode::InsufficientCredits) => {
        // handle exhausted credits
    }
    Err(Error::Api(err)) if err.code == Some(ErrorCode::TooManyRequests) => {
        // handle rate limiting
    }
    Err(Error::Transport(err)) => {
        // handle network / timeout error
    }
    Err(err) => eprintln!("{err}"),
}
```

The full list of error codes is documented at [ipregistry.co/docs/errors](https://ipregistry.co/docs/errors).

## Parsing User-Agents

Parse one or more raw User-Agent strings (such as the `User-Agent` header of an incoming request) into structured data:

```rust
let results = client
    .parse_user_agents(["Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Chrome/120.0"])
    .await?;

let ua = results[0].as_ref().expect("entry should parse");
println!("{} on {}", ua.name, ua.os.name);
```

## Filtering bots

You might want to prevent Ipregistry API calls for crawlers or bots browsing your pages. To help identify bots from the
User-Agent, the library includes a lightweight helper:

```rust
// For testing you can retrieve your current User-Agent from:
// https://api.ipregistry.co/user_agent?key=YOUR_API_KEY (look at the "user_agent" field)
if !ipregistry::is_bot(user_agent_from_request_header) {
    let info = client.lookup(client_ip).await?;
    // ...
}
```

## Examples

Runnable examples live in the [`examples/`](examples) directory. Set your key and run one:

```bash
IPREGISTRY_API_KEY=YOUR_API_KEY cargo run --example single
```

## Testing

The library ships with two tiers of tests:

- **Unit / behavior tests** run offline against an in-process mock server — no API key or network is required. This is
  the default `cargo test` and what CI runs.
- **System tests** exercise the live Ipregistry API. They are marked `#[ignore]` and are skipped unless explicitly
  requested with a valid `IPREGISTRY_API_KEY` (each successful lookup consumes credits):

  ```bash
  IPREGISTRY_API_KEY=YOUR_API_KEY cargo test --test integration -- --ignored
  ```

## Other Libraries

There are official Ipregistry client libraries available for many languages including
[Java](https://github.com/ipregistry/ipregistry-java),
[Javascript](https://github.com/ipregistry/ipregistry-javascript),
[Go](https://github.com/ipregistry/ipregistry-go),
[Python](https://github.com/ipregistry/ipregistry-python),
[Typescript](https://github.com/ipregistry/ipregistry-javascript) and more.

Are you looking for an official client with a programming language or framework we do not support yet?
[Let us know](mailto:support@ipregistry.co).

## License

This library is released under the [Apache 2.0 license](LICENSE).
