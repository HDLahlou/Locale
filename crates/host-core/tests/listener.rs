//! Single-port listener: TLS and plain HTTP on one port, chosen by the first
//! byte (spec §5.1), with a pinned TLS client (spec §8.1). All on loopback.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ConnectInfo;
use axum::routing::get;
use axum::{Extension, Router};
use client_core::tls_pin::{self, PinnedTlsError};
use host_core::listener::{
    bind_with_fallback, start, ListenerConfig, ListenerHandle, TlsIdentity, Transport,
};
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::{Request, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);
const FIRST_BYTE_TIMEOUT: Duration = Duration::from_millis(800);

#[derive(Default)]
struct Hits {
    tls: AtomicUsize,
    plain: AtomicUsize,
}

impl Hits {
    fn tls(&self) -> usize {
        self.tls.load(Ordering::SeqCst)
    }
    fn plain(&self) -> usize {
        self.plain.load(Ordering::SeqCst)
    }
}

/// One router per transport; each counts its calls and echoes the transport
/// and peer address it was handed.
fn routers(hits: Arc<Hits>) -> (Router, Router) {
    let h = hits.clone();
    let tls = Router::new().route(
        "/",
        get(
            move |Extension(t): Extension<Transport>,
                  ConnectInfo(peer): ConnectInfo<SocketAddr>| {
                h.tls.fetch_add(1, Ordering::SeqCst);
                async move { format!("tls-service {t:?} {}", peer.ip()) }
            },
        ),
    );
    let h = hits;
    let plain = Router::new().route(
        "/",
        get(
            move |Extension(t): Extension<Transport>,
                  ConnectInfo(peer): ConnectInfo<SocketAddr>| {
                h.plain.fetch_add(1, Ordering::SeqCst);
                async move { format!("plain-service {t:?} {}", peer.ip()) }
            },
        ),
    );
    (tls, plain)
}

fn config(port: u16) -> ListenerConfig {
    ListenerConfig {
        ip: LOCALHOST,
        port,
        first_byte_timeout: FIRST_BYTE_TIMEOUT,
        handshake_timeout: FIRST_BYTE_TIMEOUT,
        shutdown_grace: Duration::from_millis(500),
        ..ListenerConfig::default()
    }
}

async fn start_with(identity: &TlsIdentity, port: u16) -> (ListenerHandle, Arc<Hits>) {
    let hits = Arc::new(Hits::default());
    let (tls, plain) = routers(hits.clone());
    let handle = start(config(port), identity, tls, plain).await.unwrap();
    (handle, hits)
}

async fn http_get<S>(io: S) -> (StatusCode, String)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(io))
        .await
        .unwrap();
    tokio::spawn(conn);
    let req = Request::get("/")
        .header("host", "locale.test")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let resp = sender.send_request(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

async fn https_get(
    addr: SocketAddr,
    pin: [u8; 32],
) -> Result<(StatusCode, String), PinnedTlsError> {
    let tls = tls_pin::connect(addr, pin).await?;
    Ok(http_get(tls).await)
}

async fn plain_get(addr: SocketAddr) -> (StatusCode, String) {
    http_get(TcpStream::connect(addr).await.unwrap()).await
}

/// Waits for the server to close `stream`; returns how long that took.
async fn wait_closed(stream: &mut TcpStream, limit: Duration) -> Duration {
    let started = Instant::now();
    let mut buf = [0u8; 64];
    loop {
        match timeout(limit, stream.read(&mut buf)).await {
            Ok(Ok(0)) | Ok(Err(_)) => return started.elapsed(),
            Ok(Ok(_)) => continue,
            Err(_) => panic!("connection still open after {limit:?}"),
        }
    }
}

#[tokio::test]
async fn listener_pinned_client_completes_https_request() {
    let id = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle, hits) = start_with(&id, 0).await;

    let (status, body) = https_get(handle.local_addr(), *id.fingerprint())
        .await
        .unwrap();
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "tls-service Tls 127.0.0.1");
    assert_eq!((hits.tls(), hits.plain()), (1, 0));

    handle.shutdown().await;
}

