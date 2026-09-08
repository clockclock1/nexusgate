# Deployment

完整说明见根目录 [README.md](../README.md)。

## 推荐：Linux `ng` 管理脚本

一键安装简称命令，之后用 `ng` 管理服务端/客户端（systemd 开机自启、GitHub 更新、完全卸载）：

```bash
curl -fsSL https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh | sudo bash

sudo ng                 # 交互菜单
sudo ng install-server  # 公网机器
sudo ng install-client  # 内网机器
sudo ng config-client
sudo ng status
```

源码：`scripts/ng`、`scripts/install-ng.sh`。

常用命令：

| 命令 | 说明 |
|------|------|
| `install-server` / `install-client` | 安装并开机自启 |
| `start` / `stop` / `restart` | 启停 |
| `update-server` / `update-client` | 从 GitHub Release 更新 |
| `config-server` / `config-client` | 编辑配置 |
| `uninstall-server` / `uninstall-client` / `uninstall-all` | 完全卸载 |

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
