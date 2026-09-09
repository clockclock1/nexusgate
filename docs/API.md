# Management API

Base URL: `http://host:3000`

Auth: `Authorization: Bearer <jwt>`

## Auth

- `POST /api/auth/login` `{username,password}` → `{token,user}`
- `GET /api/auth/me`
- `POST /api/auth/logout`

## Core

- `GET /api/metrics/dashboard`
- `GET /api/server/info`
- `GET|PUT /api/server/config`（写入 `server.toml`；端口变更需重启）
- `POST /api/server/restart`（进程退出，依赖 systemd `Restart=always`）
- `GET|POST /api/nodes`（创建返回一次性 `token`）
- `GET|PUT|DELETE /api/nodes/:id`
- `POST /api/nodes/:id/token`
- `GET|POST /api/services`
- `GET|PUT|DELETE /api/services/:id`
- `GET|POST /api/routes`
- `GET|PUT|DELETE /api/routes/:id`
- `GET /api/connections`
- `GET /api/traffic`
- `GET /api/logs`
- `GET /api/metrics/topology`
- `GET /api/metrics/p2p`
- `GET|PUT /api/settings`（与 `server.toml` 对齐的可管理项）
- `GET|POST /api/users`
- `PUT|DELETE /api/users/:id`

## WebSocket

- `/ws/dashboard`
- `/ws/logs`
- `/ws/connections`
- `/ws/traffic`

Browser WS may pass `?token=<jwt>`.

## Admin Panel（`p2p-admin` 本地）

Base：面板自身（如 `http://host:8088`），**不**转发到 Super Node。

- `GET /admin/api/servers` → `{ servers, default_server }`
- `POST /admin/api/servers` `{ id, name, api_upstream, make_default? }`
- `DELETE /admin/api/servers/:id`
- `PUT /admin/api/servers/default` `{ id }`
- `GET /admin/api/servers/:id/probe` 探测该上游 `/api/health`
- `GET /admin/api/hub/status` Hub 是否启用与计数
- `GET /admin/api/hub/peers` Hub 在线对端（server/edge）

反代路由：

- HTTP：`/api/*` 带请求头 `X-Nexus-Server: <id>`（或缺省用 `default_server`）
  - 若该 id 的服务端已 dial 进 Hub → `MGMT_FORWARD`（控制面管理通讯）
  - 否则回退 HTTP `api_upstream`
- WS：`/ws/*?server=<id>&token=<jwt>`

## Admin Hub 协议（Control）

Server/Edge dial `hub_control_port`（默认 7100），AUTH 时带 `role=server|edge` 与 `hub_token`。

- `HUB_PEERS`：在线名册广播
- `OPEN_PEER_PATH`：请求到对端的通道（prefer_p2p → 失败则 Relay）
- `PEER_PATH_OFFER`：双方 dial Hub `hub_data_port` 完成 1:1 中转
- `MGMT_FORWARD` / `MGMT_FORWARD_RESULT`：面板经 Hub 转发管理 HTTP
