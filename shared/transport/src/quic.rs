//! QUIC data-plane transport (quinn).

use crate::traits::map_io;
use pin_project_lite::pin_project;
use p2p_common::{Error, Result};
use quinn::{ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tracing::debug;

pub const QUIC_ALPN: &[u8] = b"nexusgate-data/1";

pin_project! {
    /// Bidirectional QUIC stream usable with `copy_bidirectional`.
    /// Holds `Connection` (and optional client `Endpoint`) so the session stays alive.
    pub struct QuicBidiStream {
        #[pin]
        pub send: SendStream,
        #[pin]
        pub recv: RecvStream,
        // Keep alive for the stream lifetime.
        _conn: Connection,
        _endpoint: Option<Endpoint>,
    }
}

impl AsyncRead for QuicBidiStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.project().recv.poll_read(cx, buf)
    }
}

impl AsyncWrite for QuicBidiStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.project().send.poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => Poll::Ready(Ok(n)),
            Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::other(e))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project().send.poll_flush(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::other(e))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project().send.poll_shutdown(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::other(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn make_server_config() -> Result<ServerConfig> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into(), "nexusgate".into()])
        .map_err(|e| Error::internal(format!("rcgen: {e}")))?;
    let key = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    let cert_der = CertificateDer::from(cert.cert);

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key.into())
        .map_err(|e| Error::internal(format!("rustls server: {e}")))?;
    server_crypto.alpn_protocols = vec![QUIC_ALPN.to_vec()];

    let mut server = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
            .map_err(|e| Error::internal(format!("quic server crypto: {e}")))?,
    ));
    server.transport = Arc::new({
        let mut t = quinn::TransportConfig::default();
        t.max_concurrent_bidi_streams(4096u32.into());
        t.keep_alive_interval(Some(Duration::from_secs(10)));
        t
    });
    Ok(server)
}

/// Skip server cert verification (self-signed data-plane).
#[derive(Debug)]
struct SkipServerVerification(Arc<rustls::crypto::CryptoProvider>);

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(rustls::crypto::ring::default_provider())))
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn make_client_config() -> Result<ClientConfig> {
    let mut crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();
    crypto.alpn_protocols = vec![QUIC_ALPN.to_vec()];
    let mut cfg = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
            .map_err(|e| Error::internal(format!("quic client crypto: {e}")))?,
    ));
    cfg.transport_config(Arc::new({
        let mut t = quinn::TransportConfig::default();
        t.keep_alive_interval(Some(Duration::from_secs(10)));
        t
    }));
    Ok(cfg)
}

fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Bind a QUIC server endpoint (ephemeral self-signed cert).
pub async fn bind_quic_server(addr: SocketAddr) -> Result<Endpoint> {
    ensure_crypto_provider();
    let server = make_server_config()?;
    let endpoint = Endpoint::server(server, addr).map_err(map_io)?;
    debug!(%addr, "quic data endpoint bound");
    Ok(endpoint)
}

/// Create a client endpoint bound to an ephemeral UDP port.
pub fn bind_quic_client() -> Result<Endpoint> {
    ensure_crypto_provider();
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).map_err(map_io)?;
    endpoint.set_default_client_config(make_client_config()?);
    Ok(endpoint)
}

/// Accept one QUIC connection and open its first bidirectional stream.
pub async fn accept_quic_bidi(endpoint: &Endpoint) -> Result<(QuicBidiStream, SocketAddr)> {
    let incoming = endpoint
        .accept()
        .await
        .ok_or_else(|| Error::internal("quic endpoint closed"))?;
    let conn = incoming
        .await
        .map_err(|e| Error::internal(format!("quic accept: {e}")))?;
    let remote = conn.remote_address();
    let (send, recv) = conn
        .accept_bi()
        .await
        .map_err(|e| Error::internal(format!("quic accept_bi: {e}")))?;
    Ok((
        QuicBidiStream {
            send,
            recv,
            _conn: conn,
            _endpoint: None,
        },
        remote,
    ))
}

/// Dial server and open one bidirectional stream.
pub async fn connect_quic_bidi(addr: SocketAddr) -> Result<QuicBidiStream> {
    let endpoint = bind_quic_client()?;
    let conn: Connection = endpoint
        .connect(addr, "localhost")
        .map_err(|e| Error::internal(format!("quic connect setup: {e}")))?
        .await
        .map_err(|e| Error::internal(format!("quic connect: {e}")))?;
    let (send, recv) = conn
        .open_bi()
        .await
        .map_err(|e| Error::internal(format!("quic open_bi: {e}")))?;
    Ok(QuicBidiStream {
        send,
        recv,
        _conn: conn,
        _endpoint: Some(endpoint),
    })
}
