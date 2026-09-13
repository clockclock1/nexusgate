use parking_lot::Mutex;
use std::collections::BTreeSet;
use std::net::Ipv4Addr;

/// Simple contiguous IPv4 DHCP allocator for the overlay subnet.
pub struct DhcpPool {
    inner: Mutex<PoolInner>,
}

struct PoolInner {
    start: u32,
    end: u32,
    used: BTreeSet<u32>,
}

impl DhcpPool {
    pub fn new(start: Ipv4Addr, end: Ipv4Addr) -> anyhow::Result<Self> {
        let s = u32::from(start);
        let e = u32::from(end);
        if s > e {
            anyhow::bail!("dhcp_start > dhcp_end");
        }
        Ok(Self {
            inner: Mutex::new(PoolInner {
                start: s,
                end: e,
                used: BTreeSet::new(),
            }),
        })
    }

    pub fn allocate(&self) -> Option<Ipv4Addr> {
        let mut g = self.inner.lock();
        for ip in g.start..=g.end {
            if !g.used.contains(&ip) {
                g.used.insert(ip);
                return Some(Ipv4Addr::from(ip));
            }
        }
        None
    }

    pub fn reserve(&self, ip: Ipv4Addr) -> bool {
        let v = u32::from(ip);
        let mut g = self.inner.lock();
        if v < g.start || v > g.end {
            // allow fixed VIPs outside pool
            g.used.insert(v);
            return true;
        }
        if g.used.contains(&v) {
            return false;
        }
        g.used.insert(v);
        true
    }

    pub fn mark_used(&self, ip: Ipv4Addr) {
        self.inner.lock().used.insert(u32::from(ip));
    }

    pub fn release(&self, ip: Ipv4Addr) {
        self.inner.lock().used.remove(&u32::from(ip));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_sequential() {
        let p = DhcpPool::new("10.88.0.10".parse().unwrap(), "10.88.0.12".parse().unwrap())
            .unwrap();
        assert_eq!(p.allocate().unwrap().to_string(), "10.88.0.10");
        assert_eq!(p.allocate().unwrap().to_string(), "10.88.0.11");
        assert_eq!(p.allocate().unwrap().to_string(), "10.88.0.12");
        assert!(p.allocate().is_none());
    }
}
