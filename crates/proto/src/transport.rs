//! Transport, join URL, fingerprint and host code (digest §5.1, DU-11).

use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

use crate::encoding::{b64_decode_array, b64_encode, hex_lower, string_serde};
use crate::ParseError;

/// Protocol version. Sent in the `X-Locale-Proto` header on every request and in `/meta`.
pub const PROTO_VERSION: u32 = 1;

/// Default listening port. The host takes the next free port if it is taken.
pub const DEFAULT_PORT: u16 = 7420;

/// First byte of a TLS ClientHello. The listener sniffs it: `0x16` means TLS,
/// anything else plain HTTP.
pub const TLS_FIRST_BYTE: u8 = 0x16;

/// Prefix of every event ID.
pub const EVENT_ID_PREFIX: &str = "evt_";

/// Number of lowercase hex digits after [`EVENT_ID_PREFIX`].
pub const EVENT_ID_HEX_LEN: usize = 12;

/// An event ID: `evt_` followed by 12 lowercase hex digits, for example `evt_3f9a0c12b7de`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventId(String);

impl EventId {
    /// Parses and validates an event ID.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let hex = text
            .strip_prefix(EVENT_ID_PREFIX)
            .ok_or_else(|| ParseError::EventId(text.to_owned()))?;
        let valid = hex.len() == EVENT_ID_HEX_LEN
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if valid {
            Ok(EventId(text.to_owned()))
        } else {
            Err(ParseError::EventId(text.to_owned()))
        }
    }

    /// Builds an event ID from six random bytes supplied by the caller.
    pub fn from_random_bytes(bytes: [u8; 6]) -> Self {
        EventId(format!("{EVENT_ID_PREFIX}{}", hex_lower(&bytes)))
    }

    /// The ID as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for EventId {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EventId::parse(s)
    }
}

string_serde!(
    EventId,
    "EventId",
    "^evt_[0-9a-f]{12}$",
    "Event ID: evt_ followed by 12 lowercase hex digits."
);

/// Length of a join token in bytes (128 bits).
pub const JOIN_TOKEN_BYTES: usize = 16;

/// The join token: 128 random bits, base64url without padding (22 characters).
///
/// Generation happens in the host core; this type only carries and formats it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct JoinToken([u8; JOIN_TOKEN_BYTES]);

impl JoinToken {
    /// Wraps 16 random bytes.
    pub const fn from_bytes(bytes: [u8; JOIN_TOKEN_BYTES]) -> Self {
        JoinToken(bytes)
    }

    /// The raw bytes.
    pub const fn as_bytes(&self) -> &[u8; JOIN_TOKEN_BYTES] {
        &self.0
    }
}

impl fmt::Debug for JoinToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JoinToken(..)")
    }
}

impl fmt::Display for JoinToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&b64_encode(&self.0))
    }
}

impl FromStr for JoinToken {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        b64_decode_array(s)
            .map(JoinToken)
            .ok_or(ParseError::JoinToken)
    }
}

string_serde!(
    JoinToken,
    "JoinToken",
    "^[A-Za-z0-9_-]{22}$",
    "Join token: 128 random bits, base64url without padding."
);

/// Length of a certificate fingerprint in bytes (SHA-256).
pub const FINGERPRINT_BYTES: usize = 32;

/// Number of hex digits in the host code.
pub const HOST_CODE_HEX_LEN: usize = 12;

/// The certificate fingerprint: SHA-256 of the certificate DER, base64url without
/// padding (43 characters).
///
/// Hashing happens in other crates; build it from the digest with [`Fingerprint::from_digest`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint([u8; FINGERPRINT_BYTES]);

impl Fingerprint {
    /// Wraps a SHA-256 digest of the certificate DER.
    pub const fn from_digest(digest: [u8; FINGERPRINT_BYTES]) -> Self {
        Fingerprint(digest)
    }

    /// The raw digest.
    pub const fn as_bytes(&self) -> &[u8; FINGERPRINT_BYTES] {
        &self.0
    }

    /// The first 12 hex digits of the digest, uppercase, without separators
    /// (for example `4F2A9C1E77B0`). This is the mDNS `fp` value.
    pub fn host_code_hex(&self) -> String {
        let mut hex = hex_lower(&self.0[..HOST_CODE_HEX_LEN / 2]);
        hex.make_ascii_uppercase();
        hex
    }

    /// The host code shown for Nearby joins: the first 12 hex digits, uppercase,
    /// in three groups of four joined by `" · "`, for example `4F2A · 9C1E · 77B0`.
    pub fn host_code(&self) -> String {
        format_host_code(&self.host_code_hex())
    }
}

/// Groups 12 hex digits as `4F2A · 9C1E · 77B0`.
pub fn format_host_code(hex12: &str) -> String {
    let parts: Vec<&str> = [0..4, 4..8, 8..12]
        .into_iter()
        .filter_map(|r| hex12.get(r))
        .collect();
    parts.join(" \u{b7} ")
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fingerprint({self})")
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&b64_encode(&self.0))
    }
}

