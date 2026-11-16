//! Pinned TLS (spec §8.1, D6).
//!
//! The host presents a per-event self-signed certificate. A client accepts
//! exactly the certificate whose SHA-256 (over the DER) equals the pin taken
//! from the QR code or link. There is no CA validation and no hostname check,
//! but the handshake signature is still verified against the pinned
//! certificate's public key, so a peer that copies the certificate without the
//! private key cannot complete the handshake.
//!
//! A pin mismatch surfaces as [`PinnedTlsError::FingerprintMismatch`], which
//! callers map to the `fingerprint_mismatch` reason. It is a hard failure,
//! never a prompt to continue.

use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{ring, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, ClientConfig, DigitallySignedStruct, OtherError, SignatureScheme};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;

/// Raw SHA-256 of the host certificate's DER encoding.
pub type Fingerprint = [u8; 32];

/// SHA-256 of a certificate's DER encoding.
pub fn fingerprint_of(cert_der: &[u8]) -> Fingerprint {
    Sha256::digest(cert_der).into()
}

/// Marker error placed inside the rustls error when the presented certificate
/// does not match the pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FingerprintMismatch;

impl fmt::Display for FingerprintMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("host certificate does not match the pinned fingerprint")
    }
}

impl std::error::Error for FingerprintMismatch {}

/// Errors from a pinned TLS connection attempt.
#[derive(Debug)]
pub enum PinnedTlsError {
    /// The host presented a certificate whose SHA-256 is not the pin.
    /// Maps to the `fingerprint_mismatch` reason.
    FingerprintMismatch,
    /// Any other TLS failure (for example a bad handshake signature).
    Tls(rustls::Error),
    /// The TCP connection or the socket failed.
    Io(io::Error),
}

impl PinnedTlsError {
    /// True when the failure is a pin mismatch.
    pub fn is_fingerprint_mismatch(&self) -> bool {
        matches!(self, PinnedTlsError::FingerprintMismatch)
    }

    /// Classifies an I/O error returned by a TLS stream built from
    /// [`pinned_client_config`] (tokio-rustls wraps rustls errors in
    /// `io::Error`).
    pub fn from_io(err: io::Error) -> Self {
        let tls = err
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<rustls::Error>())
            .cloned();
        match tls {
            Some(tls) => Self::from(tls),
            None => PinnedTlsError::Io(err),
        }
    }
}

impl From<rustls::Error> for PinnedTlsError {
    fn from(err: rustls::Error) -> Self {
        if is_mismatch(&err) {
            PinnedTlsError::FingerprintMismatch
        } else {
            PinnedTlsError::Tls(err)
        }
    }
}

impl fmt::Display for PinnedTlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PinnedTlsError::FingerprintMismatch => FingerprintMismatch.fmt(f),
            PinnedTlsError::Tls(e) => write!(f, "tls error: {e}"),
            PinnedTlsError::Io(e) => write!(f, "connection error: {e}"),
        }
    }
}

impl std::error::Error for PinnedTlsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PinnedTlsError::FingerprintMismatch => None,
            PinnedTlsError::Tls(e) => Some(e),
            PinnedTlsError::Io(e) => Some(e),
        }
    }
}

fn is_mismatch(err: &rustls::Error) -> bool {
    match err {
        rustls::Error::InvalidCertificate(CertificateError::Other(other)) => {
            other.0.downcast_ref::<FingerprintMismatch>().is_some()
        }
        _ => false,
    }
}

/// Certificate verifier that accepts only the pinned certificate.
#[derive(Debug)]
struct PinnedVerifier {
    pin: Fingerprint,
    algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let presented = fingerprint_of(end_entity.as_ref());
        if bool::from(presented.ct_eq(&self.pin)) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(CertificateError::Other(
                OtherError(Arc::new(FingerprintMismatch)),
            )))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

