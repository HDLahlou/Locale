//! Accept loop, first-byte sniffing and per-connection HTTP serving.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::Router;
use hyper::body::Incoming;
use hyper::Request;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;
use tower::ServiceExt;

use super::identity::{IdentityError, TlsIdentity};

/// First byte of a TLS record carrying a handshake message.
const TLS_HANDSHAKE_BYTE: u8 = 0x16;

/// The transport a request arrived on. Inserted into every request's
/// extensions; read it with `axum::Extension<Transport>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transport {
    /// Pinned TLS (native apps).
    Tls,
    /// Plain HTTP (guest page and guest API).
    Plain,
}

/// Listener settings.
#[derive(Debug, Clone)]
pub struct ListenerConfig {
    /// Address to bind (all interfaces by default).
    pub ip: IpAddr,
    /// Requested port; 0 lets the OS choose.
    pub port: u16,
    /// How many consecutive ports to try, starting at `port`, when it is taken.
    pub port_attempts: u16,
    /// A connection that sends no byte within this time is closed.
    pub first_byte_timeout: Duration,
    /// A TLS handshake that does not finish within this time is dropped.
    pub handshake_timeout: Duration,
    /// On shutdown, open connections get this long to finish before they are
    /// aborted.
    pub shutdown_grace: Duration,
}

impl Default for ListenerConfig {
    fn default() -> Self {
        ListenerConfig {
            ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: 7420,
            port_attempts: 20,
            first_byte_timeout: Duration::from_secs(10),
            handshake_timeout: Duration::from_secs(10),
            shutdown_grace: Duration::from_secs(5),
        }
    }
}

/// Binds `ip:port`, or the next free port if it is taken, trying at most
/// `attempts` consecutive ports. Port 0 asks the OS for any free port.
pub async fn bind_with_fallback(ip: IpAddr, port: u16, attempts: u16) -> io::Result<TcpListener> {
    if port == 0 {
        return TcpListener::bind((ip, 0)).await;
    }
    let mut last_err = None;
    for candidate in (port..=u16::MAX).take(usize::from(attempts.max(1))) {
        match TcpListener::bind((ip, candidate)).await {
            Ok(listener) => return Ok(listener),
            // Windows reports some reserved or taken ports as access denied.
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::AddrInUse | io::ErrorKind::PermissionDenied
                ) =>
            {
                last_err = Some(e)
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| io::Error::from(io::ErrorKind::AddrInUse)))
}

/// A running listener. Dropping the handle also stops it, but only
/// [`ListenerHandle::shutdown`] waits until the port is free.
#[derive(Debug)]
pub struct ListenerHandle {
    local_addr: SocketAddr,
    shutdown_tx: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl ListenerHandle {
    /// The bound address, including the port actually used.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Stops accepting, closes the listening socket, lets open connections
    /// finish within the grace period, then aborts the rest.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(true);
        let _ = self.task.await;
    }
}

struct Shared {
    acceptor: TlsAcceptor,
    tls_router: Router,
    plain_router: Router,
    first_byte_timeout: Duration,
    handshake_timeout: Duration,
}

/// Binds the port and starts serving. `tls_router` serves connections that
/// start with a TLS handshake; `plain_router` serves everything else.
pub async fn start(
    config: ListenerConfig,
    identity: &TlsIdentity,
    tls_router: Router,
    plain_router: Router,
) -> Result<ListenerHandle, IdentityError> {
    let server_config = identity.server_config().map_err(IdentityError::Tls)?;
    let listener = bind_with_fallback(config.ip, config.port, config.port_attempts).await?;
    let local_addr = listener.local_addr()?;
    let shared = Arc::new(Shared {
        acceptor: TlsAcceptor::from(Arc::new(server_config)),
        tls_router,
        plain_router,
        first_byte_timeout: config.first_byte_timeout,
        handshake_timeout: config.handshake_timeout,
    });
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let task = tokio::spawn(accept_loop(
        listener,
        shared,
        shutdown_rx,
        config.shutdown_grace,
    ));
    Ok(ListenerHandle {
        local_addr,
        shutdown_tx,
        task,
    })
}

async fn accept_loop(
    listener: TcpListener,
    shared: Arc<Shared>,
    mut shutdown: watch::Receiver<bool>,
    grace: Duration,
) {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            // A sent value or a dropped handle both mean stop.
            _ = shutdown.changed() => break,
            accepted = listener.accept() => match accepted {
                Ok((stream, peer)) => {
                    connections.spawn(handle_connection(
                        stream,
                        peer,
                        shared.clone(),
                        shutdown.clone(),
                    ));
                }
                // Per-connection accept errors are transient; back off briefly
                // so resource exhaustion does not spin the loop.
                Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
            },
            Some(_) = connections.join_next(), if !connections.is_empty() => {}
        }
    }
    drop(listener);
    let drain = async { while connections.join_next().await.is_some() {} };
    let _ = timeout(grace, drain).await;
    connections.shutdown().await;
}

async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    shared: Arc<Shared>,
    shutdown: watch::Receiver<bool>,
) {
    let _ = stream.set_nodelay(true);
    let mut first = [0u8; 1];
    let read = timeout(shared.first_byte_timeout, stream.peek(&mut first)).await;
    match read {
        Ok(Ok(n)) if n > 0 => {}
        // Closed, failed, or silent past the timeout.
        _ => return,
    }

    if first[0] == TLS_HANDSHAKE_BYTE {
        let handshake = timeout(shared.handshake_timeout, shared.acceptor.accept(stream)).await;
        if let Ok(Ok(tls)) = handshake {
            serve(
                tls,
                Transport::Tls,
                peer,
                shared.tls_router.clone(),
                shutdown,
            )
            .await;
        }
    } else {
        serve(
            stream,
            Transport::Plain,
            peer,
            shared.plain_router.clone(),
            shutdown,
        )
        .await;
    }
}

async fn serve<S>(
    io: S,
    transport: Transport,
    peer: SocketAddr,
    router: Router,
    mut shutdown: watch::Receiver<bool>,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let service = hyper::service::service_fn(move |mut req: Request<Incoming>| {
        req.extensions_mut().insert(transport);
        req.extensions_mut().insert(ConnectInfo(peer));
        router.clone().oneshot(req)
    });
    let builder = auto::Builder::new(TokioExecutor::new());
    let conn = builder.serve_connection_with_upgrades(TokioIo::new(io), service);
    tokio::pin!(conn);
    tokio::select! {
        _ = conn.as_mut() => {}
        _ = shutdown.changed() => {
            conn.as_mut().graceful_shutdown();
            let _ = conn.await;
        }
    }
}
