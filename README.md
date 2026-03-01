# RDAP Rust Client

A modern, elegant RDAP (Registration Data Access Protocol) client written in Rust with beautiful colored output.

Forked from [Akaere-NetWorks/rdap](https://github.com/Akaere-NetWorks/rdap), add more features and fix bugs.

## Features

✨ **Modern & Fast**
- Asynchronous I/O with Tokio
- Efficient HTTP client with connection pooling
- Fast JSON parsing with Serde

🎨 **Beautiful Output**
- Colorized terminal output
- Pretty-printed tables
- Clear hierarchical display

🔍 **Full RDAP Support**
- Domain queries
- TLDs queries
- IP address queries (IPv4/IPv6)
- IP CIDR queries (IPv4/IPv6)
- AS number queries
- Entity queries
- Nameserver queries
- Search queries

🚀 **Smart Features**
- Automatic bootstrap service discovery
- Query type auto-detection
- Multi-layer RDAP queries (registry + registrar for domains)
- Custom TLD overrides for ccTLDs not in IANA bootstrap
- Abuse contact display for IP/ASN queries
- Disk caching of bootstrap files
- Configurable timeouts

⚙️ **Configuration**
- Built-in defaults for zero-config usage
- Configurable bootstrap URLs and TLD overrides
- Support for local override files (*.local.json)
- `rdap --update` command to fetch latest configs from GitHub

## Installation

### From Source

```bash
git clone https://github.com/xtomcom/rdap.git
cd rdap
cargo build --release
sudo cp target/release/rdap /usr/local/bin/
```

### Debian / Ubuntu

Debian (via extrepo):

```bash
sudo apt update && sudo apt install extrepo -y
sudo extrepo enable xtom
sudo apt update && sudo apt install rdap -y
```

One-Line Style:

```bash
sudo apt install -y lsb-release ca-certificates apt-transport-https curl gnupg dpkg
# Add xTom repository
curl -fsSL https://repo.xtom.com/xtom.key | bash -c 'gpg --dearmor > /usr/share/keyrings/xtom.gpg'
echo "deb [signed-by=/usr/share/keyrings/xtom.gpg] https://repo.xtom.com/deb stable main" | sudo tee /etc/apt/sources.list.d/xtom.list
sudo apt update
sudo apt install rdap
```

DEB822:

```bash
sudo apt install -y lsb-release ca-certificates apt-transport-https curl gnupg dpkg
curl -fsSL https://repo.xtom.com/xtom.key | bash -c 'gpg --dearmor > /usr/share/keyrings/xtom.gpg'
sudo bash -c 'cat > /etc/apt/sources.list.d/xtom.sources << EOF
Components: main
Architectures: $(dpkg --print-architecture)
Suites: stable
Types: deb
Uris: https://repo.xtom.com/deb
Signed-By: /usr/share/keyrings/xtom.gpg
EOF'
sudo apt update
sudo apt install rdap -y
```

### RHEL / CentOS / Rocky / Alma / Fedora

```bash
# Add xTom repository
sudo curl -o /etc/yum.repos.d/xtom.repo https://repo.xtom.com/rpm/xtom.repo
sudo dnf install rdap
```

### macOS

```bash
# Install Homebrew https://brew.sh/
brew tap xtomcom/brew
brew install xtomcom/brew/rdap
```

## Usage

### Basic Queries

```bash
# Query a domain
rdap example.com

# Query a TLD (top-level domain)
rdap google
rdap com
rdap io

# Query an IP address
rdap 192.0.2.1
rdap 2001:db8::1

# Query with shorthand IP (auto-normalized)
rdap 1.1          # → queries 1.0.0.1
rdap 8.8          # → queries 8.0.0.8

# Query a CIDR range
rdap 8.8.8.0/24

# Query an AS number
rdap AS15169
rdap 15169

# Query with verbose output
rdap -v example.com
```

### Advanced Options

```bash
# Specify query type explicitly
rdap -t domain example.com

# Use a specific RDAP server
rdap -s https://rdap.verisign.com/com/v1 example.com

# JSON output
rdap -f json example.com
rdap -f json-pretty example.com

# JSON output from registry (default uses registrar data for domain queries)
rdap -f json --json-source registry example.com

# Set custom timeout (in seconds)
rdap --timeout 60 example.com

# Disable registrar referral following for domain queries
rdap --no-referral example.com

# Update configuration files from GitHub
rdap --update
rdap -u
```

### Output Formats

- `text` - Beautiful colored terminal output (default)
- `json` - Compact JSON
- `json-pretty` - Pretty-printed JSON

## Examples

### Domain Query

```bash
$ rdap google.com

Abuse contact for `google.com` is `abusecomplaints@markmonitor.com`

Query from https://rdap.verisign.com/com/v1/domain/google.com

Domain Name: GOOGLE.COM
Handle: 2138514_DOMAIN_COM-VRSN
Object Class: domain
Status: client delete prohibited
Status: client transfer prohibited
Status: client update prohibited
Status: server delete prohibited
Status: server transfer prohibited
Status: server update prohibited
Nameserver: NS1.GOOGLE.COM
Nameserver: NS2.GOOGLE.COM
Nameserver: NS3.GOOGLE.COM
Nameserver: NS4.GOOGLE.COM
Delegation Signed: no
Registration: 1997-09-15T04:00:00Z
Expiration: 2028-09-14T04:00:00Z
Last Changed: 2019-09-09T15:39:04Z
Last Update: 2026-02-03T10:25:50Z

Entity Handle: 292
Role: registrar
Name: MarkMonitor Inc.
IANA Registrar ID: 292

Query from https://rdap.markmonitor.com/rdap/domain/GOOGLE.COM

Domain Name: google.com
Handle: 2138514_DOMAIN_COM-VRSN
Object Class: domain
Port43: whois.markmonitor.com
Status: client update prohibited
Status: client transfer prohibited
Status: client delete prohibited
Status: server update prohibited
Status: server transfer prohibited
Status: server delete prohibited
Nameserver: ns1.google.com
Nameserver: ns2.google.com
Nameserver: ns3.google.com
Nameserver: ns4.google.com
Delegation Signed: no
registrar expiration: 2028-09-14T07:00:00.000+00:00
Expiration: 2028-09-13T07:00:00.000+00:00
Registration: 1997-09-15T07:00:00.000+00:00
Last Changed: 2024-08-02T02:17:33.000+00:00
Last Update: 2026-02-03T10:21:36.000+00:00

Role: registrant
Name: REDACTED REGISTRANT
Organization: Google LLC
Email: REDACTED FOR PRIVACY
Phone: REDACTED FOR PRIVACY
Street: REDACTED FOR PRIVACY
Locality: REDACTED FOR PRIVACY
Postal Code: REDACTED FOR PRIVACY
last changed: 2017-12-11T15:40:13.000+00:00
Role: technical
last changed: 2017-12-11T15:40:13.000+00:00
Entity Handle: 292
Role: registrar
Name: Markmonitor Inc.
Organization: Markmonitor Inc.
Street: 3540 E Longwing Ln
Locality: Meridian
Region: ID
Postal Code: 83646
IANA Registrar ID: 292
Link: https://rdap.markmonitor.com/rdap/domain/google.com
```

### TLD Query

```bash
$ rdap google

Administrative contact for `google` is `iana-contact@google.com`

Technical contact for `google` is `crr-tech@google.com`

Query from https://rdap.iana.org/domain/google

Domain Name: google
Object Class: domain
Status: active
Nameserver: ns-tld1.charlestonroadregistry.com (216.239.32.105, 2001:4860:4802:32::69)
Nameserver: ns-tld2.charlestonroadregistry.com (216.239.34.105, 2001:4860:4802:34::69)
Nameserver: ns-tld3.charlestonroadregistry.com (216.239.36.105, 2001:4860:4802:36::69)
Nameserver: ns-tld4.charlestonroadregistry.com (216.239.38.105, 2001:4860:4802:38::69)
Nameserver: ns-tld5.charlestonroadregistry.com (216.239.60.105, 2001:4860:4805::69)
Delegation Signed: yes
DS Key Tag: 6125
DS Algorithm: 8
DS Digest Type: 2
DS Digest: 80F8B78D23107153578BAD3800E9543500474E5C30C29698B40A3DB23ED9DA9F
Last Changed: 2025-04-11T00:00:00+00:00
Registration: 2014-09-04T00:00:00+00:00
```

### IP Query (with CIDR support)

```bash
$ rdap 8.8.8.0/24

Abuse contact for `8.8.8.0/24` is `network-abuse@google.com`

Query from https://rdap.arin.net/registry/ip/8.8.8.0/24

Handle: NET-8-8-8-0-2
Start Address: 8.8.8.0
End Address: 8.8.8.255
IP Version: v4
Name: GOGL
Type: DIRECT ALLOCATION
Parent Handle: NET-8-0-0-0-0
Status: active
Port43: whois.arin.net
last changed: 2023-12-28T17:24:56-05:00
registration: 2023-12-28T17:24:33-05:00
```
### IP Query

```bash
$ rdap 8.8.8.8

Abuse contact for `8.8.8.8` is `network-abuse@google.com`

Query from https://rdap.arin.net/registry/ip/8.8.8.8

Handle: NET-8-8-8-0-2
Start Address: 8.8.8.0
End Address: 8.8.8.255
IP Version: v4
Name: GOGL
Type: DIRECT ALLOCATION
Parent Handle: NET-8-0-0-0-0
Status: active
Port43: whois.arin.net
last changed: 2023-12-28T17:24:56-05:00
registration: 2023-12-28T17:24:33-05:00
```

### AS Number Query

```bash
$ rdap AS8888

Abuse contact for `AS8888` is `abuse@xtom.com`

Query from https://rdap.db.ripe.net/autnum/8888

AS Number: AS8888
Name: XTOM
Handle: AS8888
Object Class: autnum
Status: active
Port43: whois.ripe.net
Registration: 1970-01-01T00:00:00Z
Last Changed: 2024-08-17T11:00:40Z

Entity Handle: ORG-XPL3-RIPE
Role: registrant
Name: xTom Pty Ltd
Phone: +61280066886
Address: 81 Campbell St
2010
Surry Hills
AUSTRALIA
Link: https://rdap.db.ripe.net/entity/ORG-XPL3-RIPE
```

### Entity Query

```bash
$ rdap -s https://rdap.db.ripe.net -t entity XTOM-RIPE

Query from https://rdap.db.ripe.net/entity/XTOM-RIPE

Entity Handle: XTOM-RIPE
Name: xTom Global NOC
Email: abuse@xtom.com
Phone: +49 21197635976
Address: Kreuzstr.60
40210 Duesseldorf
Germany
Port43: whois.ripe.net
registration: 2021-08-05T13:48:15Z
last changed: 2021-10-27T10:21:57Z
Link: https://rdap.db.ripe.net/entity/XTOM-RIPE

Entity Handle: xtom
Role: registrant
Name: xtom
Organization: ORG-XG42-RIPE
Link: https://rdap.db.ripe.net/entity/xtom
```

### Verbose Output

```bash
$ rdap -v AS8888

→ Query: AS8888
→ Type:  autnum

⟳ Querying RDAP server...


Abuse contact for `AS8888` is `abuse@xtom.com`

Query from https://rdap.db.ripe.net/autnum/8888

AS Number: AS8888
Name: XTOM
Handle: AS8888
Object Class: autnum
Status: active
Port43: whois.ripe.net
Registration: 1970-01-01T00:00:00Z
Last Changed: 2024-08-17T11:00:40Z

Entity Handle: ORG-XPL3-RIPE
Role: registrant
Name: xTom Pty Ltd
Phone: +61280066886
Address: 81 Campbell St
2010
Surry Hills
AUSTRALIA
Link: https://rdap.db.ripe.net/entity/ORG-XPL3-RIPE
Link: http://www.ripe.net/data-tools/support/documentation/terms (copyright)

Entity Handle: RIPE-NCC-END-MNT
Role: registrant
Name: RIPE-NCC-END-MNT
Organization: ORG-NCC1-RIPE
Link: https://rdap.db.ripe.net/entity/RIPE-NCC-END-MNT
Link: http://www.ripe.net/data-tools/support/documentation/terms (copyright)

Entity Handle: XTAU-RIPE
Role: administrative
Role: technical
Role: abuse
Name: xTom Global NOC
Email: abuse@xtom.com
Phone: +61 2 8006 6886
Address: 81 Campbell St
Surry Hills 2010 NSW
Australia
Link: https://rdap.db.ripe.net/entity/XTAU-RIPE
Link: http://www.ripe.net/data-tools/support/documentation/terms (copyright)

Entity Handle: xtom
Role: registrant
Name: xtom
Organization: ORG-XG42-RIPE
Link: https://rdap.db.ripe.net/entity/xtom
Link: http://www.ripe.net/data-tools/support/documentation/terms (copyright)

Link: https://rdap.db.ripe.net/autnums/rirSearch1/rdap-up/AS8888 (rdap-up)
Link: https://rdap.db.ripe.net/autnums/rirSearch1/rdap-up/AS8888?status=active (rdap-up rdap-active)
Link: https://rdap.db.ripe.net/autnums/rirSearch1/rdap-down/AS8888 (rdap-down)
Link: https://rdap.db.ripe.net/autnums/rirSearch1/rdap-top/AS8888 (rdap-top)
Link: https://rdap.db.ripe.net/autnums/rirSearch1/rdap-top/AS8888?status=active (rdap-top rdap-active)
Link: https://rdap.db.ripe.net/autnums/rirSearch1/rdap-bottom/AS8888 (rdap-bottom)
Link: https://rdap.db.ripe.net/autnum/8888 (self)
Link: http://www.ripe.net/data-tools/support/documentation/terms (copyright)
  xTom Pty Ltd
  
  ========================================================
  ===== X X TTTTT OOO M M =====
  ===== X X T O O MM MM =====
  ===== X T O O M M M M =====
  ===== X X T O O M M M =====
  ===== X X T OOO M M =====
  ============== BGP COMMUNITY SUPPORT ===================
  ===== 8888:3002 learned from EQUINIX SYDNEY ====
  ===== 8888:3003 learned from CT (AS4134) ====
  ===== 8888:3004 learned from CU (AS10099) ====
  ===== 8888:3005 learned from CT2 (AS4809) ====
  ===== 8888:3006 learned from GSL (AS137409) ====
  ===== 8888:110x Prepend routes to EQUINIX SYDNEY ====
  ===== 8888:120x Prepend routes to CT (AS4134) ====
  ===== 8888:130x Prepend routes to CU (AS10099) ====
  ===== 8888:140x Prepend routes to CT2 (AS4809) ====
  ===== 8888:150x Prepend routes to GSL (AS137409) ====
  ===== 8888:666 Blackhole Community ====
  ==================== LIST OF CONTENTS ==================
  ===== x=1,2,3 for prepends ====
  ===== x=0 for DO NOT announce ====
  ===== x=4 for DO NOT announce Route Servers ====
  ===== x=5 for DO NOT announce Transits ====
  ==================== Contacts ==========================
  ===== Network noc@xtom.com ====
  ===== Peering peering@xtom.com ====
  ===== Abuse abuse@xtom.com ====
  ===== Website https://xtom.com ====
Notice: Filtered
  This output has been filtered.
Notice: Whois Inaccuracy Reporting
  If you see inaccuracies in the results, please visit:
  Link: https://www.ripe.net/contact-form?topic=ripe_dbm&show_form=true
Notice: Source
  Objects returned came from source
  RIPE
Notice: Terms and Conditions
  This is the RIPE Database query service. The objects are in RDAP format.
  Link: http://www.ripe.net/db/support/db-terms-conditions.pdf

RDAP Conformance:
  nro_rdap_profile_asn_flat_0
  rirSearch1
  autnums
  cidr0
  rdap_level_0
  nro_rdap_profile_0
  redacted
```

## Library Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rdap = { git = "https://github.com/xtomcom/rdap.git" }
tokio = { version = "1.49", features = ["full"] }
```

Or use a specific version/branch:

```toml
[dependencies]
# Use main branch
rdap = { git = "https://github.com/xtomcom/rdap.git", branch = "main" }

# Or use a specific tag (when available)
# rdap = { git = "https://github.com/xtomcom/rdap.git", tag = "v0.1.0" }

# Or use a specific commit
# rdap = { git = "https://github.com/xtomcom/rdap.git", rev = "abc123" }

tokio = { version = "1.49", features = ["full"] }
```

### Basic Query

```rust
use rdap::{RdapClient, RdapRequest, QueryType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = RdapClient::new()?;
    
    // Query a domain
    let request = RdapRequest::new(QueryType::Domain, "example.com");
    let result = client.query(&request).await?;
    
    // Display with colored output
    use rdap::display::RdapDisplay;
    result.display(false); // false = non-verbose
    
    Ok(())
}
```

### Auto-Detection

```rust
use rdap::{RdapClient, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    
    // Auto-detect query type
    let query = "8.8.8.8";
    let query_type = RdapRequest::detect_type(query)?;
    
    let request = RdapRequest::new(query_type, query);
    let result = client.query(&request).await?;
    
    // Process the result based on type
    match result {
        rdap::RdapObject::Domain(domain) => {
            println!("Domain: {}", domain.ldh_name.unwrap_or_default());
        }
        rdap::RdapObject::IpNetwork(ip) => {
            println!("IP Network: {}", ip.name.unwrap_or_default());
        }
        rdap::RdapObject::Autnum(asn) => {
            println!("AS Number: AS{}", asn.start_autnum.unwrap_or(0));
        }
        _ => {}
    }
    
    Ok(())
}
```

### Custom Server

```rust
use rdap::{RdapClient, RdapRequest, QueryType};
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    
    // Use a specific RDAP server
    let server = Url::parse("https://rdap.verisign.com/com/v1")?;
    let request = RdapRequest::new(QueryType::Domain, "example.com")
        .with_server(server);
    
    let result = client.query(&request).await?;
    
    Ok(())
}
```

### JSON Output

```rust
use rdap::{RdapClient, RdapRequest, QueryType};
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    let request = RdapRequest::new(QueryType::Domain, "example.com");
    let result = client.query(&request).await?;
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&result)?;
    println!("{}", json);
    
    Ok(())
}
```

### Working with Domain Data

```rust
use rdap::{RdapClient, RdapRequest, QueryType, RdapObject};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    let request = RdapRequest::new(QueryType::Domain, "example.com");
    let result = client.query(&request).await?;
    
    if let RdapObject::Domain(domain) = result {
        // Access domain properties
        println!("Domain: {}", domain.ldh_name.unwrap_or_default());
        
        // Check status
        for status in &domain.status {
            println!("Status: {}", status);
        }
        
        // List nameservers
        for ns in &domain.nameservers {
            if let Some(name) = &ns.ldh_name {
                println!("Nameserver: {}", name);
            }
        }
        
        // Check DNSSEC
        if let Some(dnssec) = &domain.secure_dns {
            if let Some(signed) = dnssec.delegation_signed {
                println!("DNSSEC: {}", if signed { "Signed" } else { "Not signed" });
            }
        }
        
        // Access entities (registrar, registrant, etc.)
        for entity in &domain.entities {
            if let Some(handle) = &entity.handle {
                println!("Entity: {} (roles: {:?})", handle, entity.roles);
            }
            
            // Access vCard data
            if let Some(vcard) = &entity.vcard {
                if let Some(name) = vcard.name() {
                    println!("  Name: {}", name);
                }
                if let Some(email) = vcard.email() {
                    println!("  Email: {}", email);
                }
            }
        }
        
        // Access events
        for event in &domain.events {
            println!("Event: {} at {}", event.action, event.date);
        }
    }
    
    Ok(())
}
```

### Working with IP Network Data

```rust
use rdap::{RdapClient, RdapRequest, QueryType, RdapObject};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    let request = RdapRequest::new(QueryType::Ip, "8.8.8.8");
    let result = client.query(&request).await?;
    
    if let RdapObject::IpNetwork(network) = result {
        println!("Network: {}", network.name.unwrap_or_default());
        println!("Range: {} - {}", 
            network.start_address.unwrap_or_default(),
            network.end_address.unwrap_or_default()
        );
        println!("Type: {}", network.network_type.unwrap_or_default());
        
        if let Some(country) = &network.country {
            println!("Country: {}", country);
        }
    }
    
    Ok(())
}
```

### Working with AS Number Data

```rust
use rdap::{RdapClient, RdapRequest, QueryType, RdapObject};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    let request = RdapRequest::new(QueryType::Autnum, "AS15169");
    let result = client.query(&request).await?;
    
    if let RdapObject::Autnum(asn) = result {
        if let Some(start) = asn.start_autnum {
            println!("AS Number: AS{}", start);
        }
        println!("Name: {}", asn.name.unwrap_or_default());
        println!("Type: {}", asn.as_type.unwrap_or_default());
        
        if let Some(country) = &asn.country {
            println!("Country: {}", country);
        }
    }
    
    Ok(())
}
```

### Error Handling

```rust
use rdap::{RdapClient, RdapRequest, QueryType, RdapObject, RdapError};

