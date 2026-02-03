//! Example: Query with custom server

use rdap::{QueryType, RdapClient, RdapRequest, display::RdapDisplay};
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let client = RdapClient::new()?;

    // Query with a specific RDAP server
    let server = Url::parse("https://rdap.nic.cz")?;
    let request = RdapRequest::new(QueryType::Domain, "nic.cz").with_server(server);

    println!("Querying nic.cz via rdap.nic.cz...\n");
    let result = client.query(&request).await?;

    result.display(true);

    Ok(())
}
