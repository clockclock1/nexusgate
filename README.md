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
| 映射「通讯口」 | 服务端 | TCP | 客户端直连数据口（优先于 7101）；由管理端按映射下发 | **必须**让客户端能连到（装好映射后再放行对应口） |
| `51820` | 管理端 | UDP | 内部联络（打洞/管理消息） | 建议放行，失败会退回 7100 |
| 映射「访客口」 | 服务端 | TCP | 访客入口；由管理端按映射下发（常见如 `8080`） | 给外面的人用就放行 |
| `8443` | 服务端 | UDP | 访客入口（QUIC） | 用到再放行 |
| `8444` | 服务端 | UDP | 访客入口（KCP） | 用到再放行 |
| `3000` | 服务端 | TCP | 本机管理接口 | **不要**对公网开放，只绑 `127.0.0.1` |

服务端进程启动时默认**不**监听访客口/通讯口；面板保存映射后，管理端才会下发端口并让服务端绑定。

默认管理员：`admin` / `admin123`（装好后立刻改掉）。

三端配置里的口令必须一致：

- `hub_token`：管理端、服务端、客户端相同
- `[overlay] token`：三端相同（内部联络用）
- 服务端 `hub_server_id` 必须等于面板「服务端节点」的 `id`（默认都是 `default`）

同一台电脑同时跑管理端和服务端时，UDP `51820` 只能被一个进程占用。服务端请改成别的端口（例如 `51821`），并把 `bootstrap` 指到管理端的 `主机:51820`。

---

## 推荐安装（Linux）

安装靠两段脚本：`install-ng.sh` 只负责装上管理命令 `ng`；真正装三个端用 `ng install-web` / `install-server` / `install-client`。下面按屏幕上**出现的每一句提示**写「按回车用默认」还是「必须改成什么」。

建议顺序：**先管理端 → 再服务端 → 面板建节点与映射 → 最后客户端**。

向导里随时可 **Ctrl+C** 中断：未答完前**不会写配置文件**；下次再执行同一条 `install-*` / `config-*` 会**从头重新问**。

下文举例用两种常见摆法（把 IP 换成你的）：

| 摆法 | 管理端 | 服务端 | 客户端 |
|------|--------|--------|--------|
| **同机** | 公网机 `1.2.3.4` | 同一台 `1.2.3.4` | 内网机 |
| **分机** | 管理机 `10.0.0.10` | 公网机 `1.2.3.4` | 内网机 |

口令先想好两串，三端保持一致：

- `hub_token`：例如 `MyHubSecret2026`
- `overlay.token`：例如 `MyOverlaySecret2026`（若关闭 overlay 可不管）

---

### 「Overlay 引导机 IP」是什么？（旧文案：对等/引导 Overlay 主机）

这和访客穿透 **不是一回事**。

- **Hub（`hub_host` / 7100）**：服务端、客户端连管理端，用来上报节点、下发映射、走隧道控制。**穿透主路径靠这个。**
- **Overlay（虚拟网）**：可选的内部联络网（打洞/管理消息）。启用后，服务端和客户端需要知道「去哪台机器加入这张虚拟网」。

「Overlay 引导机 IP」= **跑着管理端 overlay 的那台机器的 IP**（只填主机名或 IP，**不要带端口**）。脚本会自动写成配置里的：

```toml
bootstrap = ["这台IP:51820"]   # 端口用你填的 overlay.port
```

| 你在装谁 | 怎么填 |
|----------|--------|
| **管理端** `install-web` | 管理端自己就是 Overlay 的「房主」，**直接回车留空**（不要写自己的 IP） |
| **服务端** `install-server` | 同机装管理端 → 填 `127.0.0.1`；管理端在别的机器 → 填管理机 IP（如 `10.0.0.10`） |
| **客户端** `install-client` | 填**管理机 IP**（内网要能访问到；同机极罕见才用 `127.0.0.1`） |

注意：

