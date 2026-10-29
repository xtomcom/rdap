# rdap

An RDAP (Registration Data Access Protocol) client and API server written in Rust. It looks up domains, TLDs, IP addresses and networks, AS numbers, nameservers and entities, and prints the result as colored text or JSON.

Forked from [Akaere-NetWorks/rdap](https://github.com/Akaere-NetWorks/rdap) with additional features and fixes.

## Features

- Domain, TLD, IPv4/IPv6 address and CIDR, AS number, nameserver, entity and search queries
- Query type auto-detection, including shorthand IPs (`1.1` is `1.0.0.1`) and bare TLDs (`rdap google`)
- Server discovery through the IANA bootstrap registries, cached in memory for 24 hours
- Follows the registrar referral for domains and shows registry and registrar data
- TLD overrides for ccTLDs that are missing from the IANA bootstrap
- Abuse contact lookup for IPs and AS numbers
- Text, JSON and pretty JSON output
- `rdap --serve`: HTTP API server with RDAP-compatible paths, JSON endpoints, batch queries, a result cache and optional bearer token auth
- Outbound SOCKS5/HTTP proxies, source address or interface binding, rotation and failover
- Docker image, systemd unit, deb/rpm/Homebrew packages
- Usable as a Rust library

## Installation

### From source

```bash
git clone https://github.com/xtomcom/rdap.git
cd rdap
cargo build --release
sudo cp target/release/rdap /usr/local/bin/
```

### Debian / Ubuntu

With extrepo (Debian):

```bash
sudo apt update && sudo apt install extrepo -y
sudo extrepo enable xtom
sudo apt update && sudo apt install rdap -y
```

Or add the repository by hand, either as a one-line `.list` file:

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

### macOS

```bash
brew tap xtomcom/brew
brew install xtomcom/brew/rdap
```

### Docker

```bash
docker run --rm ghcr.io/xtomcom/rdap example.com
```

See [Docker deployment](#docker-deployment) for running the API server.

## Usage

```bash
rdap example.com            # domain
rdap google                 # TLD (any name in the IANA TLD list)
rdap com
rdap 192.0.2.1              # IPv4
rdap 2001:db8::1            # IPv6
rdap 1.1                    # shorthand IP, queries 1.0.0.1
rdap 8.8.8.0/24             # CIDR
rdap AS15169                # AS number
rdap 15169
rdap -v example.com         # verbose
```

Options:

```bash
rdap -t domain example.com                          # set the query type
rdap -s https://rdap.verisign.com/com/v1 example.com # use this server, skip bootstrap
rdap -f json example.com                            # compact JSON
rdap -f json-pretty example.com                     # pretty JSON
rdap -f json --json-source registry example.com     # registry object instead of registrar
rdap --no-referral example.com                      # don't follow the registrar referral
rdap --timeout 60 example.com                       # timeout in seconds (default 30)
rdap -k example.com                                 # skip TLS certificate verification
rdap -u                                             # update config files (same as --update)
```

`-t` accepts `domain`, `tld`, `ip`, `autnum`, `entity`, `nameserver`, `help`, `domain-search`, `domain-search-by-nameserver`, `domain-search-by-nameserver-ip`, `nameserver-search`, `nameserver-search-by-ip`, `entity-search` and `entity-search-by-handle`. Entity queries and searches have no bootstrap registry and need `-s`.

For domains, JSON output contains the registrar's object when the referral succeeded, otherwise the registry's. `--json-source registry` always returns the registry object.

## Examples

Output below is trimmed (`…`).

### Domain

```
$ rdap google.com
Query from https://rdap.verisign.com/com/v1/domain/google.com

Domain Name: GOOGLE.COM
Handle: 2138514_DOMAIN_COM-VRSN
Object Class: domain
Status: client delete prohibited
…
Nameserver: NS1.GOOGLE.COM
…
Delegation Signed: no
Registration: 1997-09-15T04:00:00Z
Expiration: 2028-09-14T04:00:00Z
Last Changed: 2019-09-09T15:39:04Z

Entity Handle: 292
Role: registrar
Name: MarkMonitor Inc.
IANA Registrar ID: 292

Query from https://rdap.markmonitor.com/rdap/domain/GOOGLE.COM

Domain Name: google.com
Handle: 2138514_DOMAIN_COM-VRSN
Object Class: domain
Port43: whois.markmonitor.com
…
Role: registrant
Name: REDACTED REGISTRANT
…
Entity Handle: 292
Role: registrar
Name: Markmonitor Inc.
…
```

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
DS Key Tag: 6125
DS Algorithm: 8
DS Digest Type: 2
DS Digest: 80F8B78D23107153578BAD3800E9543500474E5C30C29698B40A3DB23ED9DA9F
Last Changed: 2025-04-11T00:00:00+00:00
Registration: 2014-09-04T00:00:00+00:00
…
```

### IP address or CIDR

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
last changed: 2023-12-28T17:24:56-05:00
registration: 2023-12-28T17:24:33-05:00

Entity Handle: ABUSE5250-ARIN
Role: abuse
Name: Abuse
Organization: Google Inc.
Email: network-abuse@google.com
…
```

`rdap 8.8.8.0/24` queries `https://rdap.arin.net/registry/ip/8.8.8.0/24` and returns the same network.

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
Registration: 1970-01-01T00:00:00Z
Last Changed: 2024-08-17T11:00:40Z

Entity Handle: ORG-XPL3-RIPE
Role: registrant
Name: xTom Pty Ltd
…
```

`-v` additionally prints the detected query and type, plus links, remarks, notices and the server's RDAP conformance list.

## API Server

`rdap --serve` runs an HTTP API server. It answers RFC 7482 RDAP paths, so any RDAP client can use it, and adds JSON endpoints for auto-detected queries, abuse contacts and batches. Bootstrap discovery, registrar referrals and caching happen on the server.

```bash
# 127.0.0.1:8888 (default)
rdap --serve

# All interfaces, another port. Set a token when you do this.
RDAP_API_TOKEN=secret rdap --serve --listen 0.0.0.0 --port 9000
RDAP_API_TOKEN=secret rdap --serve --listen :: --port 9000

# Send every query to one upstream RDAP server instead of using bootstrap
rdap -s https://rdap.example/ --serve
```

The server prints its outbound routes and listen address on startup and shuts down gracefully on SIGINT (`Ctrl+C`) or SIGTERM, waiting up to 30 seconds for open requests.

### Server options

Each option can also be set with an environment variable. The flag wins over the variable. Empty `RDAP_*` variables count as unset.

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--serve` | | | Run the API server instead of a single query |
| `--listen <IP>` | `RDAP_LISTEN` | `127.0.0.1` | Listen address (`0.0.0.0` or `::` for all) |
| `--port <PORT>` | `RDAP_PORT` | `8888` | Listen port |
| `--token <TOKEN>` | `RDAP_API_TOKEN` | none | Require `Authorization: Bearer <TOKEN>` |
| `--cache-ttl <SECS>` | `RDAP_CACHE_TTL` | `300` | Result cache lifetime, `0` disables the cache |
| `--cache-size <MiB>` | `RDAP_CACHE_SIZE` | `256` | Approximate memory budget of the result cache |
| `--cors` | `RDAP_CORS` | off | Send CORS headers for browser clients |
| `--max-batch <N>` | `RDAP_MAX_BATCH` | `100` | Maximum queries per `POST /batch` |

`-s/--server`, `--timeout`, `-k/--insecure`, `--no-referral` and the [network options](#network-options) apply to the server too. `--max-per-host` defaults to `8` in server mode. Log verbosity is set with `RUST_LOG` (default `warn`).

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/domain/{name}` | Domain or TLD (RDAP) |
| `GET` | `/ip/{addr}`, `/ip/{addr}/{len}` | IP address or CIDR (RDAP) |
| `GET` | `/autnum/{asn}` | AS number (RDAP) |
| `GET` | `/nameserver/{name}` | Nameserver (RDAP) |
| `GET` | `/entity/{handle}` | Entity (RDAP, needs `-s`) |
| `GET` | `/domains?name=`, `?nsLdhName=`, `?nsIp=` | Domain search (RDAP, needs `-s`) |
| `GET` | `/nameservers?name=`, `?ip=` | Nameserver search (RDAP, needs `-s`) |
| `GET` | `/entities?fn=`, `?handle=` | Entity search (RDAP, needs `-s`) |
| `GET` | `/help` | RDAP help object for this server |
| `GET` | `/query/{q}` | Auto-detected query, wrapped in JSON |
| `GET` | `/abuse/{q}` | Abuse contact email |
| `POST` | `/batch` | Several queries in one request |
| `GET` | `/healthz` | Health check, never needs the token |
| `GET` | `/version` | Server name and version |

RDAP endpoints return `Content-Type: application/rdap+json`, the others `application/json`. Responses are gzip or brotli compressed when the client asks for it.

#### RDAP endpoints

The body is the upstream RDAP object, unchanged. For domains it is the registrar's object when the referral succeeded, otherwise the registry's; add `?source=registry` to always get the registry's.

```bash
curl http://127.0.0.1:8888/domain/example.com
curl 'http://127.0.0.1:8888/domain/example.com?source=registry'
curl http://127.0.0.1:8888/ip/1.1.1.1
curl http://127.0.0.1:8888/ip/2001:db8::/32
curl http://127.0.0.1:8888/autnum/13335
curl http://127.0.0.1:8888/nameserver/ns1.google.com
```

```json
{
  "cidr0_cidrs": [{ "length": 24, "v4prefix": "1.1.1.0" }],
  "country": "AU",
  "endAddress": "1.1.1.255",
  "entities": […],
  "handle": "1.1.1.0 - 1.1.1.255",
  "ipVersion": "v4",
  "name": "APNIC-LABS",
  "objectClassName": "ip network",
  "port43": "whois.apnic.net",
  "startAddress": "1.1.1.0",
  "type": "ASSIGNED PORTABLE",
  …
}
```

Point any RDAP client at the server, including this one:

```bash
rdap -s http://127.0.0.1:8888/ example.com
```

Nameservers are looked up at the registry of their own TLD (`ns1.google.com` goes to `.com`). Entities and searches have no bootstrap registry, so they need a fixed upstream (`rdap -s <url> --serve`) and return `501` without one.

#### `GET /query/{q}`

Detects the query type like the CLI: domains, TLDs (`google`), IPs, shorthand IPs (`1.1`), CIDRs, `AS13335` and `13335`.

- `type`: force a query type, same names as `-t`
- `source`: `registrar` (default, falls back to the registry), `registry` or `both`

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
  "result": { "objectClassName": "ip network", … }
}
```

`query` is the normalized query, `source` says which object `result` is. With `source=both`, `result` is replaced by `registry` and `registrar`; `registrar` is `null` when there was no referral or it failed:

```json
{
  "query": "example.com",
  "type": "domain",
  "source": "both",
  "registry_url": "https://rdap.verisign.com/com/v1/domain/example.com",
  "registrar_url": null,
  "registry": { "objectClassName": "domain", "ldhName": "EXAMPLE.COM", … },
  "registrar": null
}
```

#### `GET /abuse/{q}`

Returns the abuse contact email, checking registrar data first for domains. Accepts `type` like `/query`.

```bash
curl http://127.0.0.1:8888/abuse/1.1.1.1
```

```json
{"query":"1.1.1.1","type":"ip","abuse":"helpdesk@apnic.net","source_url":"https://rdap.apnic.net/ip/1.1.1.1"}
```

If the object exists but has no abuse email, the status is still `200` with `"abuse": null`.

#### `POST /batch`

Body: `{"queries": [...], "source": "...", "type": "..."}`; `source` and `type` are optional and apply to every query. At most `--max-batch` queries and 64 KiB. Identical queries are looked up once, up to 8 at a time per batch. The response status is `200` with one result per query, in input order; `data` has the same shape as a `/query` response and `error` is an error object.

```bash
curl -X POST http://127.0.0.1:8888/batch \
  -H 'Content-Type: application/json' \
  -d '{"queries": ["1.1.1.1", "example.com", "example.invalid"]}'
```

```json
{
  "results": [
    { "query": "1.1.1.1", "ok": true, "status": 200, "data": { "query": "1.1.1.1", "type": "ip", … } },
    { "query": "example.com", "ok": true, "status": 200, "data": { "query": "example.com", "type": "domain", … } },
    { "query": "example.invalid", "ok": false, "status": 404,
      "error": { "description": ["No RDAP server known for example.invalid"], "errorCode": 404, "title": "Not Found" } }
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

Errors are RDAP error objects (RFC 7480 section 5.3) with a matching HTTP status. On the RDAP endpoints they also carry `rdapConformance`.

```json
{"description":["No RDAP record for nonexistent-example.com"],"errorCode":404,"rdapConformance":["rdap_level_0"],"title":"Not Found"}
```

| Status | Meaning |
|--------|---------|
| `400` | Invalid query, `type`, `source` or batch body; upstream answered 400 or 422 |
| `401` | Missing or wrong token (sent with `WWW-Authenticate: Bearer`) |
| `403` | Listening on loopback without a token, and the `Host` header is not local (DNS rebinding protection) |
| `404` | Object not found, no RDAP server known for it, or unknown path |
| `405` | Wrong HTTP method |
| `413` | Batch has too many queries or the body is over 64 KiB |
| `429` | The upstream RDAP server rate-limited the request |
| `501` | Query type needs a fixed upstream (`-s`): entities, searches |
| `502` | Upstream error, unparsable or unexpected upstream response, or proxy failure |
| `504` | Upstream timeout, or the request took longer than 300 seconds |

### Authentication

Off by default. With a token set, every endpoint except `/healthz` requires it:

```bash
RDAP_API_TOKEN=secret rdap --serve
curl -H 'Authorization: Bearer secret' http://127.0.0.1:8888/query/example.com
```

Use `RDAP_API_TOKEN` rather than `--token`, since command-line arguments are visible in `ps`. Surrounding whitespace is ignored. A set but empty `RDAP_API_TOKEN` disables authentication and prints a warning.

Without a token on a loopback address, only requests whose `Host` is `localhost`, `*.localhost` or a loopback IP are accepted, so a web page can't reach the API through DNS rebinding.

### Caching

Results are cached in memory for `--cache-ttl` seconds, keyed by query type and normalized query. `/ip/1.1.1.1`, `/query/1.1.1.1` and `/abuse/1.1.1.1` share one upstream lookup whatever `source` they ask for. "Not found" answers, and domain results whose registrar referral failed, are cached for at most 60 seconds. Other errors and timeouts are not cached. Concurrent identical requests share one upstream query.

### Security

- The default listen address is `127.0.0.1`. Listening elsewhere without a token prints a warning, but the server still starts. Don't expose it without `RDAP_API_TOKEN`: anyone who can reach it can send queries from your IP addresses and proxies, which can get them rate-limited or blocked by RDAP servers.
- The server speaks plain HTTP and has no per-client rate limit. Put a TLS reverse proxy with rate limiting in front of an exposed server (see [Exposing the server](#exposing-the-server)).
- CORS headers are sent only with `--cors`: any origin, `GET` and `POST`, `Authorization` and `Content-Type` headers.
- Proxy and upstream URL credentials are never logged or returned.
- Registrar referrals and redirects found in upstream answers are followed only to public `https` URLs (no loopback, private, link-local or cloud metadata addresses, no `https` to `http` downgrade), at most 3 redirects. A self-hosted upstream on a local or private address (`-s http://10.0.0.5/`) is not restricted.
- Limits: request headers within 10 seconds (also between keep-alive requests), at most 1024 open connections (or half the open-file limit, if lower) and 64 per client IP, upstream responses up to 4 MiB, at most 32 batch lookups at a time across all clients. A batch is cancelled when its client disconnects.

## Network Options

These control how rdap connects to RDAP servers and to IANA for bootstrap files, for single queries and for `--serve`.

| Flag | Env var | Description |
|------|---------|-------------|
| `--interface <NAME>` | `RDAP_INTERFACE` | Send queries from this network interface (e.g. `eth1`) |
| `--bind <IP>[,<IP>...]` | `RDAP_BIND` | Source IP(s); several addresses rotate |
| `--proxy <URL>[,<URL>...]` | `RDAP_PROXY` | Proxy URL(s); several proxies rotate |
| `--max-per-host <N>` | `RDAP_MAX_PER_HOST` | Concurrent requests per upstream server (default unlimited, 8 with `--serve`) |

```bash
rdap --interface eth1 example.com
rdap --bind 203.0.113.10 example.com
rdap --bind 2001:db8::10 2001:db8::1

# Rotate source addresses
rdap --serve --bind 203.0.113.10,203.0.113.11

# Proxies. Put credentials in the env var, arguments are visible in `ps`.
rdap --proxy socks5h://127.0.0.1:1080 example.com
RDAP_PROXY=socks5h://user:pass@10.0.0.1:1080,socks5h://10.0.0.2:1080 rdap --serve

rdap --serve --max-per-host 4
```

- Proxy schemes: `socks5://`, `socks5h://` (the proxy resolves DNS), `http://`, `https://`, with optional `user:pass@`.
- Rotation: each request starts on the next route, round-robin. With `--proxy` the routes are the proxies, otherwise the `--bind` addresses. `--interface` applies to every route.
- Failover: when a route can't connect (connect error, connect timeout, proxy error), the request is retried on the next route, up to 3 attempts or the number of routes if fewer. A route that just failed is tried last for a short while. Requests that got an HTTP response, or timed out waiting for one, are not retried.
- A source address only connects to destinations of its own address family, so the host's default address is never used as a fallback.
- `--bind` and `--proxy` take repeated flags or comma-separated lists.
- `rdap --update` downloads through the same routes.
- `--interface` uses `SO_BINDTODEVICE` on Linux and may need `CAP_NET_RAW`. If an option isn't supported on your OS, or a proxy URL is invalid, rdap exits with an error at startup.

## Docker Deployment

### Image

`ghcr.io/xtomcom/rdap` is built for `linux/amd64` and `linux/arm64` by the release workflow, with the tags `latest`, `X.Y.Z`, `X.Y` and `X` (for example `1.1.0`, `1.1`, `1`).

The runtime image is `gcr.io/distroless/cc-debian13:nonroot` (glibc and libgcc only) with the `rdap` binary built on Debian trixie. It has no shell and no package manager. It runs as user `nonroot` (UID/GID 65532) with `HOME=/home/nonroot`. The entrypoint is `rdap` and the default command is `--serve`; the image sets `RDAP_LISTEN=0.0.0.0`, so the server listens on `0.0.0.0:8888` inside the container. Override `RDAP_LISTEN` or `RDAP_PORT` if needed. Configs written by `rdap --update` go to `/home/nonroot/.config/rdap`.

### Quick start

```bash
# API server, published on localhost only
docker run -d --name rdap --restart unless-stopped \
  -p 127.0.0.1:8888:8888 -v rdap-data:/home/nonroot \
  ghcr.io/xtomcom/rdap
curl http://127.0.0.1:8888/query/1.1.1.1

# One-shot queries: arguments replace the default --serve
docker run --rm ghcr.io/xtomcom/rdap 1.1.1.1
docker run --rm ghcr.io/xtomcom/rdap -f json-pretty example.com
```

### Compose

`compose.yaml` in this repository is the reference setup. Settings go in `.env`:

```bash
cp .env.example .env    # then edit
docker compose up -d
```

`.env.example` lists the useful variables with comments: the API token, cache size and TTL, proxies and source addresses, CORS, batch size and `RUST_LOG`. Any variable from [Server options](#server-options) or [Network options](#network-options) works. `HOST_PORT` (default `8888`) is only read by Compose and sets the host port, which is published on `127.0.0.1`.

The service runs with:

- a read-only root filesystem, with a tmpfs on `/tmp`
- `cap_drop: [ALL]` and `no-new-privileges`
- `mem_limit: 1g` and `pids_limit: 512`; the result cache is sized by `RDAP_CACHE_SIZE` (default 256 MiB), so raise the memory limit if you raise that
- `restart: unless-stopped`
- the named volume `rdap-data` at `/home/nonroot`, so configs from `rdap --update` survive restarts and upgrades

Day-to-day commands:

```bash
docker compose logs -f                    # startup lines and warnings
docker compose run --rm rdap --update     # fetch current bootstrap/TLD configs into the volume
docker compose restart rdap               # load them
docker compose pull && docker compose up -d   # upgrade
docker compose down                       # stop (the volume is kept)
```

To pin a version, change the image in `compose.yaml` to `ghcr.io/xtomcom/rdap:1.1.0` (exact) or `ghcr.io/xtomcom/rdap:1.1` (patch updates). Configs in the volume take priority over the ones built into the image, so run `--update` again after upgrading, or remove the volume to fall back to the built-in configs.

### Exposing the server

Don't publish port 8888 on a public address without a token. To serve other hosts:

1. Set `RDAP_API_TOKEN` in `.env` (generate one with `openssl rand -hex 32`).
2. Keep the port on `127.0.0.1` and put a TLS reverse proxy with rate limiting in front of it. If the proxy runs in Docker on the same network, it can reach `rdap:8888` and the port doesn't need to be published at all.

Example with nginx:

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

Clients then send `Authorization: Bearer <token>`. `/healthz` stays open for monitoring.

### Outbound proxies and source addresses

`RDAP_PROXY` works as-is. The proxy must be reachable from the container: `127.0.0.1` there is the container itself, so use the host's address or `host.docker.internal` (with `extra_hosts: ["host.docker.internal:host-gateway"]` on Linux).

`--bind`/`RDAP_BIND` and `--interface`/`RDAP_INTERFACE` refer to the host's addresses and interfaces, which the container can't see on the default bridge network. Use `network_mode: host` and remove the `ports:` section. The server then listens directly on the host, so set `RDAP_LISTEN=127.0.0.1` (or set a token) in `.env`.

### Health checks

The image has no shell or `curl`, so a Docker `HEALTHCHECK` command can't run inside it. Check `GET /healthz` from outside the container instead: from the reverse proxy or load balancer, your monitoring system, or the orchestrator's HTTP probe. It returns `200 {"status":"ok"}` and never requires the token.

### Building the image

```bash
docker build -t rdap .

# Multi-arch, pushed to your registry
docker buildx build --platform linux/amd64,linux/arm64 -t registry.example.com/rdap:1.1.0 --push .
```

To use a local build with Compose, uncomment `build: .` in `compose.yaml` and run `docker compose up -d --build`.

## systemd Service

The deb and rpm packages install `rdap.service`, which runs `rdap --serve` as a dynamic unprivileged user with systemd sandboxing, `MemoryMax=1G`, `TasksMax=512` and no access to link-local or cloud metadata addresses. It is not enabled or started on install.

```bash
sudo editor /etc/default/rdap     # listen address, token, proxies, ...
sudo systemctl enable --now rdap
sudo systemctl restart rdap       # after config changes
systemctl status rdap
journalctl -u rdap -f
sudo systemctl disable --now rdap
```

`/etc/default/rdap` lists every `RDAP_*` variable, commented out with its default, and is kept on upgrades. It is mode `0600` because it may hold the token or proxy credentials. Flags without an env var (such as `--timeout` or `-s`) go in `RDAP_ARGS`. For other changes use `systemctl edit rdap` instead of editing the unit file.

The service's `$HOME` is its state directory, `/var/lib/rdap`. It also reads system-wide configs from `/etc/rdap/`, so to give it newer configs than the built-in ones, run `rdap --update` as any user and copy the results there:

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
# default-features = false leaves out the API server and its dependencies
rdap = { git = "https://github.com/xtomcom/rdap.git", tag = "v1.1.0", default-features = false }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Use `branch = "main"` or `rev = "<commit>"` instead of `tag` to track other revisions. The repository's [`examples/`](examples) directory has more complete programs (`cargo run --example basic_query`).

### Basic query

```rust
use rdap::display::RdapDisplay;
use rdap::{QueryType, RdapClient, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;
    let request = RdapRequest::new(QueryType::Domain, "example.com");
    let result = client.query(&request).await?;
    result.display(false); // colored text output; true = verbose
    Ok(())
}
```

`query` returns the registrar's object when the referral succeeded, otherwise the registry's.

### Auto-detection and typed results

```rust
use rdap::{RdapClient, RdapObject, RdapRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RdapClient::new()?;

    let query = "8.8.8.8";
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
    let request = RdapRequest::new(QueryType::Domain, "example.com");
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

### Network options, custom server, errors

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
    let request = RdapRequest::new(QueryType::Domain, "example.com").with_server(server);

    match client.query(&request).await {
        Ok(object) => println!("{}", serde_json::to_string_pretty(&object)?),
        Err(RdapError::NotFound) => eprintln!("not found"),
        Err(e) => eprintln!("error: {e}"),
    }
    Ok(())
}
```

`NetOptions` also has `insecure`, `interface`, `bind` and `max_per_host`. `RdapClient::with_timeout` sets the query timeout on an existing client.

## Configuration

Config files are loaded in this order, first match wins:

1. `~/.config/rdap/*.local.json`: your overrides, never touched by updates
2. `~/.config/rdap/*.json`: downloaded by `rdap --update`
3. `/etc/rdap/*.json`: system-wide
4. Built-in defaults, compiled into the binary

Files:

- `config.json`: IANA bootstrap URLs
- `tlds.json`: RDAP servers for TLDs missing from the IANA bootstrap
- `tlds.txt`: IANA TLD list, used to detect TLD queries such as `rdap google`
- `config.local.json`: your bootstrap config, replaces `config.json`
- `tlds.local.json`: your TLD overrides, merged on top of `tlds.json`

`rdap --update` (or `-u`) downloads `config.json` and `tlds.json` from this repository's `main` branch and `tlds.txt` from `https://data.iana.org/TLD/tlds-alpha-by-domain.txt` into `~/.config/rdap/`. `*.local.json` files are left alone.

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
compose.yaml         # reference Compose setup (settings in .env, see .env.example)
```

## RFCs

- [RFC 7480](https://tools.ietf.org/html/rfc7480): HTTP Usage in RDAP
- [RFC 7482](https://tools.ietf.org/html/rfc7482): RDAP Query Format
- [RFC 7483](https://tools.ietf.org/html/rfc7483): JSON Responses for RDAP
- [RFC 7484](https://tools.ietf.org/html/rfc7484): Finding the Authoritative RDAP Service
- [RFC 6350](https://tools.ietf.org/html/rfc6350): vCard Format
- [RFC 7095](https://tools.ietf.org/html/rfc7095): jCard

## Contributing

Pull requests are welcome.

## License

MIT, see [LICENSE](LICENSE).

## Authors

xTom and Akaere Networks

## Links

- Original Go client: https://github.com/openrdap/rdap
- IANA RDAP bootstrap: https://data.iana.org/rdap/
- IETF REGEXT working group (RDAP): https://datatracker.ietf.org/wg/regext/
