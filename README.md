# RDAP Rust Client

A modern, elegant RDAP (Registration Data Access Protocol) client written in Rust with beautiful colored output.

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
- IP address queries (IPv4/IPv6)
- AS number queries
- Entity queries
- Nameserver queries
- Search queries

🚀 **Smart Features**
- Automatic bootstrap service discovery
- Query type auto-detection
- Disk caching of bootstrap files
- Configurable timeouts

## Installation

### From Source

```bash
git clone https://github.com/Akaere-NetWorks/rdap.git
cd rdap
cargo build --release
sudo cp target/release/rdap /usr/local/bin/
```

### Using Cargo

```bash
cargo install rdap
```

## Usage

### Basic Queries

```bash
# Query a domain
rdap example.com

# Query an IP address
rdap 192.0.2.1
rdap 2001:db8::1

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

# Set custom timeout (in seconds)
rdap --timeout 60 example.com
```

### Output Formats

- `text` - Beautiful colored terminal output (default)
- `json` - Compact JSON
- `json-pretty` - Pretty-printed JSON

## Examples

### Domain Query

```bash
$ rdap -v example.com

→ Query: example.com
→ Type:  domain

⟳ Querying RDAP server...

═══ DOMAIN ═══
Domain Name: EXAMPLE.COM
Handle: 2336799_DOMAIN_COM-VRSN

Status:
  • client delete prohibited
  • client transfer prohibited
  • client update prohibited

Nameservers:
  • A.IANA-SERVERS.NET
  • B.IANA-SERVERS.NET

DNSSEC: Signed ✓

Events:
┌──────────────────┬─────────────────────────┐
│ Action           │ Date                    │
├──────────────────┼─────────────────────────┤
│ Registration     │ 1995-08-14T04:00:00Z   │
│ Expiration       │ 2024-08-13T04:00:00Z   │
│ Last Changed     │ 2023-08-14T07:01:38Z   │
└──────────────────┴─────────────────────────┘
```

### IP Query

```bash
$ rdap 8.8.8.8

═══ IP NETWORK ═══
Name: LVLT-GOGL-8-8-8
Handle: NET-8-8-8-0-1
Range: 8.8.8.0 - 8.8.8.255
Version: IPv4
Type: ALLOCATION
Country: US

Status:
  • active
```

### AS Number Query

```bash
$ rdap AS15169

═══ AUTNUM ═══
AS Number: AS15169
Name: GOOGLE
Type: Direct Allocation
Country: US
```

## Library Usage

You can also use this as a Rust library:

```rust
use rdap::{RdapClient, RdapRequest, QueryType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    
    let request = RdapRequest::new(QueryType::Domain, "example.com");
    let result = client.query(&request).await?;
    
    // Display with colors
    use rdap::display::RdapDisplay;
    result.display(false);
    
    Ok(())
}
```

## Architecture

```
src/
├── lib.rs           # Library entry point
├── main.rs          # CLI entry point
├── error.rs         # Error types
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
└── display.rs       # Pretty output formatting
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

Akaere Networks

## Links

- Original Go version: https://github.com/openrdap/rdap
- IANA RDAP Bootstrap: https://data.iana.org/rdap/
- RDAP Working Group: https://datatracker.ietf.org/wg/weirds/
