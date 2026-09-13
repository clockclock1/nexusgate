//! Overlay node runtime: equal servers (admin/server) + mesh nodes (edge).
//! Includes STUN/punch and a management channel (HTTP-ish + service register).

use crate::config::{OverlayConfig, OverlayRole};
use crate::dhcp::DhcpPool;
use crate::frame::{decode_msg, encode_msg, OverlayMsg, PeerInfoMsg};
use crate::mgmt::{
    MgmtRequest, MgmtResponse, RegisterServiceEvent, MGMT_DEFAULT_TIMEOUT, MGMT_MAX_BODY,
};
use crate::peer::{PeerEntry, PeerTable};
use crate::punch::{
    prefer_advertise, send_punch, send_punch_ack, send_stun_query,
};
use crate::tun_dev::TunDevice;
use dashmap::DashMap;
use futures::future::BoxFuture;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::{oneshot, Mutex, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

type MgmtHandler =
    Arc<dyn Fn(MgmtRequest) -> BoxFuture<'static, MgmtResponse> + Send + Sync + 'static>;
type RegisterHandler =
    Arc<dyn Fn(RegisterServiceEvent) -> BoxFuture<'static, ()> + Send + Sync + 'static>;

/// Fire-and-forget spawn (logs when ready). Prefer [`start_overlay`] when you need the handle.
pub fn spawn_overlay(cfg: OverlayConfig) -> Option<tokio::task::JoinHandle<()>> {
    if !cfg.enabled {
        return None;
    }
    if cfg.node_id.trim().is_empty() {
        tracing::warn!("overlay enabled but node_id empty; skip");
        return None;
    }
    Some(tokio::spawn(async move {
        match start_overlay(cfg).await {
            Ok(h) => {
                tracing::info!(vip = %h.vip(), peers = h.peers().list().len(), "overlay mesh ready");
                // keep mesh alive
                std::future::pending::<()>().await;
            }
            Err(e) => {
                tracing::warn!(error = %e, "overlay failed (management mesh); continuing without VIP");
            }
        }
    }))
}

/// Live mesh handle (clone freely). Management traffic can use [`OverlayMesh::mgmt_forward`].
#[derive(Clone)]
pub struct OverlayMesh {
    state: Arc<OverlayState>,
}

/// Backward-compatible alias.
pub type OverlayHandle = OverlayMesh;

impl OverlayMesh {
    pub fn vip(&self) -> Ipv4Addr {
        self.state
            .local_vip
            .try_lock()
            .map(|g| *g)
            .unwrap_or(Ipv4Addr::UNSPECIFIED)
    }

    pub fn node_id(&self) -> &str {
        &self.state.cfg.node_id
    }

    pub fn peers(&self) -> &PeerTable {
        &self.state.peers
    }

    pub fn is_peer_online(&self, node_id: &str) -> bool {
        self.state
            .peers
            .get_by_id(node_id)
            .map(|p| !p.all_targets().is_empty() || p.direct_ok)
            .unwrap_or(false)
    }

    /// Install handler for inbound management HTTP (typically the penetration server).
    pub async fn set_mgmt_handler<F, Fut>(&self, f: F)
    where
        F: Fn(MgmtRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = MgmtResponse> + Send + 'static,
    {
        let f = Arc::new(f);
        let boxed: MgmtHandler = Arc::new(move |req| {
            let f = f.clone();
            Box::pin(async move { f(req).await })
        });
        *self.state.mgmt_handler.write().await = Some(boxed);
    }

    /// Install handler for edge service registration over the mesh.
    pub async fn set_register_handler<F, Fut>(&self, f: F)
    where
        F: Fn(RegisterServiceEvent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let f = Arc::new(f);
        let boxed: RegisterHandler = Arc::new(move |ev| {
            let f = f.clone();
            Box::pin(async move { f(ev).await })
        });
        *self.state.register_handler.write().await = Some(boxed);
    }

    /// Panel → server management call over the internal mesh.
    pub async fn mgmt_forward(
        &self,
        to_node_id: &str,
        method: &str,
        path: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        timeout: Option<Duration>,
    ) -> anyhow::Result<MgmtResponse> {
        if body.len() > MGMT_MAX_BODY {
            anyhow::bail!("mgmt body too large (max {} bytes)", MGMT_MAX_BODY);
        }
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.state.pending_mgmt.insert(request_id.clone(), tx);

        let msg = OverlayMsg::MgmtReq {
            request_id: request_id.clone(),
            from_id: self.state.cfg.node_id.clone(),
            to_id: to_node_id.to_string(),
            method: method.to_string(),
            path: path.to_string(),
            headers,
            body,
        };
        if let Err(e) = send_to_node(&self.state, to_node_id, &msg).await {
            self.state.pending_mgmt.remove(&request_id);
            return Err(e);
        }

        match tokio::time::timeout(timeout.unwrap_or(MGMT_DEFAULT_TIMEOUT), rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_)) => Err(anyhow::anyhow!("mgmt channel closed")),
            Err(_) => {
                self.state.pending_mgmt.remove(&request_id);
                Err(anyhow::anyhow!("mgmt forward timeout"))
            }
        }
    }

    /// Edge publishes local services to all Overlay Servers on the mesh.
    pub async fn publish_register_service(
        &self,
        service_id: &str,
        name: &str,
        protocol: &str,
        local_addr: &str,
    ) -> anyhow::Result<()> {
        let msg = OverlayMsg::CtrlRegisterService {
            edge_id: self.state.cfg.node_id.clone(),
            service_id: service_id.to_string(),
            name: name.to_string(),
            protocol: protocol.to_string(),
            local_addr: local_addr.to_string(),
        };
        let bytes = encode_msg(&msg)?;
        let mut sent = false;
        for p in self.state.peers.list() {
            if p.role == OverlayRole::Server && p.node_id != self.state.cfg.node_id {
                for ep in p.all_targets() {
                    let _ = self.state.sock.send_to(&bytes, ep).await;
                    sent = true;
                }
            }
        }
        if !sent {
            for b in &self.state.cfg.bootstrap {
                if let Ok(addr) = resolve_addr(b).await {
                    let _ = self.state.sock.send_to(&bytes, addr).await;
                    sent = true;
                }
            }
        }
        if !sent {
            anyhow::bail!("no overlay server to register service with");
        }
        Ok(())
    }
}

