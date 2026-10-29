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
- In-memory caching of IANA bootstrap files (24h)
- Configurable timeouts

🌐 **API Server & Networking**
- `rdap --serve`: local HTTP API server with RDAP-compatible and JSON endpoints
- Batch queries, abuse contact lookup, in-memory result cache
- Optional bearer token auth and CORS
- Outbound proxies (SOCKS5/HTTP), source IP / interface binding, route rotation
- Docker image and hardened systemd service

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

### Docker

```bash
docker build -t rdap https://github.com/xtomcom/rdap.git
docker run --rm rdap example.com
```

See [Docker / Compose](#docker--compose) for running the API server.

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

## API Server

`rdap --serve` runs a small HTTP API server. It speaks plain RDAP (RFC 7482 paths), so any RDAP client can use it, and adds JSON endpoints for auto-detected queries, abuse contacts and batches. Bootstrap discovery, registrar referrals and result caching all happen on the server.

### Starting and Stopping

```bash
# Listen on 127.0.0.1:8888 (default)
rdap --serve

# Listen on all interfaces, custom port (set a token when you do this)
RDAP_API_TOKEN=secret rdap --serve --listen 0.0.0.0 --port 9000
RDAP_API_TOKEN=secret rdap --serve --listen :: --port 9000

# Send every query to one upstream RDAP server instead of using bootstrap
rdap -s https://rdap.example/ --serve
```

The server prints its listen address on startup and shuts down gracefully on `Ctrl+C` (SIGINT) or SIGTERM.

### Server Options

Every option can also be set with an environment variable. A command-line flag wins over the variable.

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--serve` | | | Run the API server instead of a single query |
| `--listen <IP>` | `RDAP_LISTEN` | `127.0.0.1` | Address to listen on (`0.0.0.0` or `::` for all) |
| `--port <PORT>` | `RDAP_PORT` | `8888` | Port to listen on |
| `--token <TOKEN>` | `RDAP_API_TOKEN` | none | Require `Authorization: Bearer <TOKEN>` |
| `--cache-ttl <SECS>` | `RDAP_CACHE_TTL` | `300` | Seconds to cache results, `0` disables the cache |
| `--cors` | `RDAP_CORS` | off | Send permissive CORS headers (for browser clients) |
| `--max-batch <N>` | `RDAP_MAX_BATCH` | `100` | Maximum queries per `POST /batch` request |

`-s/--server`, `--timeout`, `-k/--insecure`, `--no-referral` and the [network options](#network-options) also apply to the server. `--max-per-host` defaults to `8` in server mode.

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/domain/{name}` | Domain (RDAP) |
| `GET` | `/ip/{addr}`, `/ip/{addr}/{len}` | IP address or CIDR (RDAP) |
| `GET` | `/autnum/{asn}` | AS number (RDAP) |
| `GET` | `/nameserver/{name}` | Nameserver (RDAP) |
| `GET` | `/entity/{handle}` | Entity (RDAP, needs `-s`) |
| `GET` | `/help` | RDAP help object for this server |
| `GET` | `/query/{q}` | Auto-detected query, wrapped in JSON |
| `GET` | `/abuse/{q}` | Abuse contact email |
| `POST` | `/batch` | Up to `--max-batch` queries at once |
| `GET` | `/healthz` | Health check (never needs the token) |
| `GET` | `/version` | Server name and version |

RDAP endpoints return `Content-Type: application/rdap+json`, everything else `application/json`.

#### RDAP Endpoints

These behave like an RDAP server: the body is the RDAP object itself. For domains the registrar's data is returned when the referral succeeded, otherwise the registry's; add `?source=registry` to always get the registry object.

```bash
curl http://127.0.0.1:8888/domain/example.com
curl http://127.0.0.1:8888/domain/example.com?source=registry
curl http://127.0.0.1:8888/ip/1.1.1.1
curl http://127.0.0.1:8888/ip/2001:db8::/32
curl http://127.0.0.1:8888/autnum/13335
curl http://127.0.0.1:8888/nameserver/ns1.google.com
curl http://127.0.0.1:8888/help
```

```json
{
  "objectClassName": "ip network",
  "handle": "1.1.1.0 - 1.1.1.255",
  "startAddress": "1.1.1.0",
  "endAddress": "1.1.1.255",
  "ipVersion": "v4",
  "name": "APNIC-LABS",
  "...": "..."
}
```

Any RDAP client can point at the server, including this one:

```bash
rdap -s http://127.0.0.1:8888/ example.com
```

`/entity/{handle}` and RDAP searches have no bootstrap registry, so they need a fixed upstream (`rdap -s <url> --serve`); without one they return `501`.

#### `GET /query/{q}`

Auto-detects the query type like the CLI: domains, TLDs (`google`), IPs, shorthand IPs (`1.1`), CIDRs, `AS13335` and `13335`.

Query parameters:

- `type` - force a query type (`domain`, `tld`, `ip`, `autnum`, `nameserver`, ...)
- `source` - `registrar` (default, falls back to the registry), `registry`, or `both`

```bash
curl http://127.0.0.1:8888/query/1.1
curl http://127.0.0.1:8888/query/AS13335
curl 'http://127.0.0.1:8888/query/example.com?source=both'
```

```json
{
  "query": "1.0.0.1",
  "type": "ip",
  "source": "registry",
  "registry_url": "https://rdap.apnic.net/ip/1.0.0.1",
  "registrar_url": null,
  "result": { "objectClassName": "ip network", "...": "..." }
}
```

With `source=both`, `result` is replaced by both objects:

```json
{
  "query": "example.com",
  "type": "domain",
  "source": "both",
  "registry_url": "https://rdap.verisign.com/com/v1/domain/example.com",
  "registrar_url": "https://rdap.example-registrar.com/domain/example.com",
  "registry": { "objectClassName": "domain", "...": "..." },
  "registrar": { "objectClassName": "domain", "...": "..." }
}
```

`registrar` is `null` when there was no referral or it failed.

#### `GET /abuse/{q}`

Returns the abuse contact email. For domains the registrar's data is checked first, then the registry's.

```bash
curl http://127.0.0.1:8888/abuse/1.1.1.1
```

```json
{
  "query": "1.1.1.1",
  "type": "ip",
  "abuse": "helpdesk@apnic.net",
  "source_url": "https://rdap.apnic.net/ip/1.1.1.1"
}
```

If the object exists but lists no abuse email, the response is still `200` with `"abuse": null`.

#### `POST /batch`

Runs several queries in one request (at most `--max-batch`, body up to 64 KiB). Duplicates are queried once, and queries run with bounded concurrency. The response is always `200`, with one result per query in input order; `data` has the same shape as `/query`.

```bash
curl -X POST http://127.0.0.1:8888/batch \
  -H 'Content-Type: application/json' \
  -d '{"queries": ["1.1.1.1", "example.com", "example.invalid"], "source": "registrar"}'
```

```json
{
  "results": [
    { "query": "1.1.1.1", "ok": true, "status": 200, "data": { "query": "1.1.1.1", "type": "ip", "...": "..." } },
    { "query": "example.com", "ok": true, "status": 200, "data": { "query": "example.com", "type": "domain", "...": "..." } },
    { "query": "example.invalid", "ok": false, "status": 404,
      "error": { "errorCode": 404, "title": "Not Found", "description": ["No RDAP record for example.invalid"] } }
  ]
}
```

#### `GET /healthz` and `GET /version`

```bash
curl http://127.0.0.1:8888/healthz
# {"status":"ok"}

curl http://127.0.0.1:8888/version
# {"name":"rdap","version":"1.1.0"}
```

### Errors

Errors use the RDAP error object (RFC 7480 section 5.3) with a matching HTTP status:

```json
{
  "errorCode": 404,
  "title": "Not Found",
  "description": ["No RDAP record for example.invalid"]
}
```

| Status | Meaning |
|--------|---------|
| `400` | Invalid query, `type`, `source` or batch body |
| `401` | Missing or wrong token (with `WWW-Authenticate: Bearer`) |
| `404` | Object not found, no RDAP server known for it, or unknown path |
| `405` | Wrong HTTP method |
| `413` | Batch has too many queries, or the body is too large |
| `429` | The upstream RDAP server rate-limited us |
| `501` | Query type needs a fixed upstream (`-s`): entities, searches |
| `502` | Upstream error, unparsable upstream response, or proxy failure |
| `504` | Upstream timeout |

### Authentication

Authentication is off by default. With a token set, every endpoint except `/healthz` requires it:

```bash
RDAP_API_TOKEN=secret rdap --serve

curl -H 'Authorization: Bearer secret' http://127.0.0.1:8888/query/example.com
```

Prefer `RDAP_API_TOKEN` over `--token`: command-line arguments are visible to other users in `ps`.

### Caching

Results are cached in memory for `--cache-ttl` seconds (default 300, `0` turns it off), keyed by query type and normalized query, so `/ip/1.1.1.1`, `/query/1.1.1.1` and `/abuse/1.1.1.1` share one upstream lookup whatever `source` they ask for. "Not found" answers, and domain results whose registrar referral failed, are cached for at most 60 seconds; upstream errors and timeouts are never cached. Concurrent identical requests share one upstream query.

### Security Notes

- The server listens on `127.0.0.1` by default, so only local clients can reach it.
- Listening on any other address without a token prints a warning on startup, but the server still starts. **Do not expose it to a network without `RDAP_API_TOKEN`**: anyone who can reach it can send queries from your IP addresses and proxies, which can get them rate-limited or blocked by RDAP servers.
- The server speaks plain HTTP. To reach it over an untrusted network, put it behind a TLS reverse proxy.
- CORS headers are only sent with `--cors`.
- Proxy credentials are never logged.

## Network Options

These options control how rdap connects to upstream RDAP servers (and to IANA for bootstrap files). They work for one-shot queries and for `--serve`.

| Flag | Env var | Description |
|------|---------|-------------|
| `--interface <NAME>` | `RDAP_INTERFACE` | Send queries from this network interface (e.g. `eth1`) |
| `--bind <IP>[,<IP>...]` | `RDAP_BIND` | Source IP(s); several addresses rotate |
| `--proxy <URL>[,<URL>...]` | `RDAP_PROXY` | Proxy URL(s); several proxies rotate |
| `--max-per-host <N>` | `RDAP_MAX_PER_HOST` | Concurrent requests per upstream server (default: unlimited, 8 with `--serve`) |

```bash
# Send queries from a specific interface or source address
rdap --interface eth1 example.com
rdap --bind 203.0.113.10 example.com
rdap --bind 2001:db8::10 2001:db8::1

# Rotate between source addresses
rdap --serve --bind 203.0.113.10,203.0.113.11

# Use a proxy, or rotate between several
rdap --proxy socks5h://127.0.0.1:1080 example.com
rdap --serve --proxy socks5h://user:pass@10.0.0.1:1080,socks5h://10.0.0.2:1080

# Be gentle with upstream servers
rdap --serve --max-per-host 4
```

- **Proxies**: `socks5://`, `socks5h://` (DNS resolved by the proxy), `http://` and `https://`, with optional `user:pass@` credentials.
- **Rotation**: each request uses the next route, round-robin. With `--proxy`, routes are the proxies; otherwise they are the `--bind` addresses. `--interface` applies to every route.
- **Failover**: if a route can't connect (connect error or connect timeout, proxy error), the request is retried on the next route, up to 3 attempts. Requests that got an HTTP response, or timed out waiting for one (a slow server), are never retried.
- `--bind` and `--proxy` accept repeated flags or comma-separated lists.
- `--interface` uses `SO_BINDTODEVICE` on Linux and may need `CAP_NET_RAW`. If an option is not supported on your OS, or a proxy URL is invalid, rdap exits with an error at startup.

## Docker / Compose

The `Dockerfile` builds a small Debian-based image that runs the API server as an unprivileged user (UID 10001). Inside the container it listens on `0.0.0.0:8888`.

```bash
docker build -t rdap .

# API server, published on localhost only
docker run -d --name rdap -p 127.0.0.1:8888:8888 rdap
curl http://127.0.0.1:8888/query/1.1.1.1

# With a token and a proxy
docker run -d --name rdap -p 8888:8888 \
  -e RDAP_API_TOKEN=secret -e RDAP_PROXY=socks5h://10.0.0.1:1080 rdap

# One-shot queries work too
docker run --rm rdap 1.1.1.1
docker run --rm rdap -f json-pretty example.com
```

`compose.yaml` is a reference setup: it builds the image, publishes `127.0.0.1:8888`, and runs with a read-only root filesystem, no capabilities and `no-new-privileges`. `$HOME` (`/home/rdap`) is a named volume, so configs fetched with `rdap --update` survive restarts. Uncomment the `environment:` entries to configure the server.

```bash
docker compose up -d --build
docker compose logs -f
docker compose run --rm rdap --update   # refresh bootstrap/TLD configs in the volume
docker compose down
```

## systemd Service

The deb and rpm packages install a hardened `rdap.service` that runs `rdap --serve` with a dynamic, unprivileged user. It is **not** enabled or started on install.

```bash
# Configure (listen address, token, proxies, ...)
sudo editor /etc/default/rdap

# Start now and on boot
sudo systemctl enable --now rdap

# Apply config changes / check status / logs
sudo systemctl restart rdap
systemctl status rdap
journalctl -u rdap -f

# Stop and disable
sudo systemctl disable --now rdap
```

`/etc/default/rdap` lists every `RDAP_*` variable, commented out with its default, and is kept on upgrades. It is readable by root only, since it may hold the API token or proxy credentials. Flags without an env var (such as `--timeout` or `-s`) go in `RDAP_ARGS`. For other changes, use `systemctl edit rdap` instead of editing the unit file.

The service's `$HOME` is its state directory, `/var/lib/rdap`. It reads system-wide configs from `/etc/rdap/` as usual, so to give it fresher configs than the built-in ones, run `rdap --update` as any user and copy the results there:

```bash
rdap --update
sudo mkdir -p /etc/rdap
sudo cp ~/.config/rdap/config.json ~/.config/rdap/tlds.json ~/.config/rdap/tlds.txt /etc/rdap/
sudo systemctl restart rdap
```

If `--interface` fails with "Operation not permitted", grant `CAP_NET_RAW` as described in the unit file.

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
├── net.rs           # Outbound HTTP: proxies, bind/interface, rotation, per-host limits
├── server/          # API server (`rdap --serve`, feature `server`)
├── request.rs       # Request builder
├── bootstrap.rs     # Bootstrap service discovery
├── cache.rs         # Disk cache helper (unused)
├── ip.rs            # IP address normalization and CIDR handling
└── display.rs       # Pretty output formatting

config/
├── config.json      # Default bootstrap URLs
├── tlds.json        # Default TLD overrides for ccTLDs
└── tlds.txt         # IANA TLD list for TLD query detection

packaging/
├── rdap.service     # systemd unit (deb/rpm)
├── rdap.env         # /etc/default/rdap (deb/rpm)
└── arch/            # Arch Linux PKGBUILDs

Dockerfile           # Container image (API server by default)
compose.yaml         # Reference Compose setup
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