- 填的是 **IP/主机名**，不是 `IP:端口`。端口在上一项 `overlay.port` 里填。
- 同机既跑管理端又跑服务端时：管理端 overlay 用 `51820`；服务端 overlay 端口请改成 `51821`（避免抢同一个 UDP 口），但引导机仍填 `127.0.0.1`（去连管理端的 `51820`）。
- 访客打公网端口 **不依赖** overlay。不会配就先在三端都选「不启用 overlay」，只把 Hub 配通也能穿透。

---

### 0. 先装管理命令 `ng`

在**每一台**要装组件的 Linux 上执行一次：

```bash
curl -fsSL https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh | sudo bash
```

国内慢时：

```bash
NG_MIRROR=https://ghproxy.net/ curl -fsSL \
  https://ghproxy.net/https://raw.githubusercontent.com/clockclock1/nexusgate/main/scripts/install-ng.sh \
  | sudo -E bash
```

这一步没有交互问答，脚本自动：检查 root → 必要时装 curl → 下载并校验 `ng` → 装到 `/usr/local/bin/ng`。之后：

```bash
sudo ng          # 数字菜单
sudo ng help
```

---

### 1. 管理端：`sudo ng install-web`

在管理机执行。二进制下载、建用户、写 systemd 都是自动的；下面只列**要你回答的提示**。

```bash
sudo ng install-web
```

屏上出现什么 → 你怎么填：

| # | 屏上提示（大意） | 直接回车？ | 建议填写 |
|---|------------------|------------|----------|
| 1 | `监听地址 listen [0.0.0.0]` | 可以 | 要远程打开面板就保持 `0.0.0.0`；只本机浏览器访问可改 `127.0.0.1` |
| 2 | `面板端口 listen_port [8088]` | 可以 | 保持 `8088`，防火墙放行该口 |
| 3 | `API 反代地址 api_upstream [127.0.0.1:3000]` | 可以 | **同机**稍后装服务端：保持 `127.0.0.1:3000`。**分机**且服务端 API 只在服务端本机时：仍先填 `127.0.0.1:3000`（正常走 Hub 管理通道，不靠直连 3000） |
| 4 | `hub_token [change-me-hub-token]` | **不要原样用** | 改成你的口令，如 `MyHubSecret2026`（服务端、客户端必须相同） |
| 5 | `启用 overlay [Y/n]` | 可选 | 要虚拟网：`Y`；搞不清 / 只要穿透：`n`（选 n 则跳过后面 overlay 项） |
| 6 | `overlay.port [51820]` | 可以 | 保持 `51820`（管理端占用这个 UDP） |
| 7 | `overlay.subnet_cidr [10.88.0.0/16]` | 可以 | 保持默认 |
| 8 | `overlay.token（三端一致）` | **不要原样用** | 改成 `MyOverlaySecret2026`，三端相同 |
| 9 | `overlay.node_id [admin]` | 可以 | 保持 `admin` |
| 10 | `overlay.fixed_vip [10.88.0.1]` | 可以 | 保持 `10.88.0.1`（管理端在虚拟网里的固定地址） |
| 11 | `Overlay 引导机 IP… [直接回车=留空]` | **直接回车** | 管理端是房主，**留空**（不要填本机 IP）。回车后进入下一项，不会反复问 |
| 12 | `创建系统 TUN… [y/N]` | 回车 = N | 保持 N（不建系统网卡） |
| 13 | `…立即启动管理面板? [Y/n]` | 回车 = Y | 选 Y 立刻启动 |

装完访问：`http://<管理机IP>:8088`。  
登录账号是**服务端**向导里的 `admin` / `admin123`（服务端还没装时面板能开，登录要等服务端起来）。

防火墙放行：`8088/tcp`、`7100/tcp`、`7101/tcp`；启用了 overlay 再放行 `51820/udp`。

配置文件：`/opt/nexusgate/web/config/admin.toml`。

---

### 2. 服务端：`sudo ng install-server`

在公网机（或要给人访问的那台）执行。

```bash
sudo ng install-server
```

**记住**：进程起来时还不会听访客口；等面板建好映射、管理端下发端口后才会绑定。

