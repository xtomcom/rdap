//! IP address lookup example

use rdap::{QueryType, RdapClient, RdapObject, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;

    // Query an IP address
    let ip = "8.8.8.8";
    let request = RdapRequest::new(QueryType::Ip, ip);
    let result = client.query(&request).await?;

    if let RdapObject::IpNetwork(network) = result {
        println!("=== IP Network Information ===\n");

        if let Some(name) = &network.name {
            println!("Network Name: {}", name);
        }

        if let Some(handle) = &network.handle {
            println!("Handle: {}", handle);
        }

        if let (Some(start), Some(end)) = (&network.start_address, &network.end_address) {
            println!("Address Range: {} - {}", start, end);
        }

        if let Some(version) = &network.ip_version {
            println!("IP Version: IPv{}", version);
        }

        if let Some(net_type) = &network.network_type {
            println!("Network Type: {}", net_type);
        }

        if let Some(country) = &network.country {
            println!("Country: {}", country);
        }

        if let Some(parent) = &network.parent_handle {
            println!("Parent Network: {}", parent);
        }

        // Status
        if !network.status.is_empty() {
            println!("\nStatus:");
            for status in &network.status {
                println!("  - {}", status);
            }
        }

        // Entities
        if !network.entities.is_empty() {
            println!("\nContacts:");
            for entity in &network.entities {
                if let Some(handle) = &entity.handle {
                    println!("  - {} ({})", handle, entity.roles.join(", "));

                    if let Some(vcard) = &entity.vcard {
                        if let Some(name) = vcard.name() {
                            println!("    Name: {}", name);
                        }
                        if let Some(org) = vcard.org() {
                            println!("    Organization: {}", org);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
