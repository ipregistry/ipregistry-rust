//! Looks up a single IP address.
//!
//! ```sh
//! IPREGISTRY_API_KEY=YOUR_API_KEY cargo run --example single
//! ```

use std::net::IpAddr;

use ipregistry::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("IPREGISTRY_API_KEY")
        .expect("set the IPREGISTRY_API_KEY environment variable");
    let client = Client::new(api_key);

    let ip: IpAddr = "54.85.132.205".parse()?;
    let info = client.lookup(ip).await?;

    println!(
        "{} is in {}, {} ({})",
        ip, info.location.city, info.location.country.name, info.location.country.flag.emoji
    );
    println!(
        "operated by {} (AS{})",
        info.connection.organization,
        info.connection
            .asn
            .map_or_else(|| "?".into(), |asn| asn.to_string()),
    );
    println!("anonymous: {}", info.security.is_anonymous);

    Ok(())
}
