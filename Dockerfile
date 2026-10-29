# syntax=docker/dockerfile:1
#
# rdap API server image.
#
#   docker build -t rdap .
#   docker run --rm -p 127.0.0.1:8888:8888 rdap          # API server on :8888
#   docker run --rm rdap 1.1.1.1                         # one-shot query
#
# glibc-linked binary on distroless/cc (glibc + libgcc_s for unwinding): no
# shell, no package manager. Builder and runtime are both Debian 13 (trixie),
# so the binary runs against the glibc it was built with.

ARG RUST_VERSION=1.99

FROM rust:${RUST_VERSION}-slim-trixie AS build
ARG TARGETARCH
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY config ./config
COPY src ./src

# target/ is a cache mount, so copy the binary out within the same step
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/src/target,id=rdap-target-${TARGETARCH},sharing=locked \
    cargo build --release --locked --bin rdap \
    && install -Dm0755 target/release/rdap /out/rdap

# Includes CA certificates and a nonroot user (UID/GID 65532, HOME=/home/nonroot)
FROM gcr.io/distroless/cc-debian13:nonroot

# Set by the release workflow from the git tag
ARG VERSION=dev
LABEL org.opencontainers.image.title="rdap" \
      org.opencontainers.image.description="RDAP client and RDAP/JSON API server" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.url="https://github.com/xtomcom/rdap" \
      org.opencontainers.image.source="https://github.com/xtomcom/rdap" \
      org.opencontainers.image.documentation="https://github.com/xtomcom/rdap#api-server" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.vendor="xTom"

COPY --from=build /out/rdap /usr/local/bin/rdap

# Inside the container the server must listen on all interfaces; publish the
# port on 127.0.0.1 only (see compose.yaml) unless RDAP_API_TOKEN is set.
# Set via env (not CMD) so RDAP_LISTEN / RDAP_PORT can still be overridden.
ENV RDAP_LISTEN=0.0.0.0
WORKDIR /home/nonroot

EXPOSE 8888
ENTRYPOINT ["/usr/local/bin/rdap"]
CMD ["--serve"]
