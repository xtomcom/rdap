//! Extract specific domain information

use rdap::{QueryType, RdapClient, RdapObject, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    let request = RdapRequest::new(QueryType::Domain, "example.com");
    let result = client.query(&request).await?;

    if let RdapObject::Domain(domain) = result {
        println!("=== Domain Information ===\n");

        // Basic info
        if let Some(name) = &domain.ldh_name {
            println!("Domain: {}", name);
        }

        if let Some(handle) = &domain.handle {
            println!("Handle: {}", handle);
        }

        // Status
        println!("\nStatus:");
        for status in &domain.status {
            println!("  - {}", status);
        }

        // Nameservers
        println!("\nNameservers:");
        for ns in &domain.nameservers {
            if let Some(name) = &ns.ldh_name {
                print!("  - {}", name);
                if let Some(ips) = &ns.ip_addresses {
                    let addrs: Vec<String> = ips.v4.iter().chain(&ips.v6).cloned().collect();
                    if !addrs.is_empty() {
                        print!(" ({})", addrs.join(", "));
                    }
                }
                println!();
            }
        }

        // DNSSEC
        if let Some(dnssec) = &domain.secure_dns {
            println!("\nDNSSEC:");
            if let Some(signed) = dnssec.delegation_signed {
                println!("  Delegation Signed: {}", if signed { "Yes" } else { "No" });
            }
            if let Some(signed) = dnssec.zone_signed {
                println!("  Zone Signed: {}", if signed { "Yes" } else { "No" });
            }

            // DS records
            if !dnssec.ds_data.is_empty() {
                println!("\n  DS Records:");
                for ds in &dnssec.ds_data {
                    if let (Some(tag), Some(alg), Some(digest_type)) =
                        (ds.key_tag, ds.algorithm, ds.digest_type)
                    {
                        println!(
                            "    - Key Tag: {}, Algorithm: {}, Digest Type: {}",
                            tag, alg, digest_type
                        );
                        if let Some(digest) = &ds.digest {
                            println!("      Digest: {}", digest);
                        }
                    }
                }
            }
        }

        // Events
        println!("\nImportant Dates:");
        for event in &domain.events {
            match event.action.as_str() {
                "registration" => println!("  Registered: {}", event.date),
                "expiration" => println!("  Expires: {}", event.date),
                "last changed" => println!("  Last Updated: {}", event.date),
                _ => {}
            }
        }

        // Entities
        println!("\nEntities:");
        for entity in &domain.entities {
            if let Some(handle) = &entity.handle {
                println!("  - {} ({})", handle, entity.roles.join(", "));

                if let Some(vcard) = &entity.vcard {
                    if let Some(name) = vcard.name() {
                        println!("    Name: {}", name);
                    }
                    if let Some(email) = vcard.email() {
                        println!("    Email: {}", email);
                    }
                }
            }
        }
    }

    Ok(())
}
