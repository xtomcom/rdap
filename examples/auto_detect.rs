//! Auto-detect query type example

use rdap::{RdapClient, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    
    // Query different types - type is auto-detected
    let queries = vec![
        "example.com",     // Domain
        "8.8.8.8",        // IPv4
        "2001:4860:4860::8888", // IPv6
        "AS15169",        // AS number
    ];
    
    for query in queries {
        println!("\n=== Querying: {} ===", query);
        
        // Auto-detect query type
        let query_type = RdapRequest::detect_type(query)?;
        println!("Detected type: {:?}", query_type);
        
        let request = RdapRequest::new(query_type, query);
        let result = client.query(&request).await?;
        
        // Display result
        use rdap::display::RdapDisplay;
        result.display(false);
    }
    
    Ok(())
}
