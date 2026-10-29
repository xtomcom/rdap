# rdap

`rdap` is an RDAP (Registration Data Access Protocol) client, library and API server written in Rust. It implements RFC 7480–7484 and queries registration data for domains, TLDs, IPv4/IPv6 addresses and prefixes, autonomous system numbers, nameservers and entities.

The project is a fork of [Akaere-NetWorks/rdap](https://github.com/Akaere-NetWorks/rdap), maintained by xTom.

## Features

**Client**

- Query type detection: domains, TLDs (`rdap google`), IPv4/IPv6 addresses, abbreviated IPv4 notation (`1.1` resolves to `1.0.0.1`), CIDR prefixes, and AS numbers with or without the `AS` prefix.
- Authoritative server discovery through the IANA bootstrap registries (RFC 7484), with an in-memory cache and a built-in override table for ccTLDs that are not listed in the bootstrap.
- Two-tier domain lookups: the registry response followed by the registrar referral, so thick registrar data is shown next to the thin registry record.
- Abuse contact extraction for domains, IP networks and AS numbers.
- Output as formatted text, compact JSON or indented JSON.

**API server (`rdap --serve`)**

- RFC 7482 query paths (`/domain`, `/ip`, `/autnum`, `/nameserver`, `/entity` and searches), usable as an RDAP endpoint by any standards-compliant client. Upstream objects are returned unmodified.
- JSON endpoints for auto-detected queries, abuse contacts and batch lookups.
- Result cache with a memory budget, short-lived negative caching and request coalescing for concurrent identical queries.
- Optional bearer token authentication, DNS rebinding protection on loopback, connection and request limits.

**Outbound networking**

- SOCKS5 and HTTP(S) proxies, source address binding and interface binding (`SO_BINDTODEVICE`).
- Round-robin rotation across proxies or source addresses, with failover on connection errors, to spread queries over several egress addresses and reduce exposure to upstream rate limits.
- Per-host concurrency limits, response size limits and SSRF checks on referrals and redirects.

**Distribution**

- Packages for Debian/Ubuntu, RHEL-compatible systems and macOS (Homebrew), with a hardened systemd unit.
- Multi-arch container image (`linux/amd64`, `linux/arm64`) on a distroless base.
- Rust library crate; the API server can be excluded with `default-features = false`.

## Installation

### Debian / Ubuntu

With extrepo (Debian):

```bash
sudo apt update && sudo apt install extrepo -y
sudo extrepo enable xtom
sudo apt update && sudo apt install rdap -y
```

Or add the repository manually, as a one-line `.list` file:

```bash
sudo apt install -y ca-certificates curl gnupg
curl -fsSL https://repo.xtom.com/xtom.key | sudo gpg --dearmor -o /usr/share/keyrings/xtom.gpg
echo "deb [signed-by=/usr/share/keyrings/xtom.gpg] https://repo.xtom.com/deb stable main" | sudo tee /etc/apt/sources.list.d/xtom.list
sudo apt update && sudo apt install rdap -y
```

or in DEB822 format:

```bash
sudo apt install -y ca-certificates curl gnupg dpkg
curl -fsSL https://repo.xtom.com/xtom.key | sudo gpg --dearmor -o /usr/share/keyrings/xtom.gpg
sudo tee /etc/apt/sources.list.d/xtom.sources > /dev/null << EOF
Types: deb
URIs: https://repo.xtom.com/deb
Suites: stable
Components: main
Architectures: $(dpkg --print-architecture)
Signed-By: /usr/share/keyrings/xtom.gpg
EOF
sudo apt update && sudo apt install rdap -y
```

### RHEL / CentOS / Rocky / Alma / Fedora

```bash
sudo curl -o /etc/yum.repos.d/xtom.repo https://repo.xtom.com/rpm/xtom.repo
sudo dnf install rdap
```

The deb and rpm packages (amd64 and arm64) require glibc 2.17 or later, which covers RHEL/CentOS 7, Debian 8, Ubuntu 14.04 and newer releases.

### macOS

```bash
brew tap xtomcom/brew
brew install xtomcom/brew/rdap
```

### Container image

```bash
docker run --rm xtomcom/rdap xtom.com
```

See [Docker deployment](#docker-deployment) for running the API server.

### From source

```bash
git clone https://github.com/xtomcom/rdap.git
cd rdap
cargo build --release
sudo install -m 0755 target/release/rdap /usr/local/bin/
```

Prebuilt binaries for Linux (x86_64, aarch64, armv7, riscv64; glibc and musl), macOS and Windows are attached to each [GitHub release](https://github.com/xtomcom/rdap/releases).

## Usage

```bash
rdap xtom.com               # domain
rdap google                 # TLD (any label in the IANA TLD list)
rdap 8.8.8.8                # IPv4 address
rdap 2001:4860:4860::8888   # IPv6 address
rdap 1.1                    # abbreviated IPv4, queries 1.0.0.1
rdap 8.8.8.0/24             # CIDR prefix
rdap AS8888                 # AS number
rdap 8888                   # AS number without prefix
rdap -v xtom.com            # verbose output
```

Options:

```bash
rdap -t domain xtom.com                             # set the query type explicitly
rdap -s https://rdap.verisign.com/com/v1 xtom.com   # query this server, skip bootstrap
rdap -f json xtom.com                               # compact JSON
rdap -f json-pretty xtom.com                        # indented JSON
rdap -f json --json-source registry xtom.com        # registry object instead of registrar
rdap --no-referral xtom.com                         # do not follow the registrar referral
rdap --timeout 60 xtom.com                          # request timeout in seconds (default 30)
rdap -k xtom.com                                    # skip TLS certificate verification
rdap -u                                             # update config files (same as --update)
```

`-t` accepts `domain`, `tld`, `ip`, `autnum`, `entity`, `nameserver`, `help`, `domain-search`, `domain-search-by-nameserver`, `domain-search-by-nameserver-ip`, `nameserver-search`, `nameserver-search-by-ip`, `entity-search` and `entity-search-by-handle`. Entity lookups and searches have no bootstrap registry and require `-s`.

For domains, JSON output contains the registrar object when the referral succeeds and the registry object otherwise. `--json-source registry` always selects the registry object.

## Examples

Output is abbreviated (`…`).

### Domain

The registry (Verisign) answers first, then the registrar referral is followed:

```
$ rdap xtom.com
Abuse contact for `xtom.com` is `abuse@xtom.com`

Query from https://rdap.verisign.com/com/v1/domain/xtom.com

Domain Name: XTOM.COM
Handle: 15339482_DOMAIN_COM-VRSN
Object Class: domain
Status: client delete prohibited
Status: client transfer prohibited
Status: client update prohibited
Nameserver: NS1.XTOM.COM
…
Delegation Signed: yes
DS Key Tag: 54357
DS Algorithm: 13
DS Digest Type: 2
DS Digest: E3AEAAF9FE3FD3BA24CA96171D8758BC0619E1DFC4DFE8B45DE3AE86C696BE38
Registration: 1999-12-16T01:44:00Z
Expiration: 2032-12-16T01:44:00Z
Last Changed: 2025-11-25T14:35:53Z

Entity Handle: 3968
Role: registrar
Name: xTom GmbH
IANA Registrar ID: 3968

Query from https://rdap.xtom.com/domain/XTOM.COM

Domain Name: xtom.com
Unicode Name: xtom.com
Handle: 15339482_DOMAIN_COM-VRSN
Object Class: domain
…
Nameserver: ns1.xtom.com (188.244.98.133, 2a0d:2900:a:0:0:0:98:133)
Nameserver: ns2.xtom.com (188.244.99.133, 2a0d:2900:a:0:0:0:99:133)
…
Role: registrant
Name: Domain Administrator
Organization: xTom GmbH
Email: domains@xtom.com
…
```

### AS number

```
$ rdap AS8888
Abuse contact for `AS8888` is `abuse@xtom.com`

Query from https://rdap.db.ripe.net/autnum/8888

AS Number: AS8888
Name: XTOM
Handle: AS8888
Object Class: autnum
Status: active
Port43: whois.ripe.net
Last Changed: 2024-08-17T11:00:40Z

Entity Handle: ORG-XPL3-RIPE
Role: registrant
Name: xTom Pty Ltd
…
Entity Handle: XTAU-RIPE
Role: administrative
Role: technical
Role: abuse
Name: xTom Global NOC
Email: abuse@xtom.com
…
```

### IP address or prefix

```
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
…
```

`rdap 8.8.8.0/24` queries `https://rdap.arin.net/registry/ip/8.8.8.0/24` and returns the same network.

### TLD

```
$ rdap google
Administrative contact for `google` is `iana-contact@google.com`

Technical contact for `google` is `crr-tech@google.com`

Query from https://rdap.iana.org/domain/google

Domain Name: google
Object Class: domain
Status: active
Nameserver: ns-tld1.charlestonroadregistry.com (216.239.32.105, 2001:4860:4802:32::69)
…
Delegation Signed: yes
…
```

With `-v`, the output also includes the detected query type, links, remarks, notices and the server's `rdapConformance` values.

## API Server

`rdap --serve` runs an HTTP API server. It implements the RFC 7482 query paths, so it can be used as an RDAP endpoint by other clients, and adds JSON endpoints for auto-detected queries, abuse contacts and batch lookups. Bootstrap discovery, registrar referrals and caching are handled server-side.

```bash
# Listen on 127.0.0.1:8888 (default)
rdap --serve

# All interfaces, custom port. Always set a token in this case.
RDAP_API_TOKEN=secret rdap --serve --listen 0.0.0.0 --port 9000
RDAP_API_TOKEN=secret rdap --serve --listen :: --port 9000

# Forward every query to a fixed upstream RDAP server instead of using bootstrap
rdap -s https://rdap.example/ --serve
```

On startup the server logs its listen address and outbound routes. On SIGINT or SIGTERM it stops accepting connections and waits up to 30 seconds for in-flight requests.

### Server options

Every option can also be set through an environment variable; the command-line flag takes precedence. Empty `RDAP_*` variables are treated as unset.

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--serve` | | | Run the API server instead of a single query |
| `--listen <IP>` | `RDAP_LISTEN` | `127.0.0.1` | Listen address (`0.0.0.0` or `::` for all interfaces) |
| `--port <PORT>` | `RDAP_PORT` | `8888` | Listen port |
| `--token <TOKEN>` | `RDAP_API_TOKEN` | none | Require `Authorization: Bearer <TOKEN>` |
| `--cache-ttl <SECS>` | `RDAP_CACHE_TTL` | `300` | Result cache TTL; `0` disables caching |
| `--cache-size <MiB>` | `RDAP_CACHE_SIZE` | `256` | Approximate memory budget of the result cache |
| `--cors` | `RDAP_CORS` | off | Send CORS headers for browser clients |
| `--max-batch <N>` | `RDAP_MAX_BATCH` | `100` | Maximum queries per `POST /batch` |

`-s/--server`, `--timeout`, `-k/--insecure`, `--no-referral` and the [network options](#network-options) also apply in server mode, where `--max-per-host` defaults to `8`. Log verbosity is controlled by `RUST_LOG` (default `warn`).

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/domain/{name}` | Domain or TLD (RDAP) |
| `GET` | `/ip/{addr}`, `/ip/{addr}/{len}` | IP address or CIDR prefix (RDAP) |
| `GET` | `/autnum/{asn}` | AS number (RDAP) |
| `GET` | `/nameserver/{name}` | Nameserver (RDAP) |
| `GET` | `/entity/{handle}` | Entity (RDAP, requires `-s`) |
| `GET` | `/domains?name=`, `?nsLdhName=`, `?nsIp=` | Domain search (RDAP, requires `-s`) |
| `GET` | `/nameservers?name=`, `?ip=` | Nameserver search (RDAP, requires `-s`) |
| `GET` | `/entities?fn=`, `?handle=` | Entity search (RDAP, requires `-s`) |
| `GET` | `/help` | RDAP help object for this server |
| `GET` | `/query/{q}` | Auto-detected query, wrapped in a JSON envelope |
| `GET` | `/abuse/{q}` | Abuse contact address |
| `POST` | `/batch` | Multiple queries in one request |
| `GET` | `/healthz` | Health check, exempt from authentication |
| `GET` | `/version` | Server name and version |

RDAP endpoints respond with `Content-Type: application/rdap+json`, all others with `application/json`. Responses are compressed with gzip or Brotli when the client sends a matching `Accept-Encoding`.

#### RDAP endpoints

The response body is the upstream RDAP object, passed through without modification. For domains it is the registrar object when the referral succeeds and the registry object otherwise; `?source=registry` always selects the registry object.

```bash
curl http://127.0.0.1:8888/domain/xtom.com
curl 'http://127.0.0.1:8888/domain/xtom.com?source=registry'
curl http://127.0.0.1:8888/autnum/8888
curl http://127.0.0.1:8888/ip/8.8.8.8
curl http://127.0.0.1:8888/ip/2001:4860::/32
curl http://127.0.0.1:8888/nameserver/ns1.xtom.com
```

`GET /autnum/8888`:

```json
{
  "objectClassName": "autnum",
  "handle": "AS8888",
  "startAutnum": 8888,
  "endAutnum": 8888,
  "name": "XTOM",
  "status": ["active"],
  "port43": "whois.ripe.net",
  "entities": […],
  "rdapConformance": ["nro_rdap_profile_asn_flat_0", "rirSearch1", "autnums", "cidr0", "rdap_level_0", "nro_rdap_profile_0", "redacted"],
  …
}
```

Any RDAP client can use the server as its endpoint, including this one:

```bash
rdap -s http://127.0.0.1:8888/ xtom.com
```

Nameserver lookups are sent to the registry of the nameserver's TLD (`ns1.xtom.com` goes to the `.com` registry). Entity lookups and searches have no bootstrap registry; they require a fixed upstream (`rdap -s <url> --serve`) and return `501` otherwise.

#### `GET /query/{q}`

Applies the same type detection as the CLI: domains, TLDs, IP addresses, abbreviated IPv4, CIDR prefixes, `AS8888` and `8888`.

Query parameters:

- `type`: force a query type, same values as `-t`
- `source`: `registrar` (default, falls back to the registry), `registry` or `both`

```bash
curl http://127.0.0.1:8888/query/AS8888
curl http://127.0.0.1:8888/query/xtom.com
curl 'http://127.0.0.1:8888/query/xtom.com?source=both'
```

```json
{
  "query": "AS8888",
  "type": "autnum",
  "source": "registry",
  "registry_url": "https://rdap.db.ripe.net/autnum/8888",
  "registrar_url": null,
  "result": { "objectClassName": "autnum", "handle": "AS8888", … }
}
```

`query` is the normalized query string and `source` identifies the object in `result`. With `source=both`, `result` is replaced by `registry` and `registrar`; `registrar` is `null` if there is no referral or it failed:

```json
{
  "query": "xtom.com",
  "type": "domain",
  "source": "both",
  "registry_url": "https://rdap.verisign.com/com/v1/domain/xtom.com",
  "registrar_url": "https://rdap.xtom.com/domain/XTOM.COM",
  "registry": { "objectClassName": "domain", "ldhName": "XTOM.COM", … },
  "registrar": { "objectClassName": "domain", "ldhName": "xtom.com", … }
}
```

#### `GET /abuse/{q}`

Returns the abuse contact address. For domains the registrar object is checked first. Accepts `type` like `/query`.

```bash
curl http://127.0.0.1:8888/abuse/AS8888
curl http://127.0.0.1:8888/abuse/xtom.com
```

```json
{"query":"AS8888","type":"autnum","abuse":"abuse@xtom.com","source_url":"https://rdap.db.ripe.net/autnum/8888"}
{"query":"xtom.com","type":"domain","abuse":"abuse@xtom.com","source_url":"https://rdap.xtom.com/domain/XTOM.COM"}
```

If the object exists but has no abuse contact, the response is `200` with `"abuse": null`.

#### `POST /batch`

Request body: `{"queries": [...], "source": "...", "type": "..."}`. `source` and `type` are optional and apply to all queries. Limits are `--max-batch` queries and 64 KiB per body. Duplicate queries are resolved once, with up to 8 lookups in parallel per batch.

The response is always `200` and contains one entry per query, in request order. `data` has the same structure as a `/query` response; `error` is an RDAP error object.

```bash
curl -X POST http://127.0.0.1:8888/batch \
  -H 'Content-Type: application/json' \
  -d '{"queries": ["AS8888", "xtom.com", "xtom.invalid"]}'
```

```json
{
  "results": [
    { "query": "AS8888", "ok": true, "status": 200, "data": { "query": "AS8888", "type": "autnum", … } },
    { "query": "xtom.com", "ok": true, "status": 200, "data": { "query": "xtom.com", "type": "domain", … } },
    { "query": "xtom.invalid", "ok": false, "status": 404,
      "error": { "description": ["No RDAP server known for xtom.invalid"], "errorCode": 404, "title": "Not Found" } }
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

Errors are returned as RDAP error objects (RFC 7480, section 5.3) with the corresponding HTTP status. On RDAP endpoints they include `rdapConformance`.

```json
{"description":["No RDAP record for nonexistent-xtom.com"],"errorCode":404,"rdapConformance":["rdap_level_0"],"title":"Not Found"}
```

| Status | Meaning |
|--------|---------|
| `400` | Invalid query, `type`, `source` or batch body; upstream returned 400 or 422 |
| `401` | Missing or invalid token (with `WWW-Authenticate: Bearer`) |
| `403` | Loopback listener without a token received a non-local `Host` header (DNS rebinding protection) |
| `404` | Object not found, no RDAP server known for the query, or unknown path |
| `405` | Method not allowed |
| `413` | Too many batch queries, or request body larger than 64 KiB |
| `429` | Upstream RDAP server rate-limited the request |
| `501` | Query type requires a fixed upstream (`-s`): entities and searches |
| `502` | Upstream error, malformed upstream response, or proxy failure |
| `504` | Upstream timeout, or request exceeded 300 seconds |

### Authentication

Authentication is disabled by default. When a token is configured, every endpoint except `/healthz` requires it:

```bash
RDAP_API_TOKEN=secret rdap --serve
curl -H 'Authorization: Bearer secret' http://127.0.0.1:8888/query/xtom.com
```

Prefer `RDAP_API_TOKEN` over `--token`: command-line arguments are visible to other users through `ps`. Leading and trailing whitespace is stripped. An empty `RDAP_API_TOKEN` disables authentication and logs a warning. Tokens are compared in constant time.

Without a token on a loopback address, the server only accepts requests whose `Host` header is `localhost`, `*.localhost` or a loopback IP, which blocks DNS rebinding attacks from web pages.

### Caching

Results are cached in memory for `--cache-ttl` seconds, keyed by query type and normalized query. `/ip/8.8.8.8`, `/query/8.8.8.8` and `/abuse/8.8.8.8` share one cache entry regardless of `source`. Negative answers ("not found") and domain results with a failed registrar referral are cached for at most 60 seconds. Other errors and timeouts are not cached. Concurrent requests for the same key are coalesced into a single upstream query.

### Security

- The default listen address is `127.0.0.1`. Binding to another address without a token logs a warning but is allowed. An unauthenticated public server lets anyone send queries from your addresses and proxies, which can get them rate-limited or blocked by RDAP operators.
- The server speaks plain HTTP and has no per-client rate limiting. Exposed deployments should sit behind a TLS-terminating reverse proxy with rate limits (see [Exposing the server](#exposing-the-server)).
- CORS headers are only sent with `--cors`: any origin, `GET` and `POST`, `Authorization` and `Content-Type` request headers.
- Credentials in proxy and upstream URLs are never logged or returned to clients.
- Registrar referrals and HTTP redirects from upstream responses are only followed to public `https` URLs: no loopback, private, link-local or cloud metadata addresses, no downgrade from `https` to `http`, and at most 3 redirects. An explicitly configured upstream on a private address (`-s http://10.0.0.5/`) is not subject to these checks.
- Resource limits: request headers must arrive within 10 seconds (also between keep-alive requests); at most 1024 concurrent connections (or half the open-file limit, whichever is lower) and 64 per client IP; upstream responses up to 4 MiB; at most 32 concurrent batches across all clients. A batch is cancelled when the client disconnects.

## Network Options

These options control outbound connections to RDAP servers and to IANA for bootstrap files, both for single queries and in server mode.

| Flag | Env var | Description |
|------|---------|-------------|
| `--interface <NAME>` | `RDAP_INTERFACE` | Bind outbound connections to a network interface (e.g. `eth1`) |
| `--bind <IP>[,<IP>...]` | `RDAP_BIND` | Source address(es); multiple addresses are rotated |
| `--proxy <URL>[,<URL>...]` | `RDAP_PROXY` | Proxy URL(s); multiple proxies are rotated |
| `--max-per-host <N>` | `RDAP_MAX_PER_HOST` | Concurrent requests per upstream host (unlimited by default, 8 with `--serve`) |

```bash
rdap --interface eth1 xtom.com
rdap --bind 203.0.113.10 xtom.com
rdap --bind 2001:db8::10 AS8888

# Rotate across source addresses
rdap --serve --bind 203.0.113.10,203.0.113.11

# Proxies. Pass credentials through the environment, not the command line.
rdap --proxy socks5h://127.0.0.1:1080 xtom.com
RDAP_PROXY=socks5h://user:pass@10.0.0.1:1080,socks5h://10.0.0.2:1080 rdap --serve

rdap --serve --max-per-host 4
```

- Supported proxy schemes: `socks5://`, `socks5h://` (DNS resolved by the proxy), `http://` and `https://`, with optional `user:pass@` credentials.
- Rotation: each request starts on the next route in round-robin order. With `--proxy`, the routes are the proxies; otherwise they are the `--bind` addresses. `--interface` applies to all routes.
- Failover: if a route fails to connect (connection error, connect timeout or proxy error), the request is retried on the next route, up to 3 attempts or the number of routes. A route that has just failed is moved to the end of the rotation for a short cooldown period. Requests that received an HTTP response, or timed out while waiting for one, are not retried.
- A source address only connects to destinations of the same address family; the host's default address is never used as a fallback.
- `--bind` and `--proxy` accept repeated flags or comma-separated lists.
- `rdap --update` uses the same routes.
- `--interface` uses `SO_BINDTODEVICE` on Linux and may require `CAP_NET_RAW`. Unsupported options or invalid proxy URLs cause rdap to exit with an error at startup.

## Docker Deployment

### Image

The release workflow publishes the image for `linux/amd64` and `linux/arm64` to two registries with identical tags:

- Docker Hub: `xtomcom/rdap`
- GitHub Container Registry: `ghcr.io/xtomcom/rdap`

Tags are `latest`, `X.Y.Z`, `X.Y` and `X` (for example `1.1.0`, `1.1`, `1`). The examples below use Docker Hub; substitute `ghcr.io/xtomcom/rdap` to pull from GHCR.

The runtime base is `gcr.io/distroless/cc-debian13:nonroot`, which contains glibc, libgcc and CA certificates but no shell or package manager. The binary is built on Debian 13 (trixie), matching the runtime glibc. The container runs as `nonroot` (UID/GID 65532) with `HOME=/home/nonroot`.

The entrypoint is `rdap` and the default command is `--serve`. The image sets `RDAP_LISTEN=0.0.0.0`, so the server listens on `0.0.0.0:8888` inside the container; override `RDAP_LISTEN` or `RDAP_PORT` as needed. Configs downloaded with `rdap --update` are stored in `/home/nonroot/.config/rdap`.

### Quick start

```bash
# API server, published on localhost only
docker run -d --name rdap --restart unless-stopped \
  -p 127.0.0.1:8888:8888 -v rdap-data:/home/nonroot \
  xtomcom/rdap
curl http://127.0.0.1:8888/query/AS8888

# One-off queries: arguments replace the default --serve
docker run --rm xtomcom/rdap AS8888
docker run --rm xtomcom/rdap -f json-pretty xtom.com
```

### Compose

`compose.yaml` is the reference deployment. It pulls `xtomcom/rdap:latest` from Docker Hub; a commented `image:` line switches to `ghcr.io/xtomcom/rdap:latest`. Settings are read from `.env`:

```bash
cp .env.example .env    # edit as needed
docker compose up -d
```

`.env.example` documents the common settings: API token, cache size and TTL, proxies and source addresses, CORS, batch size and `RUST_LOG`. Every variable from [Server options](#server-options) and [Network options](#network-options) is supported. `HOST_PORT` (default `8888`) is used only by Compose and sets the host port, which is bound to `127.0.0.1`.

The service is configured with:

- a read-only root filesystem and a tmpfs on `/tmp`
- `cap_drop: [ALL]` and `no-new-privileges`
- `mem_limit: 1g` and `pids_limit: 512`; the result cache is bounded by `RDAP_CACHE_SIZE` (default 256 MiB), so increase the memory limit if you increase the cache
- `restart: unless-stopped`
- the named volume `rdap-data` mounted at `/home/nonroot`, so configs from `rdap --update` persist across restarts and upgrades

Common operations:

```bash
docker compose logs -f                        # startup output and warnings
docker compose run --rm rdap --update         # download current bootstrap/TLD configs into the volume
docker compose restart rdap                   # apply them
docker compose pull && docker compose up -d   # upgrade
docker compose down                           # stop (the volume is kept)
```

To pin a version, set the image in `compose.yaml` to `xtomcom/rdap:1.1.0` (exact release) or `xtomcom/rdap:1.1` (patch updates only), or the same tags under `ghcr.io/xtomcom/rdap`. Configs in the volume take precedence over those built into the image; run `--update` again after upgrading, or remove the volume to revert to the built-in configs.

### Exposing the server

Do not publish port 8888 on a public address without a token. To serve other hosts:

1. Set `RDAP_API_TOKEN` in `.env` (for example, generate one with `openssl rand -hex 32`).
2. Keep the port bound to `127.0.0.1` and put a TLS reverse proxy with rate limiting in front of it. If the proxy runs as a container on the same Docker network, it can reach `rdap:8888` directly and the port does not need to be published.

nginx example:

```nginx
limit_req_zone $binary_remote_addr zone=rdap:10m rate=5r/s;

server {
    listen 443 ssl;
    server_name rdap.example.net;
    # ssl_certificate / ssl_certificate_key ...

    location / {
        limit_req zone=rdap burst=20 nodelay;
        limit_req_status 429;
        proxy_pass http://127.0.0.1:8888;
    }
}
```

Clients authenticate with `Authorization: Bearer <token>`. `/healthz` remains unauthenticated for monitoring.

### Outbound proxies and source addresses

`RDAP_PROXY` works without changes, provided the proxy is reachable from the container. Inside the container `127.0.0.1` refers to the container itself; use the host's address or `host.docker.internal` (on Linux, add `extra_hosts: ["host.docker.internal:host-gateway"]`).

`RDAP_BIND` and `RDAP_INTERFACE` refer to host addresses and interfaces, which are not visible on the default bridge network. Use `network_mode: host` and remove the `ports:` section. The server then listens directly on the host, so set `RDAP_LISTEN=127.0.0.1` or a token in `.env`.

### Health checks

The image contains no shell or `curl`, so a Docker `HEALTHCHECK` command cannot run inside it. Probe `GET /healthz` externally instead, from a reverse proxy, load balancer, monitoring system or orchestrator HTTP probe. It returns `200 {"status":"ok"}` and does not require a token.

### Building the image

```bash
docker build -t rdap .

# Multi-arch build pushed to your own registry
docker buildx build --platform linux/amd64,linux/arm64 -t registry.example.com/rdap:1.1.0 --push .
```

To use a local build with Compose, uncomment `build: .` in `compose.yaml` and run `docker compose up -d --build`.

## systemd Service

The deb and rpm packages install `rdap.service`, which runs `rdap --serve` under a dynamic unprivileged user with systemd sandboxing, `MemoryMax=1G`, `TasksMax=512` and outbound access to link-local and cloud metadata addresses denied. The unit is not enabled or started on installation.

```bash
sudo editor /etc/default/rdap     # listen address, token, proxies, ...
sudo systemctl enable --now rdap
sudo systemctl restart rdap       # after configuration changes
systemctl status rdap
journalctl -u rdap -f
sudo systemctl disable --now rdap
```

`/etc/default/rdap` lists every `RDAP_*` variable, commented out with its default value, and is preserved across upgrades. Its mode is `0600` because it may contain the API token or proxy credentials. Flags without an environment variable (such as `--timeout` or `-s`) go in `RDAP_ARGS`. For other changes, use `systemctl edit rdap` rather than editing the unit file.

The service's `$HOME` is its state directory, `/var/lib/rdap`. It also reads system-wide configs from `/etc/rdap/`. To provide newer configs than the built-in ones, run `rdap --update` as any user and copy the files there:

```bash
rdap --update
sudo mkdir -p /etc/rdap
sudo cp ~/.config/rdap/config.json ~/.config/rdap/tlds.json ~/.config/rdap/tlds.txt /etc/rdap/
sudo systemctl restart rdap
```

If `--interface` fails with "Operation not permitted", grant `CAP_NET_RAW` as described in the unit file.

## Library Usage

```toml
[dependencies]
# default-features = false excludes the API server and its dependencies
rdap = { git = "https://github.com/xtomcom/rdap.git", tag = "v1.1.0", default-features = false }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Use `branch = "main"` or `rev = "<commit>"` instead of `tag` to track other revisions. More complete programs are in [`examples/`](examples) (`cargo run --example basic_query`).

### Basic query

```rust
use rdap::display::RdapDisplay;
use rdap::{QueryType, RdapClient, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    let request = RdapRequest::new(QueryType::Domain, "xtom.com");
    let result = client.query(&request).await?;
    result.display(false); // formatted text output; true = verbose
    Ok(())
}
```

`query` returns the registrar object when the referral succeeds and the registry object otherwise.

### Type detection and typed results

```rust
use rdap::{RdapClient, RdapObject, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;

    let query = "AS8888";
    let request = RdapRequest::new(RdapRequest::detect_type(query)?, query);

    match client.query(&request).await? {
        RdapObject::Domain(d) => println!("Domain: {}", d.ldh_name.unwrap_or_default()),
        RdapObject::IpNetwork(n) => println!(
            "{} - {} {}",
            n.start_address.unwrap_or_default(),
            n.end_address.unwrap_or_default(),
            n.name.unwrap_or_default()
        ),
        RdapObject::Autnum(a) => println!("AS{}", a.start_autnum.unwrap_or(0)),
        other => println!("{}", serde_json::to_string_pretty(&other)?),
    }
    Ok(())
}
```

### Registry and registrar data

```rust
use rdap::{QueryType, RdapClient, RdapObject, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    let request = RdapRequest::new(QueryType::Domain, "xtom.com");
    let result = client.query_with_referral(&request).await?;

    println!("Registry: {}", result.registry_url);
    if let Some(url) = &result.registrar_url {
        println!("Registrar: {url}");
    }

    if let RdapObject::Domain(domain) = &result.registry {
        for ns in &domain.nameservers {
            println!("Nameserver: {}", ns.ldh_name.as_deref().unwrap_or_default());
        }
        for event in &domain.events {
            println!("{}: {}", event.action, event.date);
        }
        for entity in &domain.entities {
            if let Some(vcard) = &entity.vcard {
                println!("{:?}: {}", entity.roles, vcard.name().unwrap_or_default());
            }
        }
    }
    Ok(())
}
```

### Network options, fixed server, error handling

```rust
use rdap::{NetOptions, QueryType, RdapClient, RdapError, RdapRequest};
use std::time::Duration;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = NetOptions {
        timeout: Duration::from_secs(10),
        proxies: vec!["socks5h://127.0.0.1:1080".into()],
        ..NetOptions::default()
    };
    let client = RdapClient::with_options(&opts)?.with_follow_referral(false);

    let server = Url::parse("https://rdap.verisign.com/com/v1/")?;
    let request = RdapRequest::new(QueryType::Domain, "xtom.com").with_server(server);

    match client.query(&request).await {
        Ok(object) => println!("{}", serde_json::to_string_pretty(&object)?),
        Err(RdapError::NotFound) => eprintln!("not found"),
        Err(e) => eprintln!("error: {e}"),
    }
    Ok(())
}
```

`NetOptions` also provides `insecure`, `interface`, `bind` and `max_per_host`. `RdapClient::with_timeout` overrides the request timeout on an existing client.

## Configuration

Configuration files are resolved in the following order; the first match wins:

1. `~/.config/rdap/*.local.json`: user overrides, never modified by updates
2. `~/.config/rdap/*.json`: files downloaded by `rdap --update`
3. `/etc/rdap/*.json`: system-wide configuration
4. Built-in defaults compiled into the binary

Files:

- `config.json`: IANA bootstrap URLs
- `tlds.json`: RDAP servers for TLDs missing from the IANA bootstrap
- `tlds.txt`: IANA TLD list, used to detect TLD queries such as `rdap google`
- `config.local.json`: user bootstrap configuration, replaces `config.json`
- `tlds.local.json`: user TLD overrides, merged over `tlds.json`

`rdap --update` (or `-u`) downloads `config.json` and `tlds.json` from the `main` branch of this repository and `tlds.txt` from `https://data.iana.org/TLD/tlds-alpha-by-domain.txt` into `~/.config/rdap/`. `*.local.json` files are not modified.

Example `~/.config/rdap/tlds.local.json`:

```json
{
  "example": "https://rdap.example.com/",
  "co.example": "https://rdap.co.example.com/"
}
```

## Project Layout

```
src/
├── main.rs          # CLI
├── lib.rs           # library entry point
├── client.rs        # RdapClient: queries, referrals
├── request.rs       # RdapRequest, QueryType, type detection
├── bootstrap.rs     # IANA bootstrap discovery (in-memory cache)
├── config.rs        # config loading and --update
├── net.rs           # outbound HTTP: proxies, bind/interface, rotation, per-host limits
├── display.rs       # text output
├── ip.rs            # IP normalization and CIDR handling
├── cache.rs         # disk cache helper (unused)
├── error.rs
├── models/          # RDAP objects: domain, entity, autnum, ip_network, nameserver, ...
└── server/          # API server (rdap --serve, feature `server`)
config/              # built-in config.json, tlds.json, tlds.txt
examples/            # library examples
packaging/           # systemd unit, /etc/default/rdap, Arch PKGBUILDs
Dockerfile           # container image
compose.yaml         # reference Compose deployment (settings in .env, see .env.example)
```

## Standards

- [RFC 7480](https://www.rfc-editor.org/rfc/rfc7480): HTTP Usage in the Registration Data Access Protocol
- [RFC 7482](https://www.rfc-editor.org/rfc/rfc7482): RDAP Query Format
- [RFC 7483](https://www.rfc-editor.org/rfc/rfc7483): JSON Responses for RDAP
- [RFC 7484](https://www.rfc-editor.org/rfc/rfc7484): Finding the Authoritative RDAP Service
- [RFC 6350](https://www.rfc-editor.org/rfc/rfc6350): vCard Format Specification
- [RFC 7095](https://www.rfc-editor.org/rfc/rfc7095): jCard, the JSON Format for vCard

## Contributing

Bug reports and pull requests are accepted on [GitHub](https://github.com/xtomcom/rdap). Before submitting, run `cargo fmt`, `cargo clippy -- -D warnings` and `cargo test`.

## License

MIT, see [LICENSE](LICENSE).

## Authors

xTom and Akaere Networks

## Links

- Original Go client: https://github.com/openrdap/rdap
- IANA RDAP bootstrap registries: https://data.iana.org/rdap/
- IETF REGEXT working group: https://datatracker.ietf.org/wg/regext/
