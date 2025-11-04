//! Example: IP address query

use rdap::{display::RdapDisplay, RdapClient, RdapRequest, QueryType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = RdapClient::new()?;

    // Query IPv4 address
    println!("=== IPv4 Query ===\n");
    let request = RdapRequest::new(QueryType::Ip, "8.8.8.8");
    let result = client.query(&request).await?;
    result.display(true);

    // Query IPv6 address
    println!("\n=== IPv6 Query ===\n");
    let request = RdapRequest::new(QueryType::Ip, "2001:4860:4860::8888");
    let result = client.query(&request).await?;
    result.display(true);

    Ok(())
}