#[tokio::main]
async fn main() {
    let client = RdapClient::new().unwrap();
    let request = RdapRequest::new(QueryType::Domain, "example.com");
    
    match client.query(&request).await {
        Ok(result) => {
            // Handle successful response
            match result {
                RdapObject::Error(err) => {
                    eprintln!("RDAP Error: {}", err.title.unwrap_or_default());
                    for desc in &err.description {
                        eprintln!("  {}", desc);
                    }
                }
                _ => {
                    use rdap::display::RdapDisplay;
                    result.display(false);
                }
            }
        }
        Err(e) => {
            match e {
                RdapError::Bootstrap(msg) => {
                    eprintln!("Bootstrap error: {}", msg);
                }
                RdapError::Http(err) => {
                    eprintln!("HTTP error: {}", err);
                }
                RdapError::InvalidQuery(msg) => {
                    eprintln!("Invalid query: {}", msg);
                }
                _ => {
                    eprintln!("Error: {}", e);
                }
            }
        }
    }
}
```

### Advanced: Custom Timeout and Configuration

```rust
use rdap::{RdapClient, RdapRequest, QueryType};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with custom timeout
    let client = RdapClient::new()?
        .with_timeout(Duration::from_secs(30));
    
    let request = RdapRequest::new(QueryType::Domain, "example.com");
    let result = client.query(&request).await?;
    
    Ok(())
}
```

### Integration Example: Web Service

Here's an example of using the RDAP library in a web service:

```rust
use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use rdap::{RdapClient, RdapRequest};
use std::sync::Arc;
use tokio::sync::Mutex;

