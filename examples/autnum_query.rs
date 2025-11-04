//! Example: AS number query

use rdap::{display::RdapDisplay, RdapClient, RdapRequest, QueryType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = RdapClient::new()?;

    // Query with "AS" prefix
    println!("Querying AS15169 (Google)...\n");
    let request = RdapRequest::new(QueryType::Autnum, "AS15169");
    let result = client.query(&request).await?;
    result.display(true);

    Ok(())
}