#[tokio::test]
async fn listener_plain_get_reaches_plain_service() {
    let id = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle, hits) = start_with(&id, 0).await;

    let (status, body) = plain_get(handle.local_addr()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "plain-service Plain 127.0.0.1");
    assert_eq!((hits.tls(), hits.plain()), (0, 1));

    handle.shutdown().await;
}

#[tokio::test]
async fn listener_transports_never_cross() {
    let id = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle, hits) = start_with(&id, 0).await;
    let addr = handle.local_addr();

    for round in 1..=3 {
        let (_, tls_body) = https_get(addr, *id.fingerprint()).await.unwrap();
        assert!(tls_body.starts_with("tls-service Tls"), "{tls_body}");
        assert_eq!((hits.tls(), hits.plain()), (round, round - 1));

        let (_, plain_body) = plain_get(addr).await;
        assert!(
            plain_body.starts_with("plain-service Plain"),
            "{plain_body}"
        );
        assert_eq!((hits.tls(), hits.plain()), (round, round));
    }

    handle.shutdown().await;
}

#[tokio::test]
async fn listener_wrong_pin_fails_with_mismatch_and_handler_never_runs() {
    let id = TlsIdentity::generate(LIFETIME).unwrap();
    let other = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle, hits) = start_with(&id, 0).await;

    let err = https_get(handle.local_addr(), *other.fingerprint())
        .await
        .unwrap_err();
    assert!(err.is_fingerprint_mismatch(), "{err:?}");

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!((hits.tls(), hits.plain()), (0, 0));

    handle.shutdown().await;
}

#[tokio::test]
async fn listener_right_pin_but_different_certificate_fails_with_mismatch() {
    // The client pins host A's fingerprint but reaches a listener that
    // presents host B's certificate (for example after an IP change).
    let a = TlsIdentity::generate(LIFETIME).unwrap();
    let b = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle_a, hits_a) = start_with(&a, 0).await;
    let (handle_b, hits_b) = start_with(&b, 0).await;

    assert!(https_get(handle_a.local_addr(), *a.fingerprint())
        .await
        .is_ok());

    let err = https_get(handle_b.local_addr(), *a.fingerprint())
        .await
        .unwrap_err();
    assert!(err.is_fingerprint_mismatch(), "{err:?}");

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!((hits_b.tls(), hits_b.plain()), (0, 0));
    assert_eq!((hits_a.tls(), hits_a.plain()), (1, 0));

    handle_a.shutdown().await;
    handle_b.shutdown().await;
}

#[tokio::test]
async fn listener_silent_connection_is_closed_without_blocking_others() {
    let id = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle, hits) = start_with(&id, 0).await;
    let addr = handle.local_addr();

    let mut silent = TcpStream::connect(addr).await.unwrap();
    let opened = Instant::now();

    // While the silent connection is still waiting for its first byte, other
    // connections are accepted and served.
    let quick = Duration::from_millis(500);
    timeout(quick, plain_get(addr))
        .await
        .expect("plain blocked");
    timeout(quick, https_get(addr, *id.fingerprint()))
        .await
        .expect("tls blocked")
        .unwrap();
    assert!(opened.elapsed() < FIRST_BYTE_TIMEOUT);
    assert_eq!((hits.tls(), hits.plain()), (1, 1));

    wait_closed(&mut silent, Duration::from_secs(5)).await;
    let lived = opened.elapsed();
    assert!(
        lived >= FIRST_BYTE_TIMEOUT - Duration::from_millis(100),
        "{lived:?}"
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn listener_stalled_tls_handshake_does_not_block_others() {
    let id = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle, hits) = start_with(&id, 0).await;
    let addr = handle.local_addr();

    // Only the TLS record type byte, then nothing.
    let mut stalled = TcpStream::connect(addr).await.unwrap();
    stalled.write_all(&[0x16]).await.unwrap();
    let opened = Instant::now();

    let quick = Duration::from_millis(500);
    timeout(quick, https_get(addr, *id.fingerprint()))
        .await
        .expect("tls blocked")
        .unwrap();
    timeout(quick, plain_get(addr))
        .await
        .expect("plain blocked");
    assert!(opened.elapsed() < FIRST_BYTE_TIMEOUT);

    wait_closed(&mut stalled, Duration::from_secs(5)).await;
    assert_eq!((hits.tls(), hits.plain()), (1, 1));

    handle.shutdown().await;
}