// Shared RDAP client
struct AppState {
    rdap_client: Arc<Mutex<RdapClient>>,
}

#[tokio::main]
async fn main() {
    let client = RdapClient::new().unwrap();
    let state = Arc::new(AppState {
        rdap_client: Arc::new(Mutex::new(client)),
    });

    let app = Router::new()
        .route("/rdap/:query", get(query_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}

async fn query_handler(
    Path(query): Path<String>,
    state: axum::extract::State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = state.rdap_client.lock().await;
    
    // Auto-detect query type
    let query_type = RdapRequest::detect_type(&query)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let request = RdapRequest::new(query_type, &query);
    
    match client.query(&request).await {
        Ok(result) => {
            let json = serde_json::to_value(&result)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(json))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

## Configuration

The RDAP client uses a configuration system with the following priority (highest to lowest):

1. `~/.config/rdap/*.local.json` - User local overrides (never overwritten by updates)
2. `~/.config/rdap/*.json` - Downloaded configs (updated via `rdap --update`)
3. `/etc/rdap/*.json` - System-wide configs
4. Built-in defaults (embedded in binary)

### Configuration Files

- **config.json** - Bootstrap URLs for IANA RDAP service discovery
- **tlds.json** - TLD overrides for ccTLDs not in IANA bootstrap
- **config.local.json** - Your custom bootstrap config (optional, survives updates)
- **tlds.local.json** - Your custom TLD overrides (merged on top of tlds.json)

### Updating Configs

```bash
# Update configs from GitHub
rdap --update
rdap -u
```

This downloads:
- `config.json` and `tlds.json` from the GitHub repository
- `tlds.txt` (IANA TLD list) from `https://data.iana.org/TLD/tlds-alpha-by-domain.txt`

Your `*.local.json` files are preserved.

### Custom TLD Overrides

Create `~/.config/rdap/tlds.local.json` to add your own TLD overrides:

```json
{
  "example": "https://rdap.example.com/",
  "co.example": "https://rdap.co.example.com/"
}
```

These will be merged on top of the base `tlds.json` configuration.

## Architecture

```
src/
├── lib.rs           # Library entry point
├── main.rs          # CLI entry point
├── error.rs         # Error types
├── config.rs        # Configuration management
├── models/          # RDAP data models
│   ├── domain.rs
│   ├── entity.rs
│   ├── autnum.rs
│   ├── ip_network.rs
│   ├── nameserver.rs
│   └── ...
├── client.rs        # RDAP client
├── request.rs       # Request builder
├── bootstrap.rs     # Bootstrap service discovery
├── cache.rs         # Bootstrap cache
├── ip.rs            # IP address normalization and CIDR handling
└── display.rs       # Pretty output formatting

config/
├── config.json      # Default bootstrap URLs
├── tlds.json        # Default TLD overrides for ccTLDs
└── tlds.txt         # IANA TLD list for TLD query detection
```

## RFCs Implemented

- [RFC 7480](https://tools.ietf.org/html/rfc7480) - HTTP Usage in RDAP
- [RFC 7482](https://tools.ietf.org/html/rfc7482) - RDAP Query Format
- [RFC 7483](https://tools.ietf.org/html/rfc7483) - JSON Responses for RDAP
- [RFC 7484](https://tools.ietf.org/html/rfc7484) - Finding the Authoritative RDAP Service
- [RFC 6350](https://tools.ietf.org/html/rfc6350) - vCard Format
- [RFC 7095](https://tools.ietf.org/html/rfc7095) - jCard

## Comparison with Go Version

| Feature | Go Version | Rust Version |
|---------|-----------|--------------|
| Performance | ⚡ Fast | ⚡⚡ Very Fast |
| Memory Usage | Good | Excellent |
| Colored Output | Basic | Advanced |
| Table Formatting | None | Beautiful |
| Async Support | Yes | Yes (Tokio) |
| Type Safety | Runtime | Compile-time |
| Binary Size | ~8MB | ~4MB (stripped) |

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see LICENSE file for details

## Author

xTom & Akaere Networks

## Links

- Original Go version: https://github.com/openrdap/rdap
- IANA RDAP Bootstrap: https://data.iana.org/rdap/
- RDAP Working Group: https://datatracker.ietf.org/wg/weirds/
