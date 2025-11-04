//! Beautiful colored output for RDAP objects

use crate::models::*;
use colored::*;

/// Display trait for RDAP objects
pub trait RdapDisplay {
    fn display(&self, verbose: bool);
}

impl RdapDisplay for RdapObject {
    fn display(&self, verbose: bool) {
        match self {
            RdapObject::Domain(d) => d.display(verbose),
            RdapObject::Entity(e) => e.display(verbose),
            RdapObject::Nameserver(ns) => ns.display(verbose),
            RdapObject::Autnum(a) => a.display(verbose),
            RdapObject::IpNetwork(ip) => ip.display(verbose),
            RdapObject::Error(err) => err.display(verbose),
            RdapObject::DomainSearch(ds) => ds.display(verbose),
            RdapObject::EntitySearch(es) => es.display(verbose),
            RdapObject::NameserverSearch(ns) => ns.display(verbose),
            RdapObject::Help(h) => h.display(verbose),
        }
    }
}

impl RdapDisplay for Domain {
    fn display(&self, verbose: bool) {
        // Domain name
        if let Some(name) = &self.ldh_name {
            println!("{}: {}", "Domain Name".bright_white().bold(), name.bright_cyan().bold());
        }
        
        if let Some(unicode) = &self.unicode_name {
            println!("{}: {}", "Unicode Name".white(), unicode.cyan());
        }
        
        if let Some(handle) = &self.handle {
            println!("{}: {}", "Handle".white(), handle.normal());
        }
        
        // Object class
        println!("{}: {}", "Object Class".white(), self.object_class_name.normal());
        
        // Port43
        if let Some(port43) = &self.port43 {
            println!("{}: {}", "Port43".white(), port43.normal());
        }
        
        // Status
        if !self.status.is_empty() {
            for status in &self.status {
                let color_status = match status.as_str() {
                    s if s.contains("active") => status.green(),
                    s if s.contains("delete") || s.contains("prohibit") => status.red(),
                    _ => status.yellow(),
                };
                println!("{}: {}", "Status".white(), color_status);
            }
        }
        
        // Nameservers
        if !self.nameservers.is_empty() {
            for ns in &self.nameservers {
                if let Some(name) = &ns.ldh_name {
                    print!("{}: {}", "Nameserver".white(), name.cyan());
                    if let Some(ips) = &ns.ip_addresses {
                        let addrs: Vec<String> = ips.v4.iter().chain(&ips.v6).cloned().collect();
                        if !addrs.is_empty() {
                            print!(" ({})", addrs.join(", ").dimmed());
                        }
                    }
                    println!();
                }
            }
        }
        
        // DNSSEC
        if let Some(dnssec) = &self.secure_dns {
            if let Some(zone_signed) = dnssec.zone_signed {
                println!("{}: {}", "Zone Signed".white(), 
                    if zone_signed { "yes".green() } else { "no".red() });
            }
            if let Some(delegation_signed) = dnssec.delegation_signed {
                println!("{}: {}", "Delegation Signed".white(), 
                    if delegation_signed { "yes".green() } else { "no".red() });
            }
            for ds in &dnssec.ds_data {
                if let Some(key_tag) = ds.key_tag {
                    println!("{}: {}", "DS Key Tag".white(), key_tag.to_string().normal());
                }
                if let Some(algorithm) = ds.algorithm {
                    println!("{}: {}", "DS Algorithm".white(), algorithm.to_string().normal());
                }
                if let Some(digest_type) = ds.digest_type {
                    println!("{}: {}", "DS Digest Type".white(), digest_type.to_string().normal());
                }
                if let Some(digest) = &ds.digest {
                    println!("{}: {}", "DS Digest".white(), digest.normal());
                }
            }
        }
        
        // Events
        for event in &self.events {
            let action = match event.action.as_str() {
                "registration" => "Registration",
                "expiration" => "Expiration",
                "last changed" => "Last Changed",
                "last update of RDAP database" => "Last Update",
                "transferred" => "Transferred",
                "locked" => "Locked",
                "unlocked" => "Unlocked",
                a => a,
            };
            println!("{}: {}", action.white(), event.date.normal());
        }
        
        // Entities
        if !self.entities.is_empty() {
            println!();
            for entity in &self.entities {
                display_entity(entity, verbose);
            }
        }
        
        // Links
        if verbose {
            for link in &self.links {
                if let Some(rel) = &link.rel {
                    println!("{}: {} ({})", "Link".white(), link.href.cyan(), rel.dimmed());
                } else {
                    println!("{}: {}", "Link".white(), link.href.cyan());
                }
            }
        }
        
        // Remarks
        if verbose {
            for remark in &self.remarks {
                display_notice(remark);
            }
        }
        
        // Notices
        if verbose {
            for notice in &self.notices {
                display_notice(notice);
            }
        }
        
        // Conformance
        if verbose && !self.conformance.is_empty() {
            println!("\n{}", "RDAP Conformance:".dimmed());
            for conf in &self.conformance {
                println!("  {}", conf.dimmed());
            }
        }
    }
}

