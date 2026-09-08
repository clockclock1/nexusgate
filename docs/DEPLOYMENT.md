# Deployment

完整说明见根目录 [README.md](../README.md)。

## 二进制

```bash
cargo build --release --locked -p p2p-server -p p2p-edge
./target/release/p2p-server --config server/config/server.toml
./target/release/p2p-edge --config client/config/edge.toml
```

## Compose

```bash
cd deploy/compose
cp .env.example .env
docker compose up -d
```

## Kubernetes

```bash
kubectl apply -f deploy/kubernetes/
```

## GHCR

```text
ghcr.io/clockclock1/nexusgate-server
ghcr.io/clockclock1/nexusgate-edge
ghcr.io/clockclock1/nexusgate-web
```
