# Build stage
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p p2p-server -p p2p-edge

FROM node:22-bookworm AS web
WORKDIR /web
COPY web/package.json web/package-lock.json* ./
RUN npm install
COPY web/ ./
RUN npm run build

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /opt/p2p
COPY --from=builder /app/target/release/p2p-server /usr/local/bin/
COPY --from=builder /app/target/release/p2p-edge /usr/local/bin/
COPY --from=web /web/dist /opt/p2p/web
COPY server/config /opt/p2p/server/config
COPY client/config /opt/p2p/client/config
RUN mkdir -p /opt/p2p/data
EXPOSE 3000 7000 7001 8080
CMD ["p2p-server", "--config", "/opt/p2p/server/config/server.toml"]
