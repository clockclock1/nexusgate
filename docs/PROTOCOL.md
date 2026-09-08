# Protocol

## Control Framing

```text
u32 big-endian length + UTF-8 JSON
```

Message types include: `HELLO`, `AUTH`, `AUTH_OK`, `REGISTER`, `REGISTER_SERVICE`,
`HEARTBEAT`, `CONNECT`, `ACCEPT`, `CLOSE`, `NAT_INFO`, `PEER_INFO`, `PATH_*`,
`CONFIG_*`, `ERROR`, `DATA_READY`.

Example CONNECT:

```json
{
  "type": "CONNECT",
  "connection_id": "uuid",
  "data_token": "hex",
  "local_addr": "127.0.0.1:8000",
  "protocol": "tcp",
  "path": "relay"
}
```

## Data Handshake

Edge dials Server data port and sends:

```text
DATA {connection_id} {data_token}\n
```

After verification, both sides switch to **raw TCP bytes** (no custom framing).

## Security

- Edge auth: node_id + token (SHA-256 hashed at rest)
- Data bind requires data_token (not connection_id alone)
- Management API: JWT Bearer
- Audit logs for admin actions
