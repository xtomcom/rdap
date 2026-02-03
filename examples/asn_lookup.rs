//! AS number lookup example

use rdap::{QueryType, RdapClient, RdapObject, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;

    // Query an AS number
    let asn = "AS15169";
    let request = RdapRequest::new(QueryType::Autnum, asn);
    let result = client.query(&request).await?;

    if let RdapObject::Autnum(autnum) = result {
        println!("=== AS Number Information ===\n");

        if let Some(start) = autnum.start_autnum {
            if let Some(end) = autnum.end_autnum {
                if start == end {
                    println!("AS Number: AS{}", start);
                } else {
                    println!("AS Range: AS{} - AS{}", start, end);
                }
            }
        }

        if let Some(name) = &autnum.name {
            println!("Name: {}", name);
        }

        if let Some(handle) = &autnum.handle {
            println!("Handle: {}", handle);
        }

        if let Some(as_type) = &autnum.as_type {
            println!("Type: {}", as_type);
        }

        if let Some(country) = &autnum.country {
            println!("Country: {}", country);
        }

        // Status
        if !autnum.status.is_empty() {
            println!("\nStatus:");
            for status in &autnum.status {
                println!("  - {}", status);
            }
        }

        // Events
        if !autnum.events.is_empty() {
            println!("\nEvents:");
            for event in &autnum.events {
                println!("  {}: {}", event.action, event.date);
            }
        }

        // Entities
        if !autnum.entities.is_empty() {
            println!("\nContacts:");
            for entity in &autnum.entities {
                if let Some(handle) = &entity.handle {
                    println!("\n  {} ({})", handle, entity.roles.join(", "));

                    if let Some(vcard) = &entity.vcard {
                        if let Some(name) = vcard.name() {
                            println!("    Name: {}", name);
                        }
                        if let Some(org) = vcard.org() {
                            println!("    Organization: {}", org);
                        }
                        if let Some(email) = vcard.email() {
                            println!("    Email: {}", email);
                        }
                        if let Some(tel) = vcard.tel() {
                            println!("    Phone: {}", tel);
                        }
                        if let Some(addr) = vcard.address() {
                            if let Some(label) = &addr.label {
                                println!("    Address: {}", label);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
