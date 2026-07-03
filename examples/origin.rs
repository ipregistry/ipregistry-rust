//! Looks up the IP address the request originates from, along with the parsed
//! User-Agent of the calling client.
//!
//! ```sh
//! IPREGISTRY_API_KEY=YOUR_API_KEY cargo run --example origin
//! ```

use ipregistry::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("IPREGISTRY_API_KEY")
        .expect("set the IPREGISTRY_API_KEY environment variable");
    let client = Client::new(api_key);

    let origin = client.lookup_origin().await?;

    println!(
        "your IP is {} ({}, {})",
        origin
            .info
            .ip
            .map_or_else(|| "?".into(), |ip| ip.to_string()),
        origin.info.location.city,
        origin.info.location.country.name,
    );
    if let Some(ua) = origin.user_agent {
        println!("your user agent is {} on {}", ua.name, ua.os.name);
    }

    Ok(())
}