impl FromStr for Fingerprint {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        b64_decode_array(s)
            .map(Fingerprint)
            .ok_or(ParseError::Fingerprint)
    }
}

string_serde!(
    Fingerprint,
    "Fingerprint",
    "^[A-Za-z0-9_-]{43}$",
    "SHA-256 of the certificate DER, base64url without padding."
);

/// The join URL in the QR code:
/// `http://<host-ip>:<port>/e/<event-id>#t=<join-token>&fp=<cert-fingerprint>`.
///
/// The token and fingerprint are in the fragment, so a browser never sends them
/// to the server. An IPv6 host is written in brackets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinUrl {
    /// The host's LAN address.
    pub host: IpAddr,
    /// The host's port.
    pub port: u16,
    /// The event.
    pub event_id: EventId,
    /// The join token (`t`).
    pub join_token: JoinToken,
    /// The certificate fingerprint (`fp`).
    pub fingerprint: Fingerprint,
}

impl JoinUrl {
    /// Parses a join URL. The fragment must hold `t` and `fp` exactly once each, in
    /// any order; other fragment keys are ignored.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let rest = text
            .strip_prefix("http://")
            .ok_or(ParseError::JoinUrl("scheme must be http://"))?;
        let (before, fragment) = rest
            .split_once('#')
            .ok_or(ParseError::JoinUrl("missing fragment"))?;
        let (authority, path) = before
            .split_once('/')
            .ok_or(ParseError::JoinUrl("missing path"))?;
        let addr: SocketAddr = authority
            .parse()
            .map_err(|_| ParseError::JoinUrl("host must be <ip>:<port>"))?;
        if addr.port() == 0 {
            return Err(ParseError::JoinUrl("port must not be 0"));
        }
        let id = path
            .strip_prefix("e/")
            .ok_or(ParseError::JoinUrl("path must be /e/<event-id>"))?;
        let event_id =
            EventId::parse(id).map_err(|_| ParseError::JoinUrl("path must be /e/<event-id>"))?;

        let mut token = None;
        let mut fp = None;
        for pair in fragment.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let slot = match key {
                "t" => &mut token,
                "fp" => &mut fp,
                _ => continue,
            };
            if slot.replace(value).is_some() {
                return Err(ParseError::JoinUrl("repeated fragment key"));
            }
        }
        let join_token = token
            .ok_or(ParseError::JoinUrl("missing t"))?
            .parse()
            .map_err(|_| ParseError::JoinUrl("invalid t"))?;
        let fingerprint = fp
            .ok_or(ParseError::JoinUrl("missing fp"))?
            .parse()
            .map_err(|_| ParseError::JoinUrl("invalid fp"))?;
        Ok(JoinUrl {
            host: addr.ip(),
            port: addr.port(),
            event_id,
            join_token,
            fingerprint,
        })
    }

    /// The guest page URL without the fragment: `http://<host-ip>:<port>/e/<event-id>`.
    pub fn page_url(&self) -> String {
        format!(
            "http://{}/e/{}",
            SocketAddr::new(self.host, self.port),
            self.event_id
        )
    }
}

impl fmt::Display for JoinUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}#t={}&fp={}",
            self.page_url(),
            self.join_token,
            self.fingerprint
        )
    }
}

impl FromStr for JoinUrl {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        JoinUrl::parse(s)
    }
}

/// Prefix of the app deep link (DU-11).
pub const DEEP_LINK_PREFIX: &str = "locale://join?u=";

/// Characters left unescaped in the deep link's `u` value: ASCII letters, digits
/// and `-._~` (RFC 3986 unreserved). Everything else is percent-encoded.
const DEEP_LINK_ESCAPE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// The deep link from the guest page into the app (DU-11):
/// `locale://join?u=<url-encoded join URL>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinDeepLink {
    /// The join URL it carries.
    pub join_url: JoinUrl,
}

impl JoinDeepLink {
    /// Parses a deep link. The query must hold `u` exactly once; other keys are ignored.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let query = text
            .strip_prefix("locale://join?")
            .ok_or(ParseError::DeepLink("must start with locale://join?"))?;
        let mut found = None;
        for pair in query.split('&') {
            if let Some(value) = pair.strip_prefix("u=") {
                if found.replace(value).is_some() {
                    return Err(ParseError::DeepLink("repeated u"));
                }
            }
        }
        let encoded = found.ok_or(ParseError::DeepLink("missing u"))?;
        let decoded = percent_decode_str(encoded)
            .decode_utf8()
            .map_err(|_| ParseError::DeepLink("u is not UTF-8"))?;
        let join_url =
            JoinUrl::parse(&decoded).map_err(|_| ParseError::DeepLink("u is not a join URL"))?;
        Ok(JoinDeepLink { join_url })
    }
}

impl fmt::Display for JoinDeepLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let url = self.join_url.to_string();
        write!(
            f,
            "{DEEP_LINK_PREFIX}{}",
            utf8_percent_encode(&url, DEEP_LINK_ESCAPE)
        )
    }
}

impl FromStr for JoinDeepLink {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        JoinDeepLink::parse(s)
    }
}
