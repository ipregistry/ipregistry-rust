// Copyright 2019 Ipregistry (https://ipregistry.co).
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Official Rust client for the [Ipregistry](https://ipregistry.co) IP
//! geolocation and threat data API.
//!
//! Look up your own IP address or specified ones. Responses return multiple
//! data points including carrier, company, currency, location, time zone,
//! threat information, and more. The library can also parse raw User-Agent
//! strings.
//!
//! You'll need an Ipregistry API key, which you can get along with 100,000
//! free lookups by signing up for a free account at
//! <https://ipregistry.co>.
//!
//! # Quick start
//!
//! ```no_run
//! use std::net::IpAddr;
//! use ipregistry::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::new("YOUR_API_KEY");
//!
//!     // Look up data for a given IPv4 or IPv6 address. On the server side,
//!     // parse the client IP from the request headers.
//!     let ip: IpAddr = "54.85.132.205".parse()?;
//!     let info = client.lookup(ip).await?;
//!     println!("{}", info.location.country.name);
//!
//!     // Look up the IP address the request originates from.
//!     let origin = client.lookup_origin().await?;
//!     println!("{:?} {}", origin.info.ip, origin.info.location.country.name);
//!
//!     Ok(())
//! }
//! ```
//!
//! Lookups are refined by chaining methods on the request before awaiting it:
//!
//! ```no_run
//! # async fn example(client: &ipregistry::Client, ip: std::net::IpAddr)
//! # -> Result<(), ipregistry::Error> {
//! let info = client
//!     .lookup(ip)
//!     .hostname(true)                            // resolve reverse-DNS hostname
//!     .fields("location.country.name,security")  // select only these fields
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Batch lookups
//!
//! [`Client::lookup_batch`] resolves many IP addresses in a single request.
//! Each entry independently succeeds or fails (for example on a reserved
//! address), so entries are plain `Result` values:
//!
//! ```no_run
//! # async fn example(client: &ipregistry::Client) -> Result<(), ipregistry::Error> {
//! use std::net::IpAddr;
//!
//! let ips: Vec<IpAddr> = ["73.2.2.2", "8.8.8.8", "2001:67c:2e8:22::c100:68b"]
//!     .iter()
//!     .map(|s| s.parse().unwrap())
//!     .collect();
//!
//! for entry in client.lookup_batch(ips).await? {
//!     match entry {
//!         Ok(info) => println!("{}", info.location.country.name),
//!         Err(err) => eprintln!("entry failed: {err}"),
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! The Ipregistry API accepts up to [`MAX_BATCH_SIZE`] IP addresses per
//! request; larger inputs are transparently split into several requests
//! dispatched with bounded concurrency and reassembled in input order.
//!
//! # Caching
//!
//! Caching is disabled by default to ensure data freshness. Enable it by
//! passing an [`InMemoryCache`] — thread-safe, with LRU and TTL eviction — or
//! any [`Cache`] implementation of your own:
//!
//! ```no_run
//! use std::time::Duration;
//! use ipregistry::{Client, InMemoryCache};
//!
//! let cache = InMemoryCache::builder()
//!     .max_size(8192)
//!     .ttl(Duration::from_secs(600))
//!     .build();
//!
//! let client = Client::builder("YOUR_API_KEY").cache(cache).build().unwrap();
//! ```
//!
//! # Errors
//!
//! All operations return [`Error`], which separates failures reported by the
//! API ([`Error::Api`], carrying a typed [`ErrorCode`]) from client-side
//! failures such as network errors ([`Error::Transport`]). See the [`Error`]
//! documentation for a matching example.
//!
//! # Runtime
//!
//! The client is asynchronous and runs on the [Tokio](https://tokio.rs)
//! runtime. A [`Client`] is cheap to clone and safe to share across tasks;
//! spawn concurrent lookups freely.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

mod cache;
mod client;
mod error;
pub mod model;
mod request;

pub use cache::{
    Cache, DEFAULT_CACHE_MAX_SIZE, DEFAULT_CACHE_TTL, InMemoryCache, InMemoryCacheBuilder,
};
pub use client::{
    Client, ClientBuilder, DEFAULT_BASE_URL, DEFAULT_BATCH_CONCURRENCY, DEFAULT_MAX_RETRIES,
    DEFAULT_RETRY_INTERVAL, DEFAULT_TIMEOUT, MAX_BATCH_SIZE,
};
pub use error::{ApiError, Error, ErrorCode, Result};
pub use model::{IpInfo, RequesterIpInfo, UserAgent, is_bot};
pub use request::{LookupBatchRequest, LookupOriginRequest, LookupRequest};

// The HTTP client type is part of the public API (see
// `ClientBuilder::http_client`); re-export it so downstream crates do not
// need to depend on a matching reqwest version themselves.
pub use reqwest;
