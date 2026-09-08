# syntax=docker/dockerfile:1.7
# Local multi-stage build (dev / source build). Release images use
# Dockerfile.server / Dockerfile.edge / Dockerfile.web with prebuilt binaries.
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --locked -p p2p-server -p p2p-edge

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /opt/nexusgate
COPY --from=builder /app/target/release/p2p-server /usr/local/bin/
COPY --from=builder /app/target/release/p2p-edge /usr/local/bin/
COPY server/config /opt/nexusgate/server/config
COPY client/config /opt/nexusgate/client/config
RUN mkdir -p /opt/nexusgate/data
EXPOSE 3000 7000 7001 8080
CMD ["p2p-server", "--config", "/opt/nexusgate/server/config/server.toml"]
