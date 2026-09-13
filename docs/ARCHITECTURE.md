# Architecture

NexusGate has **two independent layers**. Do not mix them.

## Layer A — Virtual overlay (management mesh only)

Purpose: connect **admin ↔ server ↔ edge** for **management traffic** (not visitor traffic).

| | |
|--|--|
| Mechanism | Virtual NIC (TUN) + UDP mesh; **STUN reflexive + bidirectional punch**, relay via Overlay Servers if punch fails |
| Roles | **Admin** and **Server** are **equal Overlay Servers** (DHCP / roster / relay). **Edge** is Overlay Node |
| Subnet | Configurable DHCP, default `10.88.0.0/16` (UDP **51820**) |
| Config | `[overlay]` in `admin.toml` / `server.toml` / `edge.toml` |

```text
[Management] Overlay TUN + P2P
  Admin (Overlay Server, e.g. 10.88.0.1) ══╗
  Server(Overlay Server, e.g. 10.88.0.2) ══╬══ equal overlay servers
  Edge  (Overlay Node,  DHCP VIP)        ══╝
```

## Layer B — Intranet penetration (port mapping)

Purpose: map **edge local service** → **server public port** → visitors hit **server public IP**.

```text
Visitor ──public──► Server:mapped_port ──tunnel──► Edge local_addr
```

Independent of overlay. Current tunnel path still uses Admin Hub data bridge (`OpenPeerPath`); that is transitional for the control/tunnel plane, not a replacement for Layer A.

## Components

| Component | Overlay role | Penetration role |
|-----------|--------------|------------------|
| **p2p-admin** | Overlay Server + Web panel | (optional transitional Hub 7100/7101) |
| **p2p-server** | Overlay Server (equal peer) | Public gateway ports only; localhost API |
| **p2p-edge** | Overlay Node | Outbound; bridges tunnel ↔ local service |

## Ports

| Where | Port | Layer | Meaning |
|-------|------|-------|---------|
| Admin | 8088 | UI | Web panel |
| Admin / Server / Edge | **51820** UDP | A | Overlay mesh |
| Admin | 7100 / 7101 | transitional | Hub control / data |
| Server | 8080 / 8443 / 8444 | B | Penetration TCP / QUIC / KCP entry |
| Server | api @ 127.0.0.1:3000 | mgmt | Localhost API |
| Edge | — | — | No public listen |

## Notes

- Overlay is **not** a substitute for port mapping; visitors never go through the VIP mesh by default.
- QUIC/KCP on the server are **penetration entry** protocols, not the edge↔server management plane.
- Long-term: move management onto overlay VIPs; keep Hub only as bootstrap if needed.

## Management path (current)

Panel `/api` preference order:

1. **Internal mesh** `MgmtReq` → server localhost API (node_id must match registry / `hub_server_id`)
2. Hub `MGMT_FORWARD` (transitional)
3. Direct HTTP to `api_upstream`

Edge service register: mesh `CtrlRegisterService` first; Hub `REGISTER_SERVICE` still sent as backup.
Visitor tunnels still use Hub data `:7101`.
