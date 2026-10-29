# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build the project
cargo build

# Build release version (optimized, stripped binary)
cargo build --release

# Run tests
cargo test

# Run only library tests
cargo test --lib

# Run a specific test
cargo test test_detect_type

# Run the CLI directly
cargo run -- example.com

# Check without building
cargo check

# Run an example
cargo run --example basic_query

# Build without the API server
cargo build --no-default-features

# Container image / local API server
docker build -t rdap .
docker compose up -d --build
```

## Architecture

This is an RDAP (Registration Data Access Protocol) client written in Rust, implementing RFCs 7480-7484. It provides both a CLI tool and a library.

### Core Components

- **`src/main.rs`** - CLI entry point using clap for argument parsing, includes `rdap update` subcommand
- **`src/lib.rs`** - Library entry point, re-exports public API
- **`src/client.rs`** - `RdapClient` - main client that orchestrates queries, handles HTTP requests, parses responses, and follows registrar referrals for multi-layer RDAP queries
- **`src/request.rs`** - `RdapRequest` and `QueryType` - request building and query type detection
- **`src/bootstrap.rs`** - `BootstrapClient` - IANA bootstrap service discovery to find authoritative RDAP servers, with TLD override support
- **`src/config.rs`** - Configuration management with priority loading (local > user > system > builtin)
- **`src/display.rs`** - `RdapDisplay` and `RdapDisplayWithQuery` traits - colored terminal output formatting, abuse contact display
- **`src/cache.rs`** - Disk cache helper (`~/.cache/rdap/`; currently unused — bootstrap registries are cached in memory in `bootstrap.rs`)
- **`src/ip.rs`** - IP address utilities: normalization (shorthand → standard), CIDR detection/parsing
- **`src/models/`** - RDAP data models (Domain, Entity, Autnum, IpNetwork, Nameserver, etc.)
- **`src/net.rs`** - `NetOptions` / `HttpPool` - all outbound HTTP (bootstrap, RDAP, referrals): proxies, `--bind` / `--interface`, round-robin route rotation with failover, per-host concurrency limit, body size limit
- **`src/server/`** - `rdap --serve` HTTP API server (axum, behind the default `server` feature): RFC 7482 RDAP paths plus `/query`, `/abuse`, `/batch`, `/healthz`, `/version`; bearer token auth, moka result cache

### Packaging

- **`Dockerfile`** - multi-stage build (`rust:<ver>-slim-trixie` → `gcr.io/distroless/cc-debian13:nonroot`, both Debian 13 so glibc matches; no shell in the runtime image); runs `rdap --serve` with `RDAP_LISTEN=0.0.0.0` as `nonroot` (UID 65532, `HOME=/home/nonroot`)
- **`compose.yaml`** / **`.env.example`** - reference Compose setup using `xtomcom/rdap` (Docker Hub; GHCR alternative commented) (127.0.0.1:8888, read-only root fs, `$HOME` volume); settings in `.env`
- Release workflow publishes multi-arch (amd64 + arm64, native runners) images to Docker Hub `xtomcom/rdap` and `ghcr.io/xtomcom/rdap` (secret `DOCKERHUB_TOKEN`)
- **`packaging/rdap.service`** - hardened systemd unit (`DynamicUser`, `HOME=/var/lib/rdap` via `StateDirectory`), installed by deb and rpm but never enabled/started on install
- **`packaging/rdap.env`** - installed as `/etc/default/rdap` (conffile / `%config(noreplace)`, mode 600), commented `RDAP_*` defaults; keep in sync with the env vars in `src/main.rs`
- **`packaging/arch/`** - Arch Linux PKGBUILDs
- deb/rpm metadata lives in `Cargo.toml` (`[package.metadata.deb]`, `[package.metadata.generate-rpm]`); release builds them with `cargo zigbuild` (glibc 2.17) + `cargo deb --no-build` / `cargo generate-rpm`

### Configuration System

```
Priority (highest to lowest):
1. ~/.config/rdap/*.local.json  - User local overrides (never updated)
2. ~/.config/rdap/*.json        - Downloaded configs
3. /etc/rdap/*.json             - System configs
4. Built-in defaults            - Embedded in binary via include_str!

Files:
- config.json      - Bootstrap URLs for IANA RDAP
- tlds.json        - TLD overrides for ccTLDs not in IANA bootstrap
- *.local.json     - User overrides (merged, not replaced)
```

Config files in `config/` directory are embedded into the binary at compile time.

### Query Flow

1. User provides query string (domain, TLD, IP, CIDR, AS number)
2. `RdapRequest::detect_type_with_tld_check()` auto-detects query type:
   - Pure numbers → `QueryType::Autnum` (AS number)
   - Checks if single word matches IANA TLD list (from `tlds.txt`) → `QueryType::Tld`
   - IP-like patterns → `QueryType::Ip`
   - Otherwise → `QueryType::Domain`
3. For IP queries: `ip::normalize_ip()` normalizes shorthand IPs (1.1 → 1.0.0.1)
4. For domains: Check TLD overrides from `tlds.json` first, then IANA bootstrap
5. `BootstrapClient` fetches IANA bootstrap registry to find authoritative RDAP server
6. `RdapClient` sends HTTP request with `Accept: application/rdap+json`
7. Response is parsed into appropriate `RdapObject` variant based on `objectClassName`
8. For domain queries: Follow registrar referral link for multi-layer RDAP data
9. Result is displayed via `RdapDisplay` trait (with abuse contact for IP/ASN) or serialized to JSON

### Key Types

- `RdapObject` - enum of all possible RDAP response types (Domain, Entity, IpNetwork, Autnum, etc.)
- `QueryType` - enum for query types (Domain, Tld, Ip, Autnum, Entity, Nameserver, searches)
- `RdapQueryResult` - Result with registry and optional registrar data, plus server URLs
- `VCard` - jCard/vCard parsing for contact information in entities
- `Config` - Configuration with bootstrap URLs and cache settings
- `TldOverrides` - HashMap of TLD -> RDAP server URL
- `TldList` - IANA TLD list for detecting TLD queries (e.g., `rdap google` → query .google TLD)

### CLI Options

- `rdap <query>` - Main query command
- `rdap --update` / `rdap -u` - Update config files from GitHub
- `rdap --serve` - Run the API server (`--listen`, `--port`, `--token`, `--cache-ttl`, `--cors`, `--max-batch`)
- `--interface`, `--bind`, `--proxy`, `--max-per-host` - Outbound network options (CLI and server)
- Every server/network flag has an `RDAP_*` env var (e.g. `RDAP_API_TOKEN`, `RDAP_PROXY`)

### Dependencies

- `tokio` + `reqwest` - async HTTP client
- `serde` + `serde_json` - JSON serialization
- `clap` - CLI parsing with subcommands
- `colored` + `comfy-table` - terminal output formatting
- `ipnet` - IP network/CIDR parsing and matching
