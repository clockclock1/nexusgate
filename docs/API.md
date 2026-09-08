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
- `GET|PUT /api/server/config`
- `GET|POST /api/nodes`
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
- `GET|PUT /api/settings`
- `GET|POST /api/users`
- `PUT|DELETE /api/users/:id`

## WebSocket

- `/ws/dashboard`
- `/ws/logs`
- `/ws/connections`
- `/ws/traffic`

Browser WS may pass `?token=<jwt>`.
