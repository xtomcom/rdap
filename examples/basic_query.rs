//! Example: Basic domain query

use rdap::{QueryType, RdapClient, RdapRequest, display::RdapDisplay};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Create a client
    let client = RdapClient::new()?;

    // Create a domain query
    let request = RdapRequest::new(QueryType::Domain, "example.com");

    // Execute the query
    println!("Querying example.com...\n");
    let result = client.query(&request).await?;

    // Display the result with colors
    result.display(true);

    Ok(())
}
