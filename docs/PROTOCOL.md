# Protocol

## Control Framing

```text
u32 big-endian length + UTF-8 JSON
```

Message types include: `HELLO`, `AUTH`, `AUTH_OK`, `REGISTER`, `REGISTER_SERVICE`,
`HEARTBEAT`, `CONNECT`, `ACCEPT`, `CLOSE`, `NAT_INFO`, `PEER_INFO`, `PATH_*`,
`CONFIG_*`, `ERROR`, `DATA_READY`.

### Data-plane wire transport

`ProtocolKind` describes the **proxied service** (`tcp` / `udp` / `http` / `https`).
`TransportKind` describes the **edge ↔ server data-plane wire**: `tcp` | `quic` | `kcp`.

- Edge `REGISTER.transports` advertises capability.
- Server `CONNECT.transport` + `CONNECT.data_port` select the wire (prefer `server.data_transport`,
  intersected with edge caps; fallback TCP).
- Public gateway remains TCP; only the relay data plane uses QUIC/KCP.
- Default ports: TCP `7001`, QUIC `7002`, KCP `7003`.

Example CONNECT:

```json
{
  "type": "CONNECT",
  "connection_id": "uuid",
  "data_token": "hex",
  "local_addr": "127.0.0.1:8000",
  "protocol": "tcp",
  "path": "relay",
  "transport": "quic",
  "data_port": 7002
}
```

## Data Handshake

Edge dials the negotiated data port (TCP / QUIC / KCP) and sends:

```text
DATA {connection_id} {data_token}\n
```

After verification, both sides switch to **raw bytes** (no custom framing).

## Security

- Edge auth: node_id + token (SHA-256 hashed at rest)
- Data bind requires data_token (not connection_id alone)
- Management API: JWT Bearer
- Audit logs for admin actions
- QUIC data plane uses ephemeral self-signed certs (client skips verify); harden for production if needed