/// Start mesh and wait until an internal address (VIP) is assigned.
pub async fn start_overlay(cfg: OverlayConfig) -> anyhow::Result<OverlayMesh> {
    if !cfg.enabled {
        anyhow::bail!("overlay disabled");
    }
    if cfg.node_id.trim().is_empty() {
        anyhow::bail!("overlay node_id empty");
    }

    let bind = format!("{}:{}", cfg.listen, cfg.port);
    let sock = UdpSocket::bind(&bind).await?;
    sock.set_broadcast(true)?;
    let sock = Arc::new(sock);
    info!(%bind, role = ?cfg.role, node_id = %cfg.node_id, "overlay UDP bound");

    let peers = PeerTable::default();
    let (_prefix_net, prefix_len) = cfg.subnet_prefix()?;

    let pool = if cfg.role == OverlayRole::Server {
        let (s, e) = cfg.pool_bounds()?;
        Some(Arc::new(DhcpPool::new(s, e)?))
    } else {
        None
    };

    let vip = match cfg.role {
        OverlayRole::Server => {
            if let Some(ref fixed) = cfg.fixed_vip {
                let ip = OverlayConfig::parse_vip(fixed)?;
                if let Some(ref p) = pool {
                    p.reserve(ip);
                }
                ip
            } else if let Some(ref p) = pool {
                p.allocate()
                    .ok_or_else(|| anyhow::anyhow!("dhcp pool exhausted for server VIP"))?
            } else {
                anyhow::bail!("server needs dhcp pool");
            }
        }
        OverlayRole::Node => Ipv4Addr::UNSPECIFIED,
    };

    if cfg.role == OverlayRole::Server && vip != Ipv4Addr::UNSPECIFIED {
        peers.upsert(PeerEntry {
            node_id: cfg.node_id.clone(),
            vip,
            role: OverlayRole::Server,
            endpoint: sock.local_addr().ok(),
            candidates: sock.local_addr().ok().into_iter().collect(),
            direct_ok: true,
            last_seen: Instant::now(),
            last_punch: Instant::now(),
        });
    }

    let tun_name = format!("ng-{}", &cfg.node_id[..cfg.node_id.len().min(8)]);
    let tun = if vip != Ipv4Addr::UNSPECIFIED {
        TunDevice::open(&tun_name, vip, prefix_len, cfg.create_tun).await?
    } else {
        TunDevice::open(&tun_name, Ipv4Addr::UNSPECIFIED, prefix_len, false).await?
    };
    let tun = Arc::new(Mutex::new(tun));

    let state = Arc::new(OverlayState {
        cfg: cfg.clone(),
        sock: sock.clone(),
        peers: peers.clone(),
        pool,
        tun: tun.clone(),
        local_vip: Arc::new(Mutex::new(vip)),
        prefix_len,
        reflexive: Arc::new(Mutex::new(None)),
        pending_mgmt: DashMap::new(),
        mgmt_handler: RwLock::new(None),
        register_handler: RwLock::new(None),
    });

    if !cfg.bootstrap.is_empty() || cfg.role == OverlayRole::Node {
        let st = state.clone();
        tokio::spawn(async move {
            if let Err(e) = bootstrap_join(st).await {
                warn!(error = %e, "overlay bootstrap join failed");
            }
        });
    }

    if cfg.role == OverlayRole::Server {
        let st = state.clone();
        tokio::spawn(async move {
            gossip_loop(st).await;
        });
    }

    {
        let st = state.clone();
        tokio::spawn(async move {
            heartbeat_loop(st).await;
        });
    }
    {
        let st = state.clone();
        tokio::spawn(async move {
            stun_loop(st).await;
        });
    }
    {
        let st = state.clone();
        tokio::spawn(async move {
            punch_loop(st).await;
        });
    }
    {
        let st = state.clone();
        tokio::spawn(async move {
            if let Err(e) = udp_loop(st).await {
                warn!(error = %e, "overlay udp loop exited");
            }
        });
    }
    {
        let st = state.clone();
        tokio::spawn(async move {
            if let Err(e) = tun_to_udp_loop(st).await {
                warn!(error = %e, "overlay tun loop exited");
            }
        });
    }

    let vip = wait_for_vip(&state, Duration::from_secs(30)).await?;
    info!(%vip, node_id = %cfg.node_id, "overlay ready (mgmt channel on mesh)");

    Ok(OverlayMesh { state })
}