impl RdapDisplay for IpNetwork {
    fn display(&self, verbose: bool) {
        if let Some(handle) = &self.handle {
            println!("{}: {}", "Handle".white(), handle.normal());
        }
        
        if let (Some(start), Some(end)) = (&self.start_address, &self.end_address) {
            println!("{}: {}", "Start Address".white(), start.cyan());
            println!("{}: {}", "End Address".white(), end.cyan());
        }
        
        if let Some(ip_ver) = &self.ip_version {
            println!("{}: {}", "IP Version".white(), format!("v{}", ip_ver).normal());
        }
        
        if let Some(name) = &self.name {
            println!("{}: {}", "Name".white(), name.cyan());
        }
        
        if let Some(net_type) = &self.network_type {
            println!("{}: {}", "Type".white(), net_type.normal());
        }
        
        if let Some(parent) = &self.parent_handle {
            println!("{}: {}", "Parent Handle".white(), parent.normal());
        }
        
        if let Some(country) = &self.country {
            println!("{}: {}", "Country".white(), country.green());
        }
        
        // Status
        for status in &self.status {
            println!("{}: {}", "Status".white(), status.green());
        }
        
        // Port43
        if let Some(port43) = &self.port43 {
            println!("{}: {}", "Port43".white(), port43.normal());
        }
        
        // Events
        for event in &self.events {
            println!("{}: {}", event.action.white(), event.date.normal());
        }
        
        // Entities
        if !self.entities.is_empty() {
            println!();
            for entity in &self.entities {
                display_entity(entity, verbose);
            }
        }
        
        // Links, Remarks, Notices
        if verbose {
            for link in &self.links {
                println!("{}: {}", "Link".white(), link.href.cyan());
            }
            for remark in &self.remarks {
                display_notice(remark);
            }
            for notice in &self.notices {
                display_notice(notice);
            }
        }
    }
}

