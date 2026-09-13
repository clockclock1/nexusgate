# Protocol

## Dual layers

| Layer | Purpose | Transport |
|-------|---------|-------------|
| **A Overlay** | Management mesh (admin ↔ server ↔ edge VIP) | UDP JSON `:51820` |
| **B Penetration** | Visitor → server mapped port → edge local service | Gateway + Hub tunnel (transitional) |

Do not send visitor traffic over the overlay by default.

## Layer A — Overlay mesh

Default UDP port **51820**. Framing: raw UTF-8 JSON datagrams (no length prefix).

### Messages

| Type | Role |
|------|------|
| `JOIN` / `JOIN_OK` / `JOIN_DENY` | Join mesh; `JOIN_OK.observed` = reflexive addr seen by server |
| `HEARTBEAT` | Liveness; advertise preferred endpoint (reflexive if known) |
| `PEER_LIST` / `ALLOC_ANNOUNCE` | Roster + DHCP sync between equal Overlay Servers |
| `STUN_QUERY` / `STUN_REPLY` | Reflexive endpoint discovery |
| `PUNCH` / `PUNCH_ACK` | Bidirectional UDP hole punch |
| `PUNCH_INTRO` | Server-assisted candidate exchange |
| `PACKET` / `RELAY` | Encapsulated IPv4 (base64); relay via Overlay Server if no direct path |

Path preference: confirmed direct (`PUNCH_ACK`) → candidate endpoints → `RELAY`.

Admin and Server are **equal Overlay Servers**; Edge is Overlay Node. Shared secret: `overlay.token`.

## Layer B — Penetration (Hub transitional)

### Control framing (Hub / legacy control)

```text
u32 big-endian length + UTF-8 JSON
```

### Topology

```text
Browser ──► p2p-admin :8088 (panel)
                 │
                 ├─ Overlay UDP :51820  ◄── mesh ──  admin / server / edge
                 ├─ Hub control :7100  ◄── dial ──  p2p-server / p2p-edge
                 └─ Hub data    :7101  ◄── dial ──  (tunnel relay legs)

Visitor ──TCP/QUIC/KCP──► p2p-server gateway (mapped ports only)
                              │ OpenPeerPath(purpose=data, local_addr)
                              ▼
                         Admin Hub bridges server↔edge data legs
                              │
                         Edge dials local_addr
```

### Hub messages

`HELLO`, `AUTH`, `AUTH_OK`, `REGISTER`, `REGISTER_SERVICE`, `HEARTBEAT`,
`OPEN_PEER_PATH`, `PEER_PATH_OFFER`, `HUB_PEERS`, `MGMT_FORWARD`, `MGMT_FORWARD_RESULT`, …

Tunnel setup: `OPEN_PEER_PATH` with `purpose=data` and `local_addr`, then both peers dial Hub data:

```text
DATA {connection_id} {data_token}\n
```

Then raw bidirectional bytes.

### Security (penetration / Hub)

- Hub: shared `hub_token`
- Edge node token for panel identity / optional hub_token fallback
- Server management API: localhost + Hub `MGMT_FORWARD`
