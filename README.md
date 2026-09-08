# NexusGate

高性能、**P2P 优先**的内网穿透与 Overlay Network：Rust Super Node / Edge 客户端 + 独立 React 管理控制台。

仓库：[clockclock1/nexusgate](https://github.com/clockclock1/nexusgate)

```text
公网 Client ──► Server Gateway ──► Control CONNECT ──► Edge ──► 内网服务
                     │                    │
                     └──── Data 1:1 绑定 ──┘
```

## 特性

- **Control / Data 分离**：控制通道只传信令，业务字节走独立 1:1 Data Connection
- **严格 1:1 映射**：`PublicConn ↔ DataConn ↔ LocalConn`，不做传统多路复用大 Tunnel
- **P2P First / Relay 兜底**：架构预留 NAT 穿越与路径切换（当前 MVP 已跑通 Relay）
- **Super Node**：Server 同时承担 Control、Data、TCP Gateway、Relay、Management API
- **管理面**：JWT + RBAC、SQLite 持久化、REST + WebSocket 实时状态
- **独立前端**：`web/` 单独构建与发布，不与二进制强耦合

## 架构与技术

### 组件

| 目录 | 组件 | 说明 |
|------|------|------|
| `server/` | `p2p-server` | 公网 Super Node + API |
| `client/` | `p2p-edge` | 内网 Edge 节点 |
| `web/` | React Admin | 管理控制台（Vite + Ant Design） |
| `shared/` | 协议库 | framing / control / dataplane / security 等 |

### 关键设计

1. Edge 与 Server 建立长期 **Control Session**（长度前缀 JSON：`HELLO` / `AUTH` / `HEARTBEAT` / `CONNECT` …）
2. 公网访问打到 Server **Gateway**，按 Route 匹配目标 Edge / 本地地址
3. Server 生成 `connection_id` + `data_token`，经 Control 下发 `CONNECT`
4. Edge 拨本地服务，并向 Server Data 口发送 `DATA {cid} {token}\n`
5. Server 绑定 Public ↔ Data，之后 **RAW TCP 双向拷贝**（`tokio::io::copy`）

### 技术栈

- **Rust**：Tokio、Axum、SQLx(SQLite)、DashMap、Tracing、JWT / bcrypt
- **前端**：React 18、TypeScript、Vite、Ant Design、ECharts、Zustand
- **部署**：`ng` 一键管理脚本、二进制发布、Docker / Compose、Systemd、Nginx、Kubernetes

默认端口：

| 端口 | 用途 |
|------|------|
| `3000` | Management API / WebSocket |
| `7000` | Control |
| `7001` | Data |
| `8080` | TCP Gateway（Route `public_port` 需与之对齐） |
| `8088` | Compose 中 Web 面板映射口（可选） |

默认管理员：`admin` / `admin123`

---

## 推荐安装（Linux `ng` 管理脚本）

生产环境推荐用简称命令 **`ng`** 管理服务端/客户端：支持安装、启停、配置、从 GitHub 更新、完全卸载，并写入 **systemd 开机自启**。

### 1. 安装管理命令

```bash
curl -fsSL https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh | sudo bash
```

安装后可随时唤起数字菜单（输入数字选择功能）：

```bash
ng                 # 自动 sudo，进入菜单
sudo ng            # 同上
```

菜单示例：

```text
======== NexusGate 管理菜单 ========
  1) 安装服务端
  2) 安装客户端
  3) 更新服务端
  ...
  0) 退出
请输入数字选择功能:
```

也可从本地仓库安装：

```bash
sudo bash scripts/install-ng.sh
```

### 2. 安装并启动服务端（公网机器）

```bash
sudo ng install-server
```

会自动：

- 从 GitHub Release 下载最新 `nexusgate-server-linux-amd64|arm64`
- 写入配置与 systemd 单元并 **enable --now**（开机自启）
- 默认监听 API `3000` / Control `7000` / Data `7001` / Gateway `8080`

### 3. 安装并启动客户端（内网机器）

```bash
sudo ng install-client
sudo ng config-client   # 填写 node_id、token、server 公网地址
sudo ng restart client
```

`node_id` / `token` 在服务端管理面板「创建节点」后获得（或调 API `/api/nodes`）。

### 4. 日常运维

```bash
sudo ng start all
sudo ng stop server
sudo ng restart client
sudo ng update-server      # 拉 GitHub 最新服务端并重启
sudo ng update-client
sudo ng config-server
sudo ng logs server
sudo ng uninstall-client   # 完全卸载客户端
sudo ng uninstall-server   # 完全卸载服务端（含数据）
sudo ng uninstall-all      # 清空全部
sudo ng self-update        # 更新 ng 脚本自身
```

| 命令 | 说明 |
|------|------|
| `install-server` / `install-client` | 安装并 systemd 开机自启 |
| `start` / `stop` / `restart [server\|client\|all]` | 启停控制 |
| `update-server` / `update-client` | 从 GitHub Release 更新二进制 |
| `config-server` / `config-client` | 编辑配置并可选重启 |
| `uninstall-server` / `uninstall-client` | **完全卸载**（配置/数据/单元） |
| `uninstall-all` | 删除 `/opt/nexusgate`、systemd、系统用户 |
| `status` / `logs` | 状态与日志 |
| `self-update` | 更新管理脚本 |

落盘位置：

```text
/usr/local/bin/ng
/opt/nexusgate/bin/p2p-server
/opt/nexusgate/bin/p2p-edge
/opt/nexusgate/server/config/server.toml
/opt/nexusgate/client/config/edge.toml
/opt/nexusgate/data/
/etc/systemd/system/nexusgate-server.service
/etc/systemd/system/nexusgate-edge.service
```

> 二进制安装/更新依赖 Release 资产名：`nexusgate-server-linux-amd64|arm64`、`nexusgate-edge-linux-*`。请先在仓库发布 Release（Actions 会自动挂载产物）。

---

## 快速开始（源码开发）

```bash
# 1) Server
cargo run -p p2p-server -- --config server/config/server.toml

# 2) Web
cd web && npm ci && npm run dev
# http://127.0.0.1:5173

# 3) 管理面板创建节点，把 node_id / token 写入 client/config/edge.toml

# 4) Edge
cargo run -p p2p-edge -- --config client/config/edge.toml
```

创建 Service（本地地址例如 `127.0.0.1:8000`）与 Route（`public_port=8080`）后访问：

```bash
curl http://127.0.0.1:8080/
```

API 示例：

```bash
TOKEN=$(curl -s -X POST http://127.0.0.1:3000/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"admin123"}' | jq -r .token)

curl -s -X POST http://127.0.0.1:3000/api/nodes \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"name":"edge-1"}'
```

---

## 使用方式

### 1. 注册 Edge

1. 启动 Server
2. 登录管理面板或调用 `/api/auth/login`
3. 创建 Node，保存返回的 `node_id` 与一次性 `token`
4. 写入 `client/config/edge.toml` 并启动 Edge
5. 面板中节点状态变为 `online`

### 2. 暴露内网服务

1. 在 Edge 所在机器准备本地服务（HTTP/TCP）
2. 创建 **Service**：协议 + `local_addr`
3. 创建 **Route**：`public_port`（当前需等于 Server `gateway_port`，默认 8080）→ Node → Service
4. 公网访问 `http://<server-ip>:8080/`（或对应 TCP 客户端）

### 3. 管理面板页面

Dashboard / Server / Nodes / Services / Routes / P2P / Connections / Traffic / Logs / Users / Settings

实时数据通过 `/ws/dashboard`、`/ws/connections`、`/ws/traffic`、`/ws/logs` 推送。

---

## 其它部署方式

### A. `ng` 脚本（推荐，见上文）

```bash
curl -fsSL https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh | sudo bash
sudo ng install-server
sudo ng install-client
```

### B. Release 二进制

发布 GitHub Release 后，Actions 会上传多平台产物：

**Server / Edge**

- `nexusgate-server-windows-amd64.exe` / `nexusgate-edge-windows-amd64.exe`
- `nexusgate-server-linux-amd64` / `nexusgate-edge-linux-amd64`
- `linux-arm64` / `macos-amd64` / `macos-arm64` 等同理

**前端（独立）**

- `nexusgate-web.zip`（静态资源，可用 Nginx 托管）

```bash
./nexusgate-server-linux-amd64 --config server/config/server.toml
./nexusgate-edge-linux-amd64 --config client/config/edge.toml
```

本地自行编译：

```bash
cargo build --release --locked -p p2p-server -p p2p-edge
cd web && npm ci && npm run build
```

### C. Docker（GHCR）

镜像：

```text
ghcr.io/clockclock1/nexusgate-server
ghcr.io/clockclock1/nexusgate-edge
ghcr.io/clockclock1/nexusgate-web
```

```bash
docker run --rm -p 3000:3000 -p 7000:7000 -p 7001:7001 -p 8080:8080 \
  -v nexusgate-data:/opt/nexusgate/data \
  ghcr.io/clockclock1/nexusgate-server:latest
```

源码本地构建（多阶段）：

```bash
docker build -t nexusgate-server .
```

### D. Docker Compose

```bash
cd deploy/compose
cp .env.example .env
# 发布后可将 NEXUSGATE_VERSION 改为 v0.1.0
docker compose up -d
```

- API / Gateway：宿主机 `3000` / `8080`
- Web：`http://127.0.0.1:8088`

### E. Systemd（手动）

更推荐直接用 `ng`（见上文「推荐安装」）。若手动部署：

```bash
sudo cp deploy/systemd/p2p-server.service /etc/systemd/system/nexusgate-server.service
sudo cp deploy/systemd/p2p-edge.service /etc/systemd/system/nexusgate-edge.service
# 按需修改 ExecStart 路径后：
sudo systemctl enable --now nexusgate-server
sudo systemctl enable --now nexusgate-edge
```

### F. Nginx 反代

参考 `deploy/nginx/p2p-network.conf` 与 `deploy/nginx/web.conf`：

- `/` → 前端静态资源
- `/api/`、`/ws/` → Server `:3000`
- Gateway / Control / Data 端口按需对公网放行

### G. Kubernetes

```bash
kubectl apply -f deploy/kubernetes/namespace.yaml
kubectl apply -f deploy/kubernetes/pvc.yaml
kubectl apply -f deploy/kubernetes/deployment.yaml
kubectl apply -f deploy/kubernetes/service.yaml
```

默认镜像 `ghcr.io/clockclock1/nexusgate-server:latest` 与 `ghcr.io/clockclock1/nexusgate-web:latest`。

---

## CI / CD（GitHub Actions）

参照 [Failover-Proxy workflows](https://github.com/clockclock1/Failover-Proxy/tree/main/.github/workflows)，在 **Publish Release** 时触发：

| Workflow | 作用 |
|----------|------|
| `Build Executables` | 多平台编译 Server + Edge，并挂到 Release |
| `Build Web Frontend` | **单独**构建前端 zip，并挂到 Release |
| `Docker Image` | 构建并推送 server / edge / web 多架构镜像到 GHCR |

---

## 开发

```bash
cargo check
cargo test
cargo clippy --workspace --all-targets
cd web && npm run build
```

更多文档：

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/PROTOCOL.md](docs/PROTOCOL.md)
- [docs/API.md](docs/API.md)
- [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)

## License

Apache-2.0