impl RdapDisplay for Autnum {
    fn display(&self, verbose: bool) {
        // AS Number
        if let (Some(start), Some(end)) = (self.start_autnum, self.end_autnum) {
            if start == end {
                println!("{}: {}", "AS Number".white(), format!("AS{}", start).cyan().bold());
            } else {
                println!("{}: {}", "Start Autnum".white(), format!("AS{}", start).cyan());
                println!("{}: {}", "End Autnum".white(), format!("AS{}", end).cyan());
            }
        }
        
        if let Some(name) = &self.name {
            println!("{}: {}", "Name".white(), name.cyan());
        }
        
        if let Some(handle) = &self.handle {
            println!("{}: {}", "Handle".white(), handle.normal());
        }
        
        // Object class
        if let Some(class) = &self.object_class_name {
            println!("{}: {}", "Object Class".white(), class.normal());
        }
        
        if let Some(as_type) = &self.as_type {
            println!("{}: {}", "Type".white(), as_type.normal());
        }
        
        if let Some(country) = &self.country {
            println!("{}: {}", "Country".white(), country.green());
        }
        
        // Status
        for status in &self.status {
            println!("{}: {}", "Status".white(), status.green());
        }
        
        // Port43
        if let Some(port43) = &self.port43 {
            println!("{}: {}", "Port43".white(), port43.normal());
        }
        
        // Events
        for event in &self.events {
            let action = match event.action.as_str() {
                "registration" => "Registration",
                "last changed" => "Last Changed",
                a => a,
            };
            println!("{}: {}", action.white(), event.date.normal());
        }
        
        // Entities
        if !self.entities.is_empty() {
            println!();
            for entity in &self.entities {
                display_entity(entity, verbose);
            }
        }
        
        // Links, Remarks, Notices  
        if verbose {
            for link in &self.links {
                if let Some(rel) = &link.rel {
                    println!("{}: {} ({})", "Link".white(), link.href.cyan(), rel.dimmed());
                } else {
                    println!("{}: {}", "Link".white(), link.href.cyan());
                }
            }
            for remark in &self.remarks {
                display_notice(remark);
            }
            for notice in &self.notices {
                display_notice(notice);
            }
        }
        
        // Conformance
        if verbose && !self.conformance.is_empty() {
            println!("\n{}", "RDAP Conformance:".dimmed());
            for conf in &self.conformance {
                println!("  {}", conf.dimmed());
            }
        }
    }
}

impl RdapDisplay for Entity {
    fn display(&self, verbose: bool) {
        display_entity(self, verbose);
    }
}

impl RdapDisplay for Nameserver {
    fn display(&self, verbose: bool) {
        if let Some(name) = &self.ldh_name {
            println!("{}: {}", "Nameserver".white(), name.cyan().bold());
        }
        
        if let Some(handle) = &self.handle {
            println!("{}: {}", "Handle".white(), handle.normal());
        }
        
        if let Some(ips) = &self.ip_addresses {
            for ip in &ips.v4 {
                println!("{}: {}", "IPv4".white(), ip.cyan());
            }
            for ip in &ips.v6 {
                println!("{}: {}", "IPv6".white(), ip.cyan());
            }
        }
        
        // Status
        for status in &self.status {
            println!("{}: {}", "Status".white(), status.green());
        }
        
        // Events
        for event in &self.events {
            println!("{}: {}", event.action.white(), event.date.normal());
        }
        
        // Entities
        if !self.entities.is_empty() {
            println!();
            for entity in &self.entities {
                display_entity(entity, verbose);
            }
        }
        
        if verbose {
            for link in &self.links {
                println!("{}: {}", "Link".white(), link.href.cyan());
            }
            for remark in &self.remarks {
                display_notice(remark);
            }
            for notice in &self.notices {
                display_notice(notice);
            }
        }
    }
}

impl RdapDisplay for ErrorResponse {
    fn display(&self, _verbose: bool) {
        if let Some(code) = self.error_code {
            println!("{}: {}", "Error Code".red(), code.to_string().red().bold());
        }
        
        if let Some(title) = &self.title {
            println!("{}: {}", "Title".white(), title.normal());
        }
        
        for desc in &self.description {
            println!("{}: {}", "Description".white(), desc.normal());
        }
        
        for notice in &self.notices {
            display_notice(notice);
        }
    }
}

impl RdapDisplay for DomainSearchResults {
    fn display(&self, verbose: bool) {
        println!("{}: {}", "Domain Search Results".white(), self.domains.len().to_string().cyan());
        println!();
        
        for (i, domain) in self.domains.iter().enumerate() {
            if i > 0 {
                println!("\n{}", "---".dimmed());
            }
            domain.display(verbose);
        }
    }
}

impl RdapDisplay for EntitySearchResults {
    fn display(&self, verbose: bool) {
        println!("{}: {}", "Entity Search Results".white(), self.entities.len().to_string().cyan());
        println!();
        
        for (i, entity) in self.entities.iter().enumerate() {
            if i > 0 {
                println!("\n{}", "---".dimmed());
            }
            display_entity(entity, verbose);
        }
    }
}

