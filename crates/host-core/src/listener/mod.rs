//! The host's single-port listener (spec §5.1).
//!
//! One TCP port carries both TLS and plain HTTP. Each accepted connection is
//! handled on its own task: the listener peeks at the first byte without
//! consuming it and serves TLS when it is `0x16` (a TLS handshake record),
//! plain HTTP otherwise. Native apps use TLS; plain HTTP serves the guest page
//! and guest API only, so the caller passes one router per transport.
//!
//! Every request carries its [`Transport`] and the peer address
//! (`axum::extract::ConnectInfo<SocketAddr>`) as request extensions.

mod identity;
mod server;

pub use identity::{
    fingerprint_base64url, fingerprint_of, Fingerprint, IdentityError, TlsIdentity, CERT_FILE_NAME,
    KEY_FILE_NAME, TLS_DIR_NAME,
};
pub use server::{bind_with_fallback, start, ListenerConfig, ListenerHandle, Transport};