| # | 屏上提示（大意） | 直接回车？ | 建议填写 |
|---|------------------|------------|----------|
| 1 | `监听地址 listen [0.0.0.0]` | 可以 | 保持 `0.0.0.0` |
| 2 | `穿透 TCP 端口 gateway_port [8080]` | 可以 | 面板建映射时常用 `8080`；也可以后在面板改 |
| 3 | `穿透 QUIC 端口 [8443]` | 可以 | 暂不用 QUIC 也回车 |
| 4 | `穿透 KCP 端口 [8444]` | 可以 | 同上 |
| 5 | `本机 API 端口 api_port [3000]` | 可以 | 保持 `3000`（只监听 127.0.0.1，不要对公网开） |
| 6 | `数据库 URL …` | 可以 | 保持默认 sqlite 路径 |
| 7 | `JWT 密钥 jwt_secret […]` | 可以 | 脚本已随机生成，回车即可；生产可再改长密钥 |
| 8 | `JWT 有效期秒数 [86400]` | 可以 | 一天，回车即可 |
| 9 | `管理员用户名 admin_user [admin]` | 可以 | 面板登录名 |
| 10 | `管理员密码 admin_password [admin123]` | **建议改** | 改成自己的密码，浏览器登录面板用这个 |
| 11 | `最大连接数 [100000]` | 可以 | 回车 |
| 12 | `启用中继 enable_relay [Y/n]` | 回车 = Y | 保持 Y |
| 13 | `启用 P2P enable_p2p [Y/n]` | 回车 = Y | 保持 Y |
| 14 | `hub_host [127.0.0.1]` | 看摆法 | **同机**：`127.0.0.1`。**分机**：填管理机 IP，如 `10.0.0.10` |
| 15 | `hub_control_port [7100]` | 可以 | 与管理端一致，回车 |
| 16 | `hub_data_port [7101]` | 可以 | 与管理端一致，回车 |
| 17 | `hub_token […]` | **必须对齐** | 填管理端同一串，如 `MyHubSecret2026` |
| 18 | `hub_server_id [default]` | 可以 | 保持 `default`（与管理端 `[[servers]] id` 一致） |
| 19 | `启用 overlay [Y/n]` | 与管理端一致 | 管理端开了就 Y；管理端关了就 n |
| 20 | `overlay.port [51820]` | **同机要改** | **同机**：改成 `51821`（别和管理端抢 51820）。**分机**：可保持 `51820` |
| 21 | `overlay.subnet_cidr` | 可以 | 与管理端相同，回车 |
| 22 | `overlay.token` | **必须对齐** | 与管理端相同，如 `MyOverlaySecret2026` |
| 23 | `overlay.node_id` | 可以 | 默认会用 `default`（与 hub_server_id 一致），回车 |
| 24 | `overlay.fixed_vip [10.88.0.2]` | 可以 | 保持 `10.88.0.2` |
| 25 | `Overlay 引导机 IP […]` | 看摆法 | **同机**：回车用默认 `127.0.0.1`（或手填）。**分机**：填管理机 IP `10.0.0.10`。只填 IP，不要 `:51820` |
| 26 | `创建系统 TUN [y/N]` | 回车 = N | 保持 N |
| 27 | `…立即启动服务端? [Y/n]` | 回车 = Y | 选 Y |

配置文件：`/opt/nexusgate/server/config/server.toml`。  
装完后面板 Hub 里应能看到服务端 `default` 在线。

---

### 3. 面板里建节点和映射（装客户端前）

1. 打开 `http://<管理机IP>:8088`，用服务端的用户名/密码登录。
2. 「节点」→ 新建 → **马上抄下 `node_id` 和 `token`**（token 往往只显示一次）。
3. 「映射」→ 新建：
   - 访客端口：例如 `8080`（给人访问的公网口）
   - 通讯端口：可**留空**让管理端自动分配
   - 节点：刚建的那个
   - `service_id`：例如 `web`（下面客户端本地服务要写同一个）
