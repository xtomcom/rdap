//! Example: JSON output

use rdap::{RdapClient, RdapRequest, QueryType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    let request = RdapRequest::new(QueryType::Domain, "example.com");

    let result = client.query(&request).await?;

    // Output as pretty JSON
    let json = serde_json::to_string_pretty(&result)?;
    println!("{}", json);

    Ok(())
}
