# NexusGate

内网穿透：外面的人访问你**公网机器上的端口**，流量转到**内网机器上的本地服务**。

仓库：[clockclock1/nexusgate](https://github.com/clockclock1/nexusgate)

三个程序：

| 程序 | 装在哪 | 干什么 |
|------|--------|--------|
| `p2p-admin` | 管理机（常和公网机同一台，也可以分开） | 网页面板；给服务端/客户端一个接入口 |
| `p2p-server` | 有公网 IP 的机器 | 对外开放映射端口；本机管理接口不对外 |
| `p2p-edge` | 内网机器 | 连管理机；把本地服务接到隧道上 |

```text
访客 ──► 服务端公网端口（默认 8080）──隧道──► 客户端本机服务（例如 127.0.0.1:8000）

管理网页 ──► 管理端 :8088
服务端 / 客户端 ──► 主动连管理端（不需要给内网机器开公网端口）
```

两件事不要混：

1. **管系统**：面板登录、改配置、客户端上报本地服务。走管理端接入口，打不通时管理端帮忙转。
2. **给人访问**：访客只打服务端映射端口，不走上面那条管理通道。

---

## 端口一览

| 端口 | 谁监听 | 协议 | 用途 | 要不要对公网开放 |
|------|--------|------|------|------------------|
| `8088` | 管理端 | TCP | 网页面板 | 你要远程打开面板就放行 |
| `7100` | 管理端 | TCP | 服务端/客户端接入（控制） | **必须**让服务端和客户端能连到 |
| `7101` | 管理端 | TCP | 访客隧道中转（直连失败时回退） | **必须**（和 7100 一样） |
| `7001` | 服务端 | TCP | 客户端直连数据口（优先于 7101） | **必须**让客户端能连到 |
| `51820` | 管理端 | UDP | 内部联络（打洞/管理消息） | 建议放行，失败会退回 7100 |
| `8080` | 服务端 | TCP | 访客入口（映射口） | 给外面的人用就放行 |
| `8443` | 服务端 | UDP | 访客入口（QUIC） | 用到再放行 |
| `8444` | 服务端 | UDP | 访客入口（KCP） | 用到再放行 |
| `3000` | 服务端 | TCP | 本机管理接口 | **不要**对公网开放，只绑 `127.0.0.1` |

默认管理员：`admin` / `admin123`（装好后立刻改掉）。

三端配置里的口令必须一致：

- `hub_token`：管理端、服务端、客户端相同
- `[overlay] token`：三端相同（内部联络用）
- 服务端 `hub_server_id` 必须等于面板「服务端节点」的 `id`（默认都是 `default`）

同一台电脑同时跑管理端和服务端时，UDP `51820` 只能被一个进程占用。服务端请改成别的端口（例如 `51821`），并把 `bootstrap` 指到管理端的 `主机:51820`。

---

## 推荐安装（Linux）

在公网机和管理机上安装管理命令：

```bash
curl -fsSL https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh | sudo bash
sudo ng
```

### 1. 管理端

```bash
sudo ng install-web
```

向导里确认：面板端口 `8088`、`hub_token`、内部联络 token、固定内部地址（默认 `10.88.0.1`）。

浏览器打开 `http://<管理机IP>:8088`。

### 2. 服务端（公网机）

```bash
sudo ng install-server
```

必填：

- `hub_host` = 管理机 IP（管理端和服务端同一台就填 `127.0.0.1`）
- `hub_token` 与管理端相同
- `hub_server_id` = `default`（和面板里的服务端 id 一致）
- 内部联络 `bootstrap` = `管理机IP:51820`
- 映射口默认 TCP `8080`

管理接口只在本机 `127.0.0.1:3000`，面板通过管理通道访问它，不必把 3000 暴露到公网。

### 3. 在面板里登记客户端

1. 登录面板
2. 「节点」里新建节点，记下返回的 **node_id** 和 **token**（token 只显示一次）
3. 新建映射：公网端口 `8080` → 这个节点 → 服务 id（要和客户端配置里的 `service_id` 相同，例如 `web`）

如果创建映射时客户端还没上报本地地址，等客户端上线后再保存一次映射（或在面板里改一下再保存）。否则隧道可能连到错误的本机地址。

### 4. 客户端（内网机）

```bash
sudo ng install-client
```

必填：

- `node_id` / `token`：面板里刚创建的
- `hub_host` = 管理机 IP
- `hub_token` 与管理端相同
- 内部联络 `bootstrap` = `管理机IP:51820`
- `[[services]]`：`service_id` 与映射一致，`local_addr` 是这台机器上的真实服务，例如 `127.0.0.1:8000`

### 5. 验证

面板「服务端节点 / Hub」里应能看到服务端和客户端都在线。

在内网机上先确认本地服务本身能开：

```bash
curl http://127.0.0.1:8000/
```

再从任意能访问公网机的地方访问映射口：

```bash
curl http://<公网机IP>:8080/
```

应得到内网服务的内容。

### 日常命令

```bash
sudo ng start all
sudo ng stop server
sudo ng restart client
sudo ng update-server
sudo ng update-client
sudo ng update-web
sudo ng show-config
sudo ng logs server
sudo ng logs client
sudo ng logs web
```

文件位置：

```text
/opt/nexusgate/bin/p2p-admin
/opt/nexusgate/bin/p2p-server
/opt/nexusgate/bin/p2p-edge
/opt/nexusgate/web/config/admin.toml
/opt/nexusgate/server/config/server.toml
/opt/nexusgate/client/config/edge.toml
```

国内下载 GitHub 可加镜像，见下文「GitHub 镜像」。

---

## 配置样例

三台分开部署时，把 `HUB` 换成管理机 IP。

**管理端 `admin.toml`**

```toml
listen = "0.0.0.0"
listen_port = 8088
api_upstream = "127.0.0.1:3000"
default_server = "default"

hub_enabled = true
hub_listen = "0.0.0.0"
hub_control_port = 7100
hub_data_port = 7101
hub_token = "换成你自己的口令"

[overlay]
enabled = true
port = 51820
listen = "0.0.0.0"
subnet_cidr = "10.88.0.0/16"
token = "换成另一组口令"
role = "server"
node_id = "admin"
fixed_vip = "10.88.0.1"
create_tun = false

[[servers]]
id = "default"
name = "公网服务端"
api_upstream = "127.0.0.1:3000"
```

`[[servers]].id` 必须等于服务端的 `hub_server_id`。`api_upstream` 只是管理通道失败时的备用直连地址；服务端 API 默认不对外，远程机器填了也连不上，正常走管理通道即可。

**服务端 `server.toml`**

```toml
listen = "0.0.0.0"
gateway_port = 8080
gateway_quic_port = 8443
gateway_kcp_port = 8444
gateway_transports = ["tcp", "quic", "kcp"]
api_port = 3000
database_url = "sqlite://data/p2p.db"
jwt_secret = "换成随机长字符串"
admin_user = "admin"
admin_password = "admin123"
enable_relay = true
enable_p2p = true

hub_host = "管理机IP"
hub_control_port = 7100
hub_data_port = 7101
hub_token = "与管理端 hub_token 相同"
hub_server_id = "default"

[overlay]
enabled = true
port = 51820
listen = "0.0.0.0"
token = "与管理端 overlay.token 相同"
role = "server"
node_id = "default"
fixed_vip = "10.88.0.2"
bootstrap = ["管理机IP:51820"]
create_tun = false
```

和管理端装在同一台 Windows/Linux 上时，把服务端 `[overlay] port` 改成 `51821`，`bootstrap` 仍指向管理端的 `127.0.0.1:51820`。

**客户端 `edge.toml`**

```toml
node_id = "面板里创建的 node_id"
token = "面板里创建的 token"
name = "家里的机器"

hub_host = "管理机IP"
hub_control_port = 7100
hub_data_port = 7101
hub_token = "与管理端 hub_token 相同"

[overlay]
enabled = true
port = 51820
listen = "0.0.0.0"
token = "与管理端 overlay.token 相同"
role = "node"
node_id = "与上面的 node_id 相同"
bootstrap = ["管理机IP:51820"]
create_tun = false

[[services]]
service_id = "web"
name = "local-web"
protocol = "tcp"
local_addr = "127.0.0.1:8000"
```

`create_tun = true` 只在 Linux 上有用，并且需要 `CAP_NET_ADMIN`。Windows 保持 `false`：管理消息不依赖系统网卡。

仓库里的现成样例：

- `admin/config/admin.toml`
- `server/config/server.toml`
- `client/config/edge.toml`

---

## Windows 本机三端联调

适合先确认程序能跑通。三个进程在同一台 Windows 上：

1. 编译：

```powershell
cd web; npm ci; npm run build; cd ..
cargo build --release --locked -p p2p-admin -p p2p-server -p p2p-edge
```

2. 准备三份配置（注意服务端 UDP 用 `51821`，避免和管理端 `51820` 冲突），然后：

```powershell
.\target\release\p2p-admin.exe --config admin.toml
.\target\release\p2p-server.exe --config server.toml
.\target\release\p2p-edge.exe --config edge.toml
```

3. 打开 `http://127.0.0.1:8088`，用 `admin` / `admin123` 登录。
4. 创建节点，把 `node_id`、`token` 写进客户端配置后重启客户端。
5. 客户端上线后再保存一次映射（公网端口 8080 → 该节点 → `service_id`）。
6. 本机起一个 `127.0.0.1:8000` 的服务，访问 `http://127.0.0.1:8080/` 应得到同样内容。

2026-09-18 在本机实测：面板登录、服务端/客户端上线、映射口返回本地页面，结果为通过。

---

## 虚拟机当管理端 + 服务端，Windows 当客户端

这是本机实测通过的摆法（Debian `192.168.206.129`，Windows 出站去连虚拟机，不碰 Windows 入站防火墙）。

1. 虚拟机上启动 `p2p-admin`（`8088` / `7100` / `7101` / UDP `51820`）和 `p2p-server`（映射口 `8080`，本机 API `127.0.0.1:3000`）。
2. 两进程在同一台虚拟机上时，服务端的内部联络端口改成 `51821`，`bootstrap` 填 `127.0.0.1:51820`。`hub_host` 填 `127.0.0.1`。
3. Windows 浏览器打开 `http://192.168.206.129:8088`，登录后创建节点。
4. Windows 的 `edge.toml`：`hub_host` 和 `bootstrap` 都填虚拟机地址 `192.168.206.129`（不要写 `127.0.0.1`）。
5. Windows 上先起本地服务（例如 `127.0.0.1:8000`），再启动 `p2p-edge`。面板里看到客户端在线后，再保存映射（公网端口 `8080` → 该节点 → `service_id`）。
6. 访问 `http://192.168.206.129:8080/`，应看到 Windows 上那个本地服务的内容。

2026-09-18 实测：面板在虚拟机、客户端在 Windows，`http://192.168.206.129:8080/` 返回了 Windows 本机页面。

反过来让虚拟机当客户端、Windows 当管理端时，还要在 Windows 防火墙放行 `7100/tcp`、`7101/tcp`、`51820/udp`。只放行 `8088` 不够。

---

## 手工运行二进制

从 [Releases](https://github.com/clockclock1/nexusgate/releases) 下载对应平台文件：

- `nexusgate-admin-*`
- `nexusgate-server-*`
- `nexusgate-edge-*`

```bash
./nexusgate-admin-linux-amd64 --config admin.toml
./nexusgate-server-linux-amd64 --config server.toml
./nexusgate-edge-linux-amd64 --config edge.toml
```

源码编译：

```bash
cd web && npm ci && npm run build && cd ..
cargo build --release --locked -p p2p-server -p p2p-edge -p p2p-admin
```

---

## 面板里做什么

| 页面 | 做什么 |
|------|--------|
| 登录 | 默认 `admin` / `admin123` |
| 服务端节点 | 当前连的是哪台服务端；可看它是否已接入管理端 |
| 节点 | 创建内网客户端，拿到 `node_id` / `token` |
| 映射 / Routes | 公网端口 → 哪个客户端 → 哪个 `service_id` |
| 服务 | 客户端上报的本地地址 |

创建节点也可以用接口（先登录拿 JWT）：

```bash
TOKEN=$(curl -s -X POST http://127.0.0.1:8088/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"admin123"}' | jq -r .token)

curl -s -X POST http://127.0.0.1:8088/api/nodes \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"name":"edge-1","node_id":"edge-1"}'
```

---

## GitHub 镜像（国内）

```bash
NG_MIRROR=https://ghproxy.net/ curl -fsSL \
  https://ghproxy.net/https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh \
  | sudo -E bash

sudo ng mirror
sudo ng test-mirror
NG_MIRROR=https://ghproxy.net/ sudo -E ng update-server
```

---

## 其它部署

Docker / Compose / systemd / Nginx / Kubernetes 仍在 `deploy/`。端口以本文「端口一览」为准：对外主要是管理端 `8088/7100/7101` 和服务端映射口 `8080`，不要再按旧文档去开放服务端 `7000/7001/3000`。

```bash
cd deploy/compose
cp .env.example .env
docker compose up -d
```

---

## 开发

```bash
cargo test -p p2p-overlay
cargo build --release --locked -p p2p-admin -p p2p-server -p p2p-edge
cd web && npm run build
```

更多细节：[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)、[docs/PROTOCOL.md](docs/PROTOCOL.md)、[docs/API.md](docs/API.md)。

## License

Apache-2.0
