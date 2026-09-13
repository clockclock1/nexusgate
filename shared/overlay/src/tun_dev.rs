//! TUN device helper. Unix: real TUN when enabled. Elsewhere: userspace stub.

use std::io;
use std::net::Ipv4Addr;
use tracing::warn;

pub struct TunDevice {
    #[cfg(unix)]
    inner: Option<tun::AsyncDevice>,
    name: String,
    vip: Ipv4Addr,
    prefix: u8,
    enabled: bool,
}

impl TunDevice {
    /// Create (or stub) a TUN with the given VIP.
    pub async fn open(name: &str, vip: Ipv4Addr, prefix: u8, create: bool) -> anyhow::Result<Self> {
        #[cfg(unix)]
        {
            if create {
                let mut config = tun::Configuration::default();
                config
                    .tun_name(name)
                    .address(vip)
                    .netmask(prefix_to_mask(prefix))
                    .up();
                let dev = tun::create_as_async(&config)
                    .map_err(|e| anyhow::anyhow!("tun create failed: {e}"))?;
                tracing::info!(%name, %vip, prefix, "overlay TUN up");
                return Ok(Self {
                    inner: Some(dev),
                    name: name.into(),
                    vip,
                    prefix,
                    enabled: true,
                });
            }
        }
        #[cfg(not(unix))]
        let _ = create;

        warn!(
            %name,
            %vip,
            "overlay TUN not created (userspace mode); VIP tracked in-process only"
        );
        Ok(Self {
            #[cfg(unix)]
            inner: None,
            name: name.into(),
            vip,
            prefix,
            enabled: false,
        })
    }

    pub fn vip(&self) -> Ipv4Addr {
        self.vip
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_real(&self) -> bool {
        self.enabled
    }

    /// Read one L3 packet from TUN (empty if userspace stub).
    pub async fn read_packet(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        #[cfg(unix)]
        {
            if let Some(dev) = self.inner.as_mut() {
                use tokio::io::AsyncReadExt;
                return dev.read(buf).await;
            }
        }
        let _ = buf;
        // Userspace: park until cancelled by select elsewhere.
        futures::future::pending::<()>().await;
        Ok(0)
    }

    /// Write one L3 packet into TUN.
    pub async fn write_packet(&mut self, packet: &[u8]) -> io::Result<()> {
        #[cfg(unix)]
        {
            if let Some(dev) = self.inner.as_mut() {
                use tokio::io::AsyncWriteExt;
                dev.write_all(packet).await?;
                return Ok(());
            }
        }
        let _ = (self.vip, self.prefix, packet);
        Ok(())
    }
}

#[cfg(unix)]
fn prefix_to_mask(prefix: u8) -> Ipv4Addr {
    if prefix == 0 {
        return Ipv4Addr::UNSPECIFIED;
    }
    let mask = u32::MAX << (32 - prefix as u32);
    Ipv4Addr::from(mask)
}
