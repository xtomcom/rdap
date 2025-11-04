//! Error handling example

use rdap::{RdapClient, RdapRequest, QueryType, RdapObject, RdapError};

#[tokio::main]
async fn main() {
    let client = RdapClient::new().unwrap();
    
    // Try queries that might fail
    let queries = vec![
        ("nonexistent-domain-12345.com", QueryType::Domain),
        ("999.999.999.999", QueryType::Ip),
        ("AS99999999", QueryType::Autnum),
    ];
    
    for (query, query_type) in queries {
        println!("\n=== Querying: {} ===", query);
        
        let request = RdapRequest::new(query_type, query);
        
        match client.query(&request).await {
            Ok(result) => {
                // Check if it's an error response from the RDAP server
                match result {
                    RdapObject::Error(err) => {
                        println!("❌ RDAP Error Response:");
                        if let Some(code) = err.error_code {
                            println!("   Code: {}", code);
                        }
                        if let Some(title) = &err.title {
                            println!("   Title: {}", title);
                        }
                        for desc in &err.description {
                            println!("   Description: {}", desc);
                        }
                    }
                    _ => {
                        println!("✅ Query successful");
                        use rdap::display::RdapDisplay;
                        result.display(false);
                    }
                }
            }
            Err(e) => {
                println!("❌ Client Error:");
                match e {
                    RdapError::Bootstrap(msg) => {
                        println!("   Bootstrap error: {}", msg);
                        println!("   (Try specifying a server with -s option)");
                    }
                    RdapError::Http(err) => {
                        println!("   HTTP error: {}", err);
                    }
                    RdapError::InvalidQuery(msg) => {
                        println!("   Invalid query: {}", msg);
                    }
                    RdapError::Json(err) => {
                        println!("   JSON parse error: {}", err);
                    }
                    RdapError::InvalidUrl(err) => {
                        println!("   URL error: {}", err);
                    }
                    RdapError::Io(err) => {
                        println!("   I/O error: {}", err);
                    }
                    _ => {
                        println!("   Other error: {}", e);
                    }
                }
            }
        }
    }
}