4. 客户端还没上线就建了映射的话：等客户端上线后再保存一次映射。

---

### 4. 客户端：`sudo ng install-client`

在内网业务机执行。

```bash
sudo ng install-client
```

| # | 屏上提示（大意） | 直接回车？ | 建议填写 |
|---|------------------|------------|----------|
| 1 | `节点 ID node_id […]` | **不要用默认** | 面板里抄的 `node_id` |
| 2 | `节点 Token token` | **必填** | 面板里抄的 `token` |
| 3 | `节点显示名 name […]` | 随意 | 例如 `家里的NAS` |
| 4 | `hub_host [127.0.0.1]` | **几乎都要改** | 填**管理机 IP**（同机摆法才用 `127.0.0.1`）。例：`1.2.3.4` 或 `10.0.0.10` |
| 5 | `hub_control_port [7100]` | 可以 | 回车 |
| 6 | `hub_data_port [7101]` | 可以 | 回车 |
| 7 | `hub_token（空则回退用节点 token）` | **建议填** | 与管理端相同的 `MyHubSecret2026`；真留空则脚本用节点 token |
| 8 | `启用 overlay [Y/n]` | 与管理端一致 | 管理端开了就 Y |
| 9 | `overlay.port [51820]` | 可以 | 客户端自己的 UDP 口，一般保持 `51820`（和引导机上的管理端端口可以相同，不在同一台机） |
| 10 | `overlay.subnet_cidr` | 可以 | 与管理端相同 |
| 11 | `overlay.token` | **必须对齐** | 与管理端相同 |
| 12 | `overlay.node_id` | 可以 | 默认已是上面的 node_id，回车 |
| 13 | `Overlay 引导机 IP […]` | **填管理机** | 与 `hub_host` 相同：管理机 IP。只填 IP，不要带端口 |
| 14 | `创建系统 TUN [y/N]` | 回车 = N | 保持 N |
| 15 | `service_id [web]` | 对齐映射 | 与面板映射的 service_id 一致，常用 `web` |
| 16 | `name [local-web]` | 可以 | 回车或改显示名 |
| 17 | `protocol [tcp]` | 可以 | 回车 |
| 18 | `local_addr (本机回源) [127.0.0.1:8000]` | 按实情 | 改成这台机真实服务，如 `127.0.0.1:8000`；先本机 `curl` 能通 |
| 19 | `继续添加下一个本地服务? [y/N]` | 回车 = N | 只要一个服务就 N；多个就 Y 再填一组 |
| 20 | `…立即启动客户端? [Y/n]` | 回车 = Y | 选 Y |

配置文件：`/opt/nexusgate/client/config/edge.toml`。

---

### 5. 验证

1. 面板 Hub：服务端、客户端都在线。  
2. 服务端 `ss -lnt`：出现映射的访客口和通讯口。  
3. 内网机：`curl http://127.0.0.1:8000/`（换成你的 local_addr）。  
4. 外网：`curl http://<公网机IP>:<访客端口>/` 应等于内网内容。

---

### 日常命令

```bash
sudo ng start all
sudo ng stop server
sudo ng restart client
sudo ng update-server
sudo ng update-client
sudo ng update-web
sudo ng config-server          # 重新走服务端向导
sudo ng config-client
sudo ng config-web
sudo ng show-config
sudo ng logs server
sudo ng logs client -n 100
sudo ng logs web
sudo ng mirror
sudo ng test-mirror
```

文件位置：

```text
/opt/nexusgate/bin/p2p-admin | p2p-server | p2p-edge
/opt/nexusgate/web/config/admin.toml
/opt/nexusgate/server/config/server.toml
/opt/nexusgate/client/config/edge.toml
/etc/systemd/system/nexusgate-{web,server,edge}.service
```

卸载：

```bash
sudo ng uninstall-client
sudo ng uninstall-server
sudo ng uninstall-web
sudo ng uninstall-all          # 需输入 YES
```

国内下载见下文「GitHub 镜像」。

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