/// A rustls client configuration that trusts only the certificate whose
/// SHA-256 equals `pin`. Uses the `ring` crypto provider.
pub fn pinned_client_config(pin: Fingerprint) -> Arc<ClientConfig> {
    let provider = Arc::new(ring::default_provider());
    let verifier = PinnedVerifier {
        pin,
        algorithms: provider.signature_verification_algorithms,
    };
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("the ring provider supports the default protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();
    Arc::new(config)
}

/// A tokio-rustls connector over [`pinned_client_config`].
pub fn pinned_connector(pin: Fingerprint) -> TlsConnector {
    TlsConnector::from(pinned_client_config(pin))
}

/// Opens a TCP connection to `addr` and completes a pinned TLS handshake.
pub async fn connect(
    addr: SocketAddr,
    pin: Fingerprint,
) -> Result<TlsStream<TcpStream>, PinnedTlsError> {
    let tcp = TcpStream::connect(addr).await.map_err(PinnedTlsError::Io)?;
    // No hostname check: the IP is only used as the TLS server name.
    let server_name = ServerName::IpAddress(addr.ip().into());
    pinned_connector(pin)
        .connect(server_name, tcp)
        .await
        .map_err(PinnedTlsError::from_io)
}

#[cfg(test)]
mod tests {
    use super::*;

    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
    use rustls::server::{ClientHello, ResolvesServerCert};
    use rustls::sign::CertifiedKey;
    use rustls::ServerConfig;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_rustls::TlsAcceptor;

    struct TestCert {
        der: CertificateDer<'static>,
        key: PrivatePkcs8KeyDer<'static>,
    }

    fn make_cert() -> TestCert {
        let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
        let cert = rcgen::CertificateParams::new(Vec::<String>::new())
            .unwrap()
            .self_signed(&key)
            .unwrap();
        TestCert {
            der: cert.der().clone(),
            key: PrivatePkcs8KeyDer::from(key.serialize_der()),
        }
    }

    /// Serves one certificate with an arbitrary (possibly unrelated) key, so a
    /// test can present a certificate without its private key.
    #[derive(Debug)]
    struct FixedResolver(Arc<CertifiedKey>);

    impl ResolvesServerCert for FixedResolver {
        fn resolve(&self, _hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
            Some(self.0.clone())
        }
    }

    fn server_config(
        cert: &CertificateDer<'static>,
        key: &PrivatePkcs8KeyDer<'static>,
    ) -> ServerConfig {
        let provider = Arc::new(ring::default_provider());
        let signing_key = provider
            .key_provider
            .load_private_key(PrivateKeyDer::Pkcs8(key.clone_key()))
            .unwrap();
        let certified = CertifiedKey::new(vec![cert.clone()], signing_key);
        ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(FixedResolver(Arc::new(certified))))
    }

    /// Starts a one-shot TLS echo server; returns its address and a channel
    /// that reports whether the server-side handshake succeeded.
    async fn one_shot_server(config: ServerConfig) -> (SocketAddr, tokio::task::JoinHandle<bool>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let task = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            match acceptor.accept(tcp).await {
                Ok(mut tls) => {
                    let mut buf = [0u8; 4];
                    if tls.read_exact(&mut buf).await.is_ok() {
                        let _ = tls.write_all(&buf).await;
                        let _ = tls.shutdown().await;
                    }
                    true
                }
                Err(_) => false,
            }
        });
        (addr, task)
    }

    fn verifier(pin: Fingerprint) -> PinnedVerifier {
        PinnedVerifier {
            pin,
            algorithms: ring::default_provider().signature_verification_algorithms,
        }
    }

    fn verify(
        v: &PinnedVerifier,
        der: &CertificateDer<'_>,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let name = ServerName::try_from("ignored.invalid").unwrap();
        v.verify_server_cert(der, &[], &name, &[], UnixTime::now())
    }

    #[test]
    fn fingerprint_is_sha256_of_der() {
        let c = make_cert();
        let expected: [u8; 32] = Sha256::digest(c.der.as_ref()).into();
        assert_eq!(fingerprint_of(c.der.as_ref()), expected);
    }

    #[test]
    fn verifier_accepts_only_the_pinned_certificate() {
        let c = make_cert();
        let pin = fingerprint_of(c.der.as_ref());
        assert!(verify(&verifier(pin), &c.der).is_ok());

        // Any single differing byte, first or last, is a mismatch.
        for idx in [0usize, 31] {
            let mut wrong = pin;
            wrong[idx] ^= 0x01;
            let err = verify(&verifier(wrong), &c.der).unwrap_err();
            assert!(is_mismatch(&err), "byte {idx}: {err:?}");
            assert!(PinnedTlsError::from(err).is_fingerprint_mismatch());
        }

        // A different certificate never matches the pin.
        let other = make_cert();
        let err = verify(&verifier(pin), &other.der).unwrap_err();
        assert!(PinnedTlsError::from(err).is_fingerprint_mismatch());
    }

    #[test]
    fn verifier_ignores_server_name() {
        let c = make_cert();
        let v = verifier(fingerprint_of(c.der.as_ref()));
        for name in ["example.com", "192.168.1.20", "not-the-host.invalid"] {
            let name = ServerName::try_from(name).unwrap();
            assert!(v
                .verify_server_cert(&c.der, &[], &name, &[], UnixTime::now())
                .is_ok());
        }
    }

    #[test]
    fn io_errors_are_classified() {
        let mismatch = rustls::Error::InvalidCertificate(CertificateError::Other(OtherError(
            Arc::new(FingerprintMismatch),
        )));
        let wrapped = io::Error::new(io::ErrorKind::InvalidData, mismatch);
        assert!(PinnedTlsError::from_io(wrapped).is_fingerprint_mismatch());

        let other_tls = io::Error::new(
            io::ErrorKind::InvalidData,
            rustls::Error::InvalidCertificate(CertificateError::BadSignature),
        );
        assert!(matches!(
            PinnedTlsError::from_io(other_tls),
            PinnedTlsError::Tls(_)
        ));

        let plain = io::Error::new(io::ErrorKind::ConnectionRefused, "refused");
        assert!(matches!(
            PinnedTlsError::from_io(plain),
            PinnedTlsError::Io(_)
        ));
    }

    #[tokio::test]
    async fn handshake_succeeds_with_matching_pin() {
        let c = make_cert();
        let (addr, server) = one_shot_server(server_config(&c.der, &c.key)).await;
        let mut tls = connect(addr, fingerprint_of(c.der.as_ref())).await.unwrap();
        tls.write_all(b"ping").await.unwrap();
        let mut buf = [0u8; 4];
        tls.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"ping");
        assert!(server.await.unwrap());
    }

    #[tokio::test]
    async fn handshake_fails_with_typed_error_on_mismatch() {
        let c = make_cert();
        let other = make_cert();
        let (addr, server) = one_shot_server(server_config(&c.der, &c.key)).await;
        let err = connect(addr, fingerprint_of(other.der.as_ref()))
            .await
            .unwrap_err();
        assert!(err.is_fingerprint_mismatch(), "{err:?}");
        assert!(!server.await.unwrap(), "server handshake must not complete");
    }

    #[tokio::test]
    async fn handshake_signature_is_still_verified() {
        // The server presents the pinned certificate but signs with a key that
        // does not belong to it. The pin matches, so the failure must come
        // from signature verification, not from the pin check.
        let pinned = make_cert();
        let impostor = make_cert();
        let (addr, server) = one_shot_server(server_config(&pinned.der, &impostor.key)).await;
        let err = connect(addr, fingerprint_of(pinned.der.as_ref()))
            .await
            .unwrap_err();
        assert!(!err.is_fingerprint_mismatch(), "{err:?}");
        assert!(matches!(err, PinnedTlsError::Tls(_)), "{err:?}");
        assert!(!server.await.unwrap());
    }
}