impl RdapDisplay for NameserverSearchResults {
    fn display(&self, verbose: bool) {
        println!("{}: {}", "Nameserver Search Results".white(), self.nameservers.len().to_string().cyan());
        println!();
        
        for (i, ns) in self.nameservers.iter().enumerate() {
            if i > 0 {
                println!("\n{}", "---".dimmed());
            }
            ns.display(verbose);
        }
    }
}

impl RdapDisplay for HelpResponse {
    fn display(&self, _verbose: bool) {
        for notice in &self.notices {
            display_notice(notice);
        }
    }
}

// Helper functions

fn display_entity(entity: &Entity, verbose: bool) {
    // Entity header
    if let Some(handle) = &entity.handle {
        println!("{}: {}", "Entity Handle".white(), handle.normal());
    }
    
    if !entity.roles.is_empty() {
        for role in &entity.roles {
            println!("{}: {}", "Role".white(), role.yellow());
        }
    }
    
    // vCard information
    if let Some(vcard) = &entity.vcard {
        if let Some(name) = vcard.name() {
            println!("{}: {}", "Name".white(), name.cyan());
        }
        if let Some(org) = vcard.org() {
            println!("{}: {}", "Organization".white(), org.normal());
        }
        if let Some(email) = vcard.email() {
            println!("{}: {}", "Email".white(), email.cyan());
        }
        if let Some(tel) = vcard.tel() {
            println!("{}: {}", "Phone".white(), tel.normal());
        }
        
        if let Some(addr) = vcard.address() {
            if !addr.po_box.is_empty() {
                println!("{}: {}", "PO Box".white(), addr.po_box.normal());
            }
            if !addr.extended.is_empty() {
                println!("{}: {}", "Extended Address".white(), addr.extended.normal());
            }
            if !addr.street.is_empty() {
                println!("{}: {}", "Street".white(), addr.street.normal());
            }
            if !addr.locality.is_empty() {
                println!("{}: {}", "Locality".white(), addr.locality.normal());
            }
            if !addr.region.is_empty() {
                println!("{}: {}", "Region".white(), addr.region.normal());
            }
            if !addr.postal_code.is_empty() {
                println!("{}: {}", "Postal Code".white(), addr.postal_code.normal());
            }
            if !addr.country.is_empty() {
                println!("{}: {}", "Country".white(), addr.country.green());
            }
        }
        
        // Display all vCard properties in verbose mode
        if verbose {
            for prop in vcard.properties() {
                if !["fn", "email", "tel", "org", "adr"].contains(&prop.name.as_str()) {
                    println!("{}: {:?}", prop.name.white(), prop.value);
                }
            }
        }
    }
    
    // Status
    for status in &entity.status {
        println!("{}: {}", "Status".white(), status.green());
    }
    
    // Port43
    if let Some(port43) = &entity.port43 {
        println!("{}: {}", "Port43".white(), port43.normal());
    }
    
    // Events
    for event in &entity.events {
        println!("{}: {}", event.action.white(), event.date.normal());
    }
    
    // Public IDs
    for public_id in &entity.public_ids {
        println!("{}: {}", public_id.id_type.white(), public_id.identifier.cyan());
    }
    
    // Nested entities
    if !entity.entities.is_empty() && verbose {
        for sub_entity in &entity.entities {
            println!();
            display_entity(sub_entity, verbose);
        }
    }
    
    // Links, Remarks
    if verbose {
        for link in &entity.links {
            if let Some(rel) = &link.rel {
                println!("{}: {} ({})", "Link".white(), link.href.cyan(), rel.dimmed());
            } else {
                println!("{}: {}", "Link".white(), link.href.cyan());
            }
        }
        for remark in &entity.remarks {
            display_notice(remark);
        }
    }
}

fn display_notice(notice: &Notice) {
    if let Some(title) = &notice.title {
        println!("{}: {}", "Notice".white(), title.cyan());
    }
    for desc in &notice.description {
        println!("  {}", desc.normal());
    }
    for link in &notice.links {
        println!("  {}: {}", "Link".dimmed(), link.href.cyan());
    }
}