#[tokio::test]
async fn listener_shuts_down_cleanly_and_frees_the_port() {
    let id = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle, _hits) = start_with(&id, 0).await;
    let addr = handle.local_addr();

    // An idle keep-alive connection and a silent one are open at shutdown.
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(
        TcpStream::connect(addr).await.unwrap(),
    ))
    .await
    .unwrap();
    let keep_alive = tokio::spawn(conn);
    let req = Request::get("/")
        .header("host", "locale.test")
        .body(Empty::<Bytes>::new())
        .unwrap();
    let resp = sender.send_request(req).await.unwrap();
    resp.into_body().collect().await.unwrap();
    let mut silent = TcpStream::connect(addr).await.unwrap();

    timeout(Duration::from_secs(3), handle.shutdown())
        .await
        .expect("shutdown hung");

    // The open connections are closed.
    timeout(Duration::from_secs(1), keep_alive)
        .await
        .expect("keep-alive connection still open")
        .unwrap()
        .ok();
    wait_closed(&mut silent, Duration::from_secs(1)).await;

    // Nothing accepts on the port, and it can be bound again.
    assert!(TcpStream::connect(addr).await.is_err());
    let rebound = TcpListener::bind(addr).await.expect("port still held");
    drop(rebound);
}

#[tokio::test]
async fn listener_dropping_the_handle_stops_it() {
    let id = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle, _hits) = start_with(&id, 0).await;
    let addr = handle.local_addr();
    drop(handle);

    let deadline = Instant::now() + Duration::from_secs(3);
    while TcpStream::connect(addr).await.is_ok() {
        assert!(Instant::now() < deadline, "listener still accepting");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// A port that some other socket holds, low enough to leave room above it.
fn hold_a_port() -> std::net::TcpListener {
    loop {
        let held = std::net::TcpListener::bind((LOCALHOST, 0)).unwrap();
        if held.local_addr().unwrap().port() < 65000 {
            return held;
        }
    }
}

#[tokio::test]
async fn listener_taken_port_falls_forward_to_next_free_one() {
    let held = hold_a_port();
    let taken = held.local_addr().unwrap().port();

    let id = TlsIdentity::generate(LIFETIME).unwrap();
    let (handle, hits) = start_with(&id, taken).await;
    let got = handle.local_addr().port();
    let attempts = ListenerConfig::default().port_attempts;
    assert!(
        got > taken && got < taken + attempts,
        "taken {taken}, got {got}"
    );

    let (_, body) = plain_get(handle.local_addr()).await;
    assert!(body.starts_with("plain-service"));
    https_get(handle.local_addr(), *id.fingerprint())
        .await
        .unwrap();
    assert_eq!((hits.tls(), hits.plain()), (1, 1));

    handle.shutdown().await;
    drop(held);
}

#[tokio::test]
async fn listener_requested_free_port_is_used_as_is() {
    let free = {
        let probe = hold_a_port();
        probe.local_addr().unwrap().port()
    };
    let listener = bind_with_fallback(LOCALHOST, free, 20).await.unwrap();
    assert_eq!(listener.local_addr().unwrap().port(), free);
}

#[tokio::test]
async fn listener_bind_fails_when_every_attempt_is_taken() {
    let held = hold_a_port();
    let taken = held.local_addr().unwrap().port();
    let err = bind_with_fallback(LOCALHOST, taken, 1).await.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
}
