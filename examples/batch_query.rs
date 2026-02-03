//! Batch query example - query multiple domains/IPs

use rdap::{RdapClient, RdapRequest};
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;

    // List of domains to query
    let queries = vec!["example.com", "google.com", "github.com", "rust-lang.org"];

    println!("Querying {} domains...\n", queries.len());

    for query in queries {
        println!("=== {} ===", query);

        // Auto-detect type
        let query_type = RdapRequest::detect_type(query)?;
        let request = RdapRequest::new(query_type, query);

        match client.query(&request).await {
            Ok(result) => {
                use rdap::display::RdapDisplay;
                result.display(false);
            }
            Err(e) => {
                eprintln!("Error querying {}: {}", query, e);
            }
        }

        println!();

        // Be nice to the server - add a small delay
        sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}
