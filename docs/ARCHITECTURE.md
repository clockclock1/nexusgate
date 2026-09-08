# Architecture

## Components

| Component | Role |
|-----------|------|
| Server | Super Node: control coordinator, data bind, TCP gateway, relay, management API |
| Edge | Intranet client: maintain control session, accept CONNECT, dial data + local service |
| Web Panel | React console over REST/WebSocket |

## Control vs Data

- **Control Channel**: long-lived TCP session, length-prefixed JSON only.
- **Data Connection**: one public connection ↔ one data connection ↔ one local connection.
- Business bytes never multiplex through the control channel.

## Connection Flow (TCP Relay MVP)

```text
Client --TCP--> Server Gateway
                  │ create connection_id + data_token
                  │ Control CONNECT --> Edge
Edge <--TCP data--> Server Data Port (handshake DATA cid token)
Edge <--TCP-------> Local Service
Server binds PublicStream <-> DataStream
then raw bidirectional copy
```

## P2P Path (scaffolded)

```text
Server distributes PEER_INFO
Edges probe candidates / hole punch
Success -> direct data path
Failure -> relay fallback
```

## Runtime State vs Persistence

- Hot path: DashMap / atomics / channels (online nodes, pending binds, active conns, metrics)
- SQLite: users, nodes, services, routes, settings, audit logs

## Extension Hooks

Transport traits reserve QUIC/TLS. Dataplane modules reserve UDP/HTTP/HTTPS/splice/io_uring.
