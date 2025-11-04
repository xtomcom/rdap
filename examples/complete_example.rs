//! Complete example showing various RDAP library features
//!
//! This example demonstrates:
//! - Creating a client
//! - Auto-detecting query types
//! - Handling different response types
//! - Error handling
//! - Extracting specific information

use rdap::{RdapClient, RdapRequest, RdapObject, RdapError};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RDAP Library Complete Example ===\n");
    
    // Create a client with custom timeout
    let client = RdapClient::new()?
        .with_timeout(Duration::from_secs(30));
    
    // Example 1: Domain query with detailed information extraction
    println!("1. Domain Query:");
    query_domain(&client, "example.com").await?;
    
    println!("\n{}\n", "=".repeat(60));
    
    // Example 2: IP address query
    println!("2. IP Address Query:");
    query_ip(&client, "8.8.8.8").await?;
    
    println!("\n{}\n", "=".repeat(60));
    
    // Example 3: AS number query
    println!("3. AS Number Query:");
    query_asn(&client, "AS15169").await?;
    
    println!("\n{}\n", "=".repeat(60));
    
    // Example 4: Error handling
    println!("4. Error Handling:");
    demonstrate_error_handling(&client).await;
    
    Ok(())
}

async fn query_domain(client: &RdapClient, domain: &str) -> Result<(), Box<dyn std::error::Error>> {
    let query_type = RdapRequest::detect_type(domain)?;
    let request = RdapRequest::new(query_type, domain);
    let result = client.query(&request).await?;
    
    if let RdapObject::Domain(domain_obj) = result {
        // Extract basic information
        if let Some(name) = &domain_obj.ldh_name {
            println!("  Domain: {}", name);
        }
        
        // Status information
        if !domain_obj.status.is_empty() {
            println!("  Status: {}", domain_obj.status.join(", "));
        }
        
        // Nameservers
        if !domain_obj.nameservers.is_empty() {
            println!("  Nameservers:");
            for ns in &domain_obj.nameservers {
                if let Some(name) = &ns.ldh_name {
                    println!("    - {}", name);
                }
            }
        }
        
        // DNSSEC status
        if let Some(dnssec) = &domain_obj.secure_dns {
            if let Some(signed) = dnssec.delegation_signed {
                println!("  DNSSEC: {}", if signed { "Enabled" } else { "Disabled" });
            }
        }
        
        // Important dates
        for event in &domain_obj.events {
            match event.action.as_str() {
                "registration" => println!("  Registered: {}", event.date),
                "expiration" => println!("  Expires: {}", event.date),
                _ => {}
            }
        }
    }
    
    Ok(())
}

async fn query_ip(client: &RdapClient, ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    let query_type = RdapRequest::detect_type(ip)?;
    let request = RdapRequest::new(query_type, ip);
    let result = client.query(&request).await?;
    
    if let RdapObject::IpNetwork(network) = result {
        if let Some(name) = &network.name {
            println!("  Network: {}", name);
        }
        
        if let (Some(start), Some(end)) = (&network.start_address, &network.end_address) {
            println!("  Range: {} - {}", start, end);
        }
        
        if let Some(country) = &network.country {
            println!("  Country: {}", country);
        }
        
        // Find registrant
        for entity in &network.entities {
            if entity.roles.contains(&"registrant".to_string()) {
                if let Some(vcard) = &entity.vcard {
                    if let Some(org) = vcard.name() {
                        println!("  Organization: {}", org);
                    }
                }
                break;
            }
        }
    }
    
    Ok(())
}

async fn query_asn(client: &RdapClient, asn: &str) -> Result<(), Box<dyn std::error::Error>> {
    let query_type = RdapRequest::detect_type(asn)?;
    let request = RdapRequest::new(query_type, asn);
    let result = client.query(&request).await?;
    
    if let RdapObject::Autnum(autnum) = result {
        if let Some(start) = autnum.start_autnum {
            println!("  AS Number: AS{}", start);
        }
        
        if let Some(name) = &autnum.name {
            println!("  Name: {}", name);
        }
        
        if let Some(country) = &autnum.country {
            println!("  Country: {}", country);
        }
        
        // Count entities by role
        let mut role_counts = std::collections::HashMap::new();
        for entity in &autnum.entities {
            for role in &entity.roles {
                *role_counts.entry(role.clone()).or_insert(0) += 1;
            }
        }
        
        if !role_counts.is_empty() {
            println!("  Contacts:");
            for (role, count) in role_counts {
                println!("    - {}: {}", role, count);
            }
        }
    }
    
    Ok(())
}

async fn demonstrate_error_handling(client: &RdapClient) {
    let test_queries = vec![
        "nonexistent-domain-xyz123.com",
        "999.999.999.999",
    ];
    
    for query in test_queries {
        println!("  Testing: {}", query);
        
        let query_type = match RdapRequest::detect_type(query) {
            Ok(qt) => qt,
            Err(e) => {
                println!("    ❌ Invalid query: {}", e);
                continue;
            }
        };
        
        let request = RdapRequest::new(query_type, query);
        
        match client.query(&request).await {
            Ok(result) => {
                match result {
                    RdapObject::Error(err) => {
                        println!("    ⚠️  RDAP Error:");
                        if let Some(title) = &err.title {
                            println!("       {}", title);
                        }
                    }
                    _ => {
                        println!("    ✅ Query successful");
                    }
                }
            }
            Err(e) => {
                match e {
                    RdapError::Bootstrap(msg) => {
                        println!("    ❌ Bootstrap error: {}", msg);
                    }
                    RdapError::NotFound => {
                        println!("    ℹ️  Object not found");
                    }
                    _ => {
                        println!("    ❌ Error: {}", e);
                    }
                }
            }
        }
    }
}
