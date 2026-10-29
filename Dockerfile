# syntax=docker/dockerfile:1
#
# rdap API server image.
#
#   docker build -t rdap .
#   docker run --rm -p 127.0.0.1:8888:8888 rdap          # API server on :8888
#   docker run --rm rdap 1.1.1.1                         # one-shot query
#
# Builder and runtime share the Debian codename so the binary links against the
# same glibc it runs on. Bump both together.

ARG RUST_VERSION=1.99
ARG DEBIAN_CODENAME=trixie

FROM rust:${RUST_VERSION}-slim-${DEBIAN_CODENAME} AS build
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY config ./config
COPY src ./src

# target/ is a cache mount, so copy the binary out within the same step
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --release --locked --bin rdap \
    && install -Dm0755 target/release/rdap /out/rdap

FROM debian:${DEBIAN_CODENAME}-slim

ARG VERSION=1.1.0
LABEL org.opencontainers.image.title="rdap" \
      org.opencontainers.image.description="Modern RDAP client and local RDAP/JSON API server" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.url="https://github.com/xtomcom/rdap" \
      org.opencontainers.image.source="https://github.com/xtomcom/rdap" \
      org.opencontainers.image.documentation="https://github.com/xtomcom/rdap#api-server" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.vendor="xTom"

# rustls verifies upstream RDAP servers against the system root store
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# `rdap --update` writes $HOME/.config/rdap (bootstrap data is cached in
# memory); pre-create the dirs so a volume mounted at /home/rdap starts out
# owned by the rdap user.
RUN groupadd --system --gid 10001 rdap \
    && useradd --system --uid 10001 --gid rdap --home-dir /home/rdap --shell /usr/sbin/nologin rdap \
    && install -d -o rdap -g rdap -m 0750 /home/rdap /home/rdap/.cache /home/rdap/.cache/rdap \
       /home/rdap/.config /home/rdap/.config/rdap

COPY --from=build /out/rdap /usr/local/bin/rdap

USER 10001:10001
# Inside the container the server must listen on all interfaces; publish the
# port on 127.0.0.1 only (see compose.yaml) unless RDAP_API_TOKEN is set.
# Set via env (not CMD) so RDAP_LISTEN / RDAP_PORT can still be overridden.
ENV HOME=/home/rdap \
    RDAP_LISTEN=0.0.0.0
WORKDIR /home/rdap

EXPOSE 8888
ENTRYPOINT ["rdap"]
CMD ["--serve"]
