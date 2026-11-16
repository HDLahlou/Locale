//! Per-event TLS identity: generation, fingerprint, storage (spec §5.1).

use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use host_core::listener::{
    fingerprint_base64url, fingerprint_of, IdentityError, TlsIdentity, CERT_FILE_NAME,
    KEY_FILE_NAME, TLS_DIR_NAME,
};
use sha2::{Digest, Sha256};
use x509_parser::oid_registry::{
    OID_EC_P256, OID_KEY_TYPE_EC_PUBLIC_KEY, OID_SIG_ECDSA_WITH_SHA256,
};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[test]
fn listener_identity_fingerprint_is_sha256_of_cert_der() {
    let id = TlsIdentity::generate(DAY).unwrap();
    let expected: [u8; 32] = Sha256::digest(id.cert_der().as_ref()).into();
    assert_eq!(id.fingerprint(), &expected);
    assert_eq!(fingerprint_of(id.cert_der().as_ref()), expected);

    let b64 = id.fingerprint_base64url();
    assert_eq!(b64, fingerprint_base64url(&expected));
    assert_eq!(b64.len(), 43, "32 bytes, base64url, no padding");
    assert!(!b64.contains('=') && !b64.contains('+') && !b64.contains('/'));
    assert_eq!(URL_SAFE_NO_PAD.decode(&b64).unwrap(), expected);
}

#[test]
fn listener_identity_is_self_signed_p256_for_the_given_lifetime() {
    for lifetime in [DAY, 7 * DAY + Duration::from_secs(3600)] {
        let id = TlsIdentity::generate(lifetime).unwrap();
        let (_, cert) = x509_parser::parse_x509_certificate(id.cert_der().as_ref()).unwrap();

        assert_eq!(cert.issuer(), cert.subject(), "self-signed");
        assert_eq!(
            cert.signature_algorithm.algorithm,
            OID_SIG_ECDSA_WITH_SHA256
        );
        let spki = &cert.tbs_certificate.subject_pki;
        assert_eq!(spki.algorithm.algorithm, OID_KEY_TYPE_EC_PUBLIC_KEY);
        let curve = spki
            .algorithm
            .parameters
            .as_ref()
            .and_then(|p| p.as_oid().ok())
            .unwrap();
        assert_eq!(curve, OID_EC_P256);

        let validity = cert.validity();
        let span = validity.not_after.timestamp() - validity.not_before.timestamp();
        let want = lifetime.as_secs() as i64;
        assert!((span - want).abs() <= 1, "span {span}s, want {want}s");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!((validity.not_before.timestamp() - now).abs() <= 5);
    }
}

#[test]
fn listener_identity_each_generation_is_distinct() {
    let a = TlsIdentity::generate(DAY).unwrap();
    let b = TlsIdentity::generate(DAY).unwrap();
    assert_ne!(a.fingerprint(), b.fingerprint());
}

#[test]
fn listener_identity_rejects_unrepresentable_lifetime() {
    let err = TlsIdentity::generate(Duration::MAX).unwrap_err();
    assert!(matches!(err, IdentityError::LifetimeTooLong), "{err:?}");
}

#[test]
fn listener_identity_save_and_load_keeps_fingerprint() {
    let event_dir = tempfile::tempdir().unwrap();
    let tls_dir = event_dir.path().join(TLS_DIR_NAME);
    let id = TlsIdentity::generate(DAY).unwrap();
    id.save(&tls_dir).unwrap();

    assert!(tls_dir.join(CERT_FILE_NAME).is_file());
    assert!(tls_dir.join(KEY_FILE_NAME).is_file());

    let loaded = TlsIdentity::load(&tls_dir).unwrap();
    assert_eq!(loaded.fingerprint(), id.fingerprint());
    assert_eq!(loaded.cert_der(), id.cert_der());
    loaded.server_config().unwrap();
}

#[cfg(unix)]
#[test]
fn listener_identity_key_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let event_dir = tempfile::tempdir().unwrap();
    let tls_dir = event_dir.path().join(TLS_DIR_NAME);
    let key_path = tls_dir.join(KEY_FILE_NAME);

    TlsIdentity::generate(DAY).unwrap().save(&tls_dir).unwrap();
    let mode = std::fs::metadata(&key_path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "fresh key file mode {mode:o}");

    // Saving over a key file with looser permissions tightens it again.
    std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644)).unwrap();
    TlsIdentity::generate(DAY).unwrap().save(&tls_dir).unwrap();
    let mode = std::fs::metadata(&key_path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "rewritten key file mode {mode:o}");
}

#[test]
fn listener_identity_load_rejects_missing_or_mismatched_files() {
    let empty = tempfile::tempdir().unwrap();
    assert!(matches!(
        TlsIdentity::load(empty.path()),
        Err(IdentityError::Io(_))
    ));

    // A certificate from one identity with the key of another is unusable.
    let a_dir = tempfile::tempdir().unwrap();
    let b_dir = tempfile::tempdir().unwrap();
    TlsIdentity::generate(DAY)
        .unwrap()
        .save(a_dir.path())
        .unwrap();
    TlsIdentity::generate(DAY)
        .unwrap()
        .save(b_dir.path())
        .unwrap();
    std::fs::copy(
        b_dir.path().join(KEY_FILE_NAME),
        a_dir.path().join(KEY_FILE_NAME),
    )
    .unwrap();
    assert!(matches!(
        TlsIdentity::load(a_dir.path()),
        Err(IdentityError::Tls(_))
    ));

    // A corrupt key is rejected.
    std::fs::write(b_dir.path().join(KEY_FILE_NAME), b"not a key").unwrap();
    assert!(TlsIdentity::load(b_dir.path()).is_err());
}

#[test]
fn listener_identity_debug_shows_no_key_or_certificate_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let id = TlsIdentity::generate(DAY).unwrap();
    id.save(dir.path()).unwrap();
    let key = std::fs::read(dir.path().join(KEY_FILE_NAME)).unwrap();

    let shown = format!("{id:?}");
    assert!(shown.contains(&id.fingerprint_base64url()));
    for secret in [&key[..], id.cert_der().as_ref()] {
        let b64 = URL_SAFE_NO_PAD.encode(secret);
        let std_b64 = base64::engine::general_purpose::STANDARD.encode(secret);
        let hex: String = secret.iter().map(|b| format!("{b:02x}")).collect();
        assert!(!shown.contains(&b64[..16]));
        assert!(!shown.contains(&std_b64[..16]));
        assert!(!shown.contains(&hex[..16]));
    }
    assert!(!shown.contains("cert_der") && !shown.contains("key_der"));
}