/// Deprecated name — use [`start_overlay`].
pub async fn run_overlay(cfg: OverlayConfig) -> anyhow::Result<OverlayMesh> {
    start_overlay(cfg).await
}

struct OverlayState {
    cfg: OverlayConfig,
    sock: Arc<UdpSocket>,
    peers: PeerTable,
    pool: Option<Arc<DhcpPool>>,
    tun: Arc<Mutex<TunDevice>>,
    local_vip: Arc<Mutex<Ipv4Addr>>,
    prefix_len: u8,
    /// Our UDP address as seen by an Overlay Server (NAT reflexive).
    reflexive: Arc<Mutex<Option<SocketAddr>>>,
    pending_mgmt: DashMap<String, oneshot::Sender<MgmtResponse>>,
    mgmt_handler: RwLock<Option<MgmtHandler>>,
    register_handler: RwLock<Option<RegisterHandler>>,
}

async fn send_to_node(
    state: &Arc<OverlayState>,
    to_id: &str,
    msg: &OverlayMsg,
) -> anyhow::Result<()> {
    let bytes = encode_msg(msg)?;
    if let Some(peer) = state.peers.get_by_id(to_id) {
        let targets = peer.all_targets();
        if let Some(ep) = peer.endpoint.or_else(|| targets.first().copied()) {
            state.sock.send_to(&bytes, ep).await?;
            return Ok(());
        }
    }
    // Relay via any other overlay server (or bootstrap).
    let mut sent = false;
    for p in state.peers.list() {
        if p.role == OverlayRole::Server && p.node_id != state.cfg.node_id && p.node_id != to_id
        {
            for ep in p.all_targets() {
                let _ = state.sock.send_to(&bytes, ep).await;
                sent = true;
            }
        }
    }
    if !sent {
        for b in &state.cfg.bootstrap {
            if let Ok(addr) = resolve_addr(b).await {
                let _ = state.sock.send_to(&bytes, addr).await;
                sent = true;
            }
        }
    }
    if sent {
        Ok(())
    } else {
        anyhow::bail!("no route to overlay peer {to_id}")
    }
}

