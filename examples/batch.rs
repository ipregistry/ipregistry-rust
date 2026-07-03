//! Looks up several IP addresses in a single request. Each entry
//! independently succeeds or fails.
//!
//! ```sh
//! IPREGISTRY_API_KEY=YOUR_API_KEY cargo run --example batch
//! ```

use std::net::IpAddr;

use ipregistry::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("IPREGISTRY_API_KEY")
        .expect("set the IPREGISTRY_API_KEY environment variable");
    let client = Client::new(api_key);

    let ips: Vec<IpAddr> = ["73.2.2.2", "8.8.8.8", "2001:67c:2e8:22::c100:68b"]
        .iter()
        .map(|s| s.parse())
        .collect::<Result<_, _>>()?;

    for (ip, entry) in ips.iter().zip(client.lookup_batch(ips.clone()).await?) {
        match entry {
            Ok(info) => println!("{ip}: {}", info.location.country.name),
            Err(err) => println!("{ip}: lookup failed: {err}"),
        }
    }

    Ok(())
}
