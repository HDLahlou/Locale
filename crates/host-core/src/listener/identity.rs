//! Per-event TLS identity (spec §5.1): a self-signed ECDSA P-256 certificate,
//! its private key, and the fingerprint (SHA-256 of the certificate DER).

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rustls::crypto::ring;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

/// Raw SHA-256 of the certificate's DER encoding.
pub type Fingerprint = [u8; 32];

/// Name of the folder, inside the event folder, that holds the identity.
pub const TLS_DIR_NAME: &str = "tls";
/// Certificate file name inside the `tls/` folder (DER).
pub const CERT_FILE_NAME: &str = "cert.der";
/// Private key file name inside the `tls/` folder (PKCS#8 DER, mode 0600).
pub const KEY_FILE_NAME: &str = "key.der";

/// SHA-256 of a certificate's DER encoding.
pub fn fingerprint_of(cert_der: &[u8]) -> Fingerprint {
    Sha256::digest(cert_der).into()
}

/// The base64url form of a fingerprint, without padding (the `fp` value).
pub fn fingerprint_base64url(fingerprint: &Fingerprint) -> String {
    URL_SAFE_NO_PAD.encode(fingerprint)
}

/// Errors creating, storing or loading an identity.
#[derive(Debug)]
pub enum IdentityError {
    /// Certificate generation failed.
    Generate(String),
    /// The lifetime does not fit in a certificate validity period.
    LifetimeTooLong,
    /// Reading or writing the `tls/` folder failed.
    Io(io::Error),
    /// The stored key or certificate is not usable for TLS.
    Tls(rustls::Error),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::Generate(e) => write!(f, "certificate generation failed: {e}"),
            IdentityError::LifetimeTooLong => f.write_str("certificate lifetime is too long"),
            IdentityError::Io(e) => write!(f, "identity storage failed: {e}"),
            IdentityError::Tls(e) => write!(f, "identity is not usable for tls: {e}"),
        }
    }
}

impl std::error::Error for IdentityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IdentityError::Io(e) => Some(e),
            IdentityError::Tls(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for IdentityError {
    fn from(e: io::Error) -> Self {
        IdentityError::Io(e)
    }
}

/// A per-event TLS identity. `Debug` shows only the fingerprint; the key and
/// certificate bytes are never formatted.
pub struct TlsIdentity {
    cert_der: CertificateDer<'static>,
    key_der: PrivatePkcs8KeyDer<'static>,
    fingerprint: Fingerprint,
}

impl fmt::Debug for TlsIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TlsIdentity")
            .field("fingerprint", &self.fingerprint_base64url())
            .finish_non_exhaustive()
    }
}

impl TlsIdentity {
    /// Generates a self-signed ECDSA P-256 certificate valid from now for
    /// `lifetime` (the event's maximum life).
    pub fn generate(lifetime: Duration) -> Result<Self, IdentityError> {
        let gen_err = |e: rcgen::Error| IdentityError::Generate(e.to_string());
        let not_before = OffsetDateTime::now_utc();
        let not_after = time::Duration::try_from(lifetime)
            .ok()
            .and_then(|d| not_before.checked_add(d))
            .ok_or(IdentityError::LifetimeTooLong)?;

        let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).map_err(gen_err)?;
        let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(gen_err)?;
        params.distinguished_name = rcgen::DistinguishedName::new();
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "Locale host");
        params.not_before = not_before;
        params.not_after = not_after;
        let cert = params.self_signed(&key).map_err(gen_err)?;

        Self::from_der(
            cert.der().clone(),
            PrivatePkcs8KeyDer::from(key.serialize_der()),
        )
    }

    fn from_der(
        cert_der: CertificateDer<'static>,
        key_der: PrivatePkcs8KeyDer<'static>,
    ) -> Result<Self, IdentityError> {
        let identity = TlsIdentity {
            fingerprint: fingerprint_of(cert_der.as_ref()),
            cert_der,
            key_der,
        };
        // Reject a key that rustls cannot use or that does not match the
        // certificate before anyone relies on the identity.
        identity.server_config().map_err(IdentityError::Tls)?;
        Ok(identity)
    }

    /// The fingerprint: SHA-256 of the certificate DER.
    pub fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }

    /// The fingerprint as base64url without padding.
    pub fn fingerprint_base64url(&self) -> String {
        fingerprint_base64url(&self.fingerprint)
    }

    /// The certificate in DER form.
    pub fn cert_der(&self) -> &CertificateDer<'static> {
        &self.cert_der
    }

    /// Writes the identity into `tls_dir` (created if missing): the
    /// certificate as `cert.der` and the key as `key.der`, the key with
    /// owner-only permissions (0600) on Unix.
    pub fn save(&self, tls_dir: &Path) -> Result<(), IdentityError> {
        fs::create_dir_all(tls_dir)?;
        write_private(
            &tls_dir.join(KEY_FILE_NAME),
            self.key_der.secret_pkcs8_der(),
        )?;
        fs::write(tls_dir.join(CERT_FILE_NAME), self.cert_der.as_ref())?;
        Ok(())
    }

    /// Loads an identity written by [`TlsIdentity::save`].
    pub fn load(tls_dir: &Path) -> Result<Self, IdentityError> {
        let cert = fs::read(tls_dir.join(CERT_FILE_NAME))?;
        let key = fs::read(tls_dir.join(KEY_FILE_NAME))?;
        Self::from_der(CertificateDer::from(cert), PrivatePkcs8KeyDer::from(key))
    }

    /// A rustls server configuration presenting this identity, built on the
    /// `ring` provider.
    pub fn server_config(&self) -> Result<ServerConfig, rustls::Error> {
        ServerConfig::builder_with_provider(Arc::new(ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(
                vec![self.cert_der.clone()],
                PrivateKeyDer::Pkcs8(self.key_der.clone_key()),
            )
    }
}

/// Writes `bytes` to `path` so that only the owner can read it.
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    // The mode above applies only on creation; tighten a pre-existing file too.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(bytes)?;
    file.sync_all()
}