async fn wait_for_vip(state: &Arc<OverlayState>, timeout: Duration) -> anyhow::Result<Ipv4Addr> {
    let start = Instant::now();
    loop {
        let vip = *state.local_vip.lock().await;
        if vip != Ipv4Addr::UNSPECIFIED {
            return Ok(vip);
        }
        if start.elapsed() > timeout {
            anyhow::bail!("timeout waiting for overlay VIP (check bootstrap / token)");
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn advertise_endpoint(state: &Arc<OverlayState>) -> Option<String> {
    let local = state.sock.local_addr().ok();
    let reflexive = *state.reflexive.lock().await;
    prefer_advertise(local, reflexive).map(|a| a.to_string())
}

fn peer_info_msg(p: &PeerEntry) -> PeerInfoMsg {
    PeerInfoMsg {
        node_id: p.node_id.clone(),
        vip: p.vip.to_string(),
        role: p.role,
        endpoint: p.endpoint.map(|e| e.to_string()),
        endpoints: p.candidates.iter().map(|c| c.to_string()).collect(),
    }
}

fn parse_endpoints(primary: Option<&String>, extras: &[String]) -> Vec<SocketAddr> {
    let mut out = Vec::new();
    if let Some(s) = primary {
        if let Ok(a) = s.parse() {
            out.push(a);
        }
    }
    for s in extras {
        if let Ok(a) = s.parse::<SocketAddr>() {
            if !out.contains(&a) {
                out.push(a);
            }
        }
    }
    out
}

async fn bootstrap_join(state: Arc<OverlayState>) -> anyhow::Result<()> {
    let vip_hint = {
        let v = *state.local_vip.lock().await;
        if v == Ipv4Addr::UNSPECIFIED {
            None
        } else {
            Some(v.to_string())
        }
    };

    for _ in 0..10 {
        let endpoint = advertise_endpoint(&state).await;
        let msg = OverlayMsg::Join {
            node_id: state.cfg.node_id.clone(),
            token: state.cfg.token.clone(),
            role: state.cfg.role,
            endpoint,
            vip: vip_hint.clone(),
        };
        let bytes = encode_msg(&msg)?;
        for b in &state.cfg.bootstrap {
            if let Ok(addr) = resolve_addr(b).await {
                let _ = state.sock.send_to(&bytes, addr).await;
                let _ = send_stun_query(&state.sock, addr, &state.cfg.node_id).await;
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        if *state.local_vip.lock().await != Ipv4Addr::UNSPECIFIED
            && state.cfg.role == OverlayRole::Node
        {
            return Ok(());
        }
        if state.cfg.role == OverlayRole::Server {
            return Ok(());
        }
    }
    Ok(())
}

async fn resolve_addr(s: &str) -> anyhow::Result<SocketAddr> {
    if let Ok(a) = s.parse::<SocketAddr>() {
        return Ok(a);
    }
    let mut it = tokio::net::lookup_host(s).await?;
    it.next()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve {s}"))
}

async fn stun_loop(state: Arc<OverlayState>) {
    let mut tick = tokio::time::interval(Duration::from_secs(20));
    loop {
        tick.tick().await;
        for b in &state.cfg.bootstrap {
            if let Ok(addr) = resolve_addr(b).await {
                let _ = send_stun_query(&state.sock, addr, &state.cfg.node_id).await;
            }
        }
        for p in state.peers.list() {
            if p.role == OverlayRole::Server && p.node_id != state.cfg.node_id {
                for t in p.all_targets() {
                    let _ = send_stun_query(&state.sock, t, &state.cfg.node_id).await;
                }
            }
        }
    }
}

async fn punch_loop(state: Arc<OverlayState>) {
    let mut tick = tokio::time::interval(Duration::from_secs(3));
    loop {
        tick.tick().await;
        if *state.local_vip.lock().await == Ipv4Addr::UNSPECIFIED {
            continue;
        }
        for peer in state.peers.needs_punch(&state.cfg.node_id, Duration::from_secs(3)) {
            for target in peer.all_targets() {
                if let Err(e) =
                    send_punch(&state.sock, target, &state.cfg.node_id, &peer.node_id).await
                {
                    debug!(error = %e, %target, "punch send failed");
                }
            }
            if let Some(mut e) = state.peers.get_by_id(&peer.node_id) {
                e.last_punch = Instant::now();
                // refresh punch timer without clearing candidates
                state.peers.upsert(e);
            }
        }
    }
}

async fn heartbeat_loop(state: Arc<OverlayState>) {
    let mut tick = tokio::time::interval(Duration::from_secs(5));
    loop {
        tick.tick().await;
        let vip = *state.local_vip.lock().await;
        if vip == Ipv4Addr::UNSPECIFIED {
            continue;
        }
        let endpoint = advertise_endpoint(&state).await;
        let msg = OverlayMsg::Heartbeat {
            node_id: state.cfg.node_id.clone(),
            vip: vip.to_string(),
            endpoint,
        };
        if let Ok(bytes) = encode_msg(&msg) {
            broadcast_to_peers(&state, &bytes).await;
            for b in &state.cfg.bootstrap {
                if let Ok(addr) = resolve_addr(b).await {
                    let _ = state.sock.send_to(&bytes, addr).await;
                }
            }
        }
        for id in state.peers.stale_ids(Duration::from_secs(45)) {
            if id != state.cfg.node_id {
                state.peers.remove(&id);
            }
        }
    }
}

async fn gossip_loop(state: Arc<OverlayState>) {
    let mut tick = tokio::time::interval(Duration::from_secs(8));
    loop {
        tick.tick().await;
        let peers: Vec<PeerInfoMsg> = state.peers.list().iter().map(peer_info_msg).collect();
        let msg = OverlayMsg::PeerList { peers };
        if let Ok(bytes) = encode_msg(&msg) {
            for b in &state.cfg.bootstrap {
                if let Ok(addr) = resolve_addr(b).await {
                    let _ = state.sock.send_to(&bytes, addr).await;
                }
            }
            broadcast_to_peers(&state, &bytes).await;
        }
    }
}

async fn broadcast_to_peers(state: &Arc<OverlayState>, bytes: &[u8]) {
    for p in state.peers.list() {
        if p.node_id == state.cfg.node_id {
            continue;
        }
        for ep in p.all_targets() {
            let _ = state.sock.send_to(bytes, ep).await;
        }
    }
}

/// Introduce `a` and `b` so they punch each other (server-assisted).
async fn introduce_pair(state: &Arc<OverlayState>, a: &PeerEntry, b: &PeerEntry) {
    if a.node_id == b.node_id {
        return;
    }
    let to_a = OverlayMsg::PunchIntro {
        peer_id: b.node_id.clone(),
        peer_vip: b.vip.to_string(),
        peer_role: b.role,
        endpoints: b.candidates.iter().map(|c| c.to_string()).collect(),
    };
    let to_b = OverlayMsg::PunchIntro {
        peer_id: a.node_id.clone(),
        peer_vip: a.vip.to_string(),
        peer_role: a.role,
        endpoints: a.candidates.iter().map(|c| c.to_string()).collect(),
    };
    if let (Ok(ba), Ok(bb)) = (encode_msg(&to_a), encode_msg(&to_b)) {
        for ep in a.all_targets() {
            let _ = state.sock.send_to(&ba, ep).await;
        }
        for ep in b.all_targets() {
            let _ = state.sock.send_to(&bb, ep).await;
        }
    }
}

async fn introduce_new_peer(state: &Arc<OverlayState>, newbie: &PeerEntry) {
    for other in state.peers.list() {
        if other.node_id == newbie.node_id || other.node_id == state.cfg.node_id {
            continue;
        }
        introduce_pair(state, newbie, &other).await;
    }
}

async fn udp_loop(state: Arc<OverlayState>) -> anyhow::Result<()> {
    let mut buf = vec![0u8; 65535];
    loop {
        let (n, peer_addr) = state.sock.recv_from(&mut buf).await?;
        let msg = match decode_msg(&buf[..n]) {
            Ok(m) => m,
            Err(e) => {
                warn!(%peer_addr, error = %e, "bad overlay packet");
                continue;
            }
        };
        if let Err(e) = handle_msg(&state, msg, peer_addr).await {
            warn!(%peer_addr, error = %e, "overlay handle failed");
        }
    }
}

async fn handle_msg(
    state: &Arc<OverlayState>,
    msg: OverlayMsg,
    from: SocketAddr,
) -> anyhow::Result<()> {
    match msg {
        OverlayMsg::Join {
            node_id,
            token,
            role,
            endpoint,
            vip: prefer_vip,
        } => {
            if state.cfg.role != OverlayRole::Server {
                return Ok(());
            }
            if token != state.cfg.token {
                let deny = OverlayMsg::JoinDeny {
                    reason: "bad token".into(),
                };
                let _ = state.sock.send_to(&encode_msg(&deny)?, from).await;
                return Ok(());
            }
            let pool = state
                .pool
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no dhcp pool"))?;

            let vip = if let Some(existing) = state.peers.get_by_id(&node_id) {
                existing.vip
            } else if let Some(ref s) = prefer_vip {
                let ip = OverlayConfig::parse_vip(s)?;
                pool.reserve(ip);
                ip
            } else {
                pool.allocate()
                    .ok_or_else(|| anyhow::anyhow!("dhcp exhausted"))?
            };

            // Prefer server-observed `from` over self-reported (often RFC1918).
            let reported = endpoint.as_ref().and_then(|s| s.parse().ok());
            let mut entry = PeerEntry {
                node_id: node_id.clone(),
                vip,
                role,
                endpoint: Some(from),
                candidates: vec![],
                direct_ok: true,
                last_seen: Instant::now(),
                last_punch: Instant::now(),
            };
            entry.add_candidate(from);
            if let Some(r) = reported {
                entry.add_candidate(r);
            }
            state.peers.upsert(entry.clone());

            let ann = OverlayMsg::AllocAnnounce {
                node_id: node_id.clone(),
                vip: vip.to_string(),
            };
            if let Ok(bytes) = encode_msg(&ann) {
                for b in &state.cfg.bootstrap {
                    if let Ok(addr) = resolve_addr(b).await {
                        let _ = state.sock.send_to(&bytes, addr).await;
                    }
                }
            }

            let peers: Vec<PeerInfoMsg> = state.peers.list().iter().map(peer_info_msg).collect();
            let ok = OverlayMsg::JoinOk {
                node_id: node_id.clone(),
                vip: vip.to_string(),
                subnet_cidr: state.cfg.subnet_cidr.clone(),
                peers,
                observed: Some(from.to_string()),
            };
            state.sock.send_to(&encode_msg(&ok)?, from).await?;
            introduce_new_peer(state, &entry).await;
            info!(%node_id, %vip, observed = %from, "overlay join accepted");
        }
        OverlayMsg::JoinOk {
            node_id,
            vip,
            subnet_cidr: _,
            peers,
            observed,
        } => {
            if node_id != state.cfg.node_id {
                return Ok(());
            }
            if let Some(obs) = observed.as_ref().and_then(|s| s.parse().ok()) {
                *state.reflexive.lock().await = Some(obs);
                info!(%obs, "overlay reflexive endpoint (from JoinOk)");
            }
            let ip: Ipv4Addr = vip.parse()?;
            {
                let mut g = state.local_vip.lock().await;
                if *g == Ipv4Addr::UNSPECIFIED || *g != ip {
                    *g = ip;
                    let mut tun = state.tun.lock().await;
                    *tun = TunDevice::open(
                        tun.name(),
                        ip,
                        state.prefix_len,
                        state.cfg.create_tun,
                    )
                    .await?;
                }
            }
            let adv = advertise_endpoint(state).await.and_then(|s| s.parse().ok());
            let mut self_e = PeerEntry {
                node_id: state.cfg.node_id.clone(),
                vip: ip,
                role: state.cfg.role,
                endpoint: adv.or(state.sock.local_addr().ok()),
                candidates: vec![],
                direct_ok: true,
                last_seen: Instant::now(),
                last_punch: Instant::now(),
            };
            if let Some(a) = self_e.endpoint {
                self_e.add_candidate(a);
            }
            state.peers.upsert(self_e);

            for p in peers {
                upsert_from_info(state, &p, None);
            }
            // Immediately punch toward roster.
            for peer in state.peers.list() {
                if peer.node_id == state.cfg.node_id {
                    continue;
                }
                for t in peer.all_targets() {
                    let _ = send_punch(&state.sock, t, &state.cfg.node_id, &peer.node_id).await;
                }
            }
            info!(%ip, "overlay VIP assigned");
        }
        OverlayMsg::JoinDeny { reason } => {
            warn!(%reason, "overlay join denied");
        }
        OverlayMsg::Heartbeat {
            node_id,
            vip,
            endpoint,
        } => {
            if let Ok(ip) = vip.parse::<Ipv4Addr>() {
                let role = state
                    .peers
                    .get_by_id(&node_id)
                    .map(|p| p.role)
                    .unwrap_or(OverlayRole::Node);
                let mut entry = state.peers.get_by_id(&node_id).unwrap_or(PeerEntry {
                    node_id: node_id.clone(),
                    vip: ip,
                    role,
                    endpoint: None,
                    candidates: vec![],
                    direct_ok: false,
                    last_seen: Instant::now(),
                    last_punch: Instant::now()
                        .checked_sub(Duration::from_secs(60))
                        .unwrap_or_else(Instant::now),
                });
                entry.vip = ip;
                entry.last_seen = Instant::now();
                entry.add_candidate(from);
                if let Some(ep) = endpoint.and_then(|s| s.parse().ok()) {
                    entry.add_candidate(ep);
                }
                state.peers.upsert(entry);
            }
        }
        OverlayMsg::PeerList { peers } => {
            for p in peers {
                upsert_from_info(state, &p, None);
                if let Ok(ip) = p.vip.parse::<Ipv4Addr>() {
                    if let Some(ref pool) = state.pool {
                        pool.mark_used(ip);
                    }
                }
            }
        }
        OverlayMsg::AllocAnnounce { node_id, vip } => {
            if let Ok(ip) = vip.parse::<Ipv4Addr>() {
                if let Some(ref pool) = state.pool {
                    pool.mark_used(ip);
                }
                if state.peers.get_by_id(&node_id).is_none() {
                    state.peers.upsert(PeerEntry {
                        node_id,
                        vip: ip,
                        role: OverlayRole::Node,
                        endpoint: None,
                        candidates: vec![],
                        direct_ok: false,
                        last_seen: Instant::now(),
                        last_punch: Instant::now()
                            .checked_sub(Duration::from_secs(60))
                            .unwrap_or_else(Instant::now),
                    });
                }
            }
        }
        OverlayMsg::StunQuery { node_id: _, tx_id } => {
            let reply = OverlayMsg::StunReply {
                node_id: state.cfg.node_id.clone(),
                tx_id,
                observed: from.to_string(),
            };
            state.sock.send_to(&encode_msg(&reply)?, from).await?;
        }
        OverlayMsg::StunReply {
            node_id: _,
            tx_id: _,
            observed,
        } => {
            if let Ok(obs) = observed.parse::<SocketAddr>() {
                let mut g = state.reflexive.lock().await;
                let changed = g.map(|old| old != obs).unwrap_or(true);
                *g = Some(obs);
                if changed {
                    info!(%obs, "overlay reflexive endpoint (STUN)");
                }
            }
        }
        OverlayMsg::Punch {
            from_id,
            to_id,
            tx_id,
        } => {
            if to_id != state.cfg.node_id {
                return Ok(());
            }
            state.peers.mark_direct(&from_id, from);
            send_punch_ack(&state.sock, from, &state.cfg.node_id, &from_id, tx_id).await?;
            debug!(peer = %from_id, %from, "punch received");
        }
        OverlayMsg::PunchAck {
            from_id,
            to_id,
            tx_id: _,
        } => {
            if to_id != state.cfg.node_id {
                return Ok(());
            }
            state.peers.mark_direct(&from_id, from);
            info!(peer = %from_id, %from, "overlay direct path ok");
        }
        OverlayMsg::PunchIntro {
            peer_id,
            peer_vip,
            peer_role,
            endpoints,
        } => {
            if peer_id == state.cfg.node_id {
                return Ok(());
            }
            let vip: Ipv4Addr = peer_vip.parse()?;
            let addrs = parse_endpoints(None, &endpoints);
            let mut entry = state.peers.get_by_id(&peer_id).unwrap_or(PeerEntry {
                node_id: peer_id.clone(),
                vip,
                role: peer_role,
                endpoint: addrs.first().copied(),
                candidates: vec![],
                direct_ok: false,
                last_seen: Instant::now(),
                last_punch: Instant::now()
                    .checked_sub(Duration::from_secs(60))
                    .unwrap_or_else(Instant::now),
            });
            entry.vip = vip;
            entry.role = peer_role;
            for a in &addrs {
                entry.add_candidate(*a);
            }
            state.peers.upsert(entry);
            for a in addrs {
                let _ = send_punch(&state.sock, a, &state.cfg.node_id, &peer_id).await;
            }
            debug!(%peer_id, n = endpoints.len(), "punch intro");
        }
        OverlayMsg::Packet {
            from_vip: _,
            to_vip,
            payload,
        } => {
            // Any live Packet is also a path confirmation.
            let local = *state.local_vip.lock().await;
            let dest: Ipv4Addr = to_vip.parse()?;
            if dest == local {
                state.tun.lock().await.write_packet(&payload).await?;
            } else if state.cfg.role == OverlayRole::Server {
                forward_payload(state, dest, &payload).await?;
            }
        }
        OverlayMsg::Relay {
            from_vip: _,
            to_vip,
            payload,
        } => {
            if state.cfg.role != OverlayRole::Server {
                return Ok(());
            }
            let dest: Ipv4Addr = to_vip.parse()?;
            forward_payload(state, dest, &payload).await?;
        }
        OverlayMsg::MgmtReq {
            request_id,
            from_id,
            to_id,
            method,
            path,
            headers,
            body,
        } => {
            if to_id != state.cfg.node_id {
                // Relay toward target if we know a direct path.
                if state.cfg.role == OverlayRole::Server
                    && state
                        .peers
                        .get_by_id(&to_id)
                        .map(|p| !p.all_targets().is_empty())
                        .unwrap_or(false)
                {
                    let msg = OverlayMsg::MgmtReq {
                        request_id,
                        from_id,
                        to_id: to_id.clone(),
                        method,
                        path,
                        headers,
                        body,
                    };
                    let _ = send_to_node(state, &to_id, &msg).await;
                }
                return Ok(());
            }
            let handler = state.mgmt_handler.read().await.clone();
            let st = state.clone();
            tokio::spawn(async move {
                let resp = if let Some(h) = handler {
                    h(MgmtRequest {
                        request_id: request_id.clone(),
                        method,
                        path,
                        headers,
                        body,
                    })
                    .await
                } else {
                    MgmtResponse::err("no mgmt handler on this node")
                };
                let msg = OverlayMsg::MgmtRes {
                    request_id,
                    from_id: st.cfg.node_id.clone(),
                    to_id: from_id.clone(),
                    status: resp.status,
                    headers: resp.headers,
                    body: resp.body,
                    error: resp.error,
                };
                if let Err(e) = send_to_node(&st, &from_id, &msg).await {
                    warn!(error = %e, "mgmt res send failed");
                }
            });
        }
        OverlayMsg::MgmtRes {
            request_id,
            from_id: _,
            to_id,
            status,
            headers,
            body,
            error,
        } => {
            if to_id != state.cfg.node_id {
                if state.cfg.role == OverlayRole::Server
                    && state
                        .peers
                        .get_by_id(&to_id)
                        .map(|p| !p.all_targets().is_empty())
                        .unwrap_or(false)
                {
                    let msg = OverlayMsg::MgmtRes {
                        request_id,
                        from_id: state.cfg.node_id.clone(),
                        to_id: to_id.clone(),
                        status,
                        headers,
                        body,
                        error,
                    };
                    let _ = send_to_node(state, &to_id, &msg).await;
                }
                return Ok(());
            }
            if let Some((_, tx)) = state.pending_mgmt.remove(&request_id) {
                let _ = tx.send(MgmtResponse {
                    status,
                    headers,
                    body,
                    error,
                });
            }
        }
        OverlayMsg::CtrlRegisterService {
            edge_id,
            service_id,
            name,
            protocol,
            local_addr,
        } => {
            let handler = state.register_handler.read().await.clone();
            if let Some(h) = handler {
                h(RegisterServiceEvent {
                    edge_id,
                    service_id,
                    name,
                    protocol,
                    local_addr,
                })
                .await;
            }
        }
    }
    Ok(())
}

fn upsert_from_info(state: &Arc<OverlayState>, p: &PeerInfoMsg, observed: Option<SocketAddr>) {
    let Ok(ip) = p.vip.parse::<Ipv4Addr>() else {
        return;
    };
    if p.node_id == state.cfg.node_id {
        return;
    }
    let addrs = parse_endpoints(p.endpoint.as_ref(), &p.endpoints);
    let mut entry = state.peers.get_by_id(&p.node_id).unwrap_or(PeerEntry {
        node_id: p.node_id.clone(),
        vip: ip,
        role: p.role,
        endpoint: None,
        candidates: vec![],
        direct_ok: false,
        last_seen: Instant::now(),
        last_punch: Instant::now()
            .checked_sub(Duration::from_secs(60))
            .unwrap_or_else(Instant::now),
    });
    entry.vip = ip;
    entry.role = p.role;
    entry.last_seen = Instant::now();
    for a in &addrs {
        entry.add_candidate(*a);
    }
    if let Some(o) = observed {
        entry.add_candidate(o);
    }
    state.peers.upsert(entry);
}

async fn forward_payload(
    state: &Arc<OverlayState>,
    dest: Ipv4Addr,
    payload: &[u8],
) -> anyhow::Result<()> {
    let local = *state.local_vip.lock().await;
    if dest == local {
        state.tun.lock().await.write_packet(payload).await?;
        return Ok(());
    }
    if let Some(peer) = state.peers.get_by_vip(dest) {
        let targets = peer.all_targets();
        if !targets.is_empty() {
            let msg = OverlayMsg::Packet {
                from_vip: local.to_string(),
                to_vip: dest.to_string(),
                payload: payload.to_vec(),
            };
            let bytes = encode_msg(&msg)?;
            // Prefer confirmed direct endpoint first.
            if let Some(ep) = peer.endpoint {
                state.sock.send_to(&bytes, ep).await?;
            } else {
                state.sock.send_to(&bytes, targets[0]).await?;
            }
        }
    }
    Ok(())
}

async fn tun_to_udp_loop(state: Arc<OverlayState>) -> anyhow::Result<()> {
    let mut buf = vec![0u8; 65535];
    loop {
        let n = {
            let mut tun = state.tun.lock().await;
            if !tun.is_real() && tun.vip() == Ipv4Addr::UNSPECIFIED {
                drop(tun);
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            match tun.read_packet(&mut buf).await {
                Ok(0) => continue,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e.into()),
            }
        };
        if n < 20 {
            continue;
        }
        let dest = Ipv4Addr::new(buf[16], buf[17], buf[18], buf[19]);
        let local = *state.local_vip.lock().await;
        let payload = buf[..n].to_vec();

        if let Some(peer) = state.peers.get_by_vip(dest) {
            let targets = peer.all_targets();
            if !targets.is_empty() {
                let msg = OverlayMsg::Packet {
                    from_vip: local.to_string(),
                    to_vip: dest.to_string(),
                    payload: payload.clone(),
                };
                if let Ok(bytes) = encode_msg(&msg) {
                    let prefer = peer.endpoint.unwrap_or(targets[0]);
                    let _ = state.sock.send_to(&bytes, prefer).await;
                    // Opportunistic punch on other candidates if not yet direct.
                    if !peer.direct_ok {
                        for t in targets.into_iter().filter(|t| *t != prefer).take(3) {
                            let _ =
                                send_punch(&state.sock, t, &state.cfg.node_id, &peer.node_id).await;
                        }
                    }
                    continue;
                }
            }
        }

        // Fallback: relay via any overlay server peer
        let msg = OverlayMsg::Relay {
            from_vip: local.to_string(),
            to_vip: dest.to_string(),
            payload,
        };
        let bytes = encode_msg(&msg)?;
        let mut sent = false;
        for p in state.peers.list() {
            if p.role == OverlayRole::Server && p.node_id != state.cfg.node_id {
                for ep in p.all_targets() {
                    let _ = state.sock.send_to(&bytes, ep).await;
                    sent = true;
                }
            }
        }
        if !sent {
            for b in &state.cfg.bootstrap {
                if let Ok(addr) = resolve_addr(b).await {
                    let _ = state.sock.send_to(&bytes, addr).await;
                }
            }
        }
    }
}
