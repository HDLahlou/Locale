//! mDNS advertisement (digest §5.6, DU-32).
//!
//! Service type `_locale._tcp.local.`, instance name = event ID. Advertised only
//! for open and password events (MVP §5.3 step 1). Counts come from `/meta`.

use crate::model::AccessType;
use crate::transport::{EventId, HOST_CODE_HEX_LEN};
use crate::ParseError;

/// The mDNS service type.
pub const SERVICE_TYPE: &str = "_locale._tcp.local.";

/// TXT keys, in advertisement order.
pub mod keys {
    /// Protocol version.
    pub const V: &str = "v";
    /// Event ID.
    pub const ID: &str = "id";
    /// Event name.
    pub const NAME: &str = "name";
    /// Host display name.
    pub const HOST: &str = "host";
    /// Access type: `open` or `password`.
    pub const ACCESS: &str = "access";
    /// Start time, Unix milliseconds in decimal.
    pub const ST: &str = "st";
    /// End time, Unix milliseconds in decimal.
    pub const EN: &str = "en";
    /// Host code: 12 uppercase hex digits.
    pub const FP: &str = "fp";
    /// All keys.
    pub const ALL: &[&str] = &[V, ID, NAME, HOST, ACCESS, ST, EN, FP];
}

/// Whether an event with this access type is advertised.
pub const fn is_advertised(access: AccessType) -> bool {
    matches!(access, AccessType::Open | AccessType::Password)
}

/// The TXT record of an advertised event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxtRecord {
    /// Protocol version (`v`), [`crate::PROTO_VERSION`] when advertised by this version.
    pub v: u32,
    /// Event ID (`id`); also the instance name.
    pub id: EventId,
    /// Event name (`name`).
    pub name: String,
    /// Host display name (`host`).
    pub host: String,
    /// Access type (`access`): `open` or `password` only.
    pub access: AccessType,
    /// Start time (`st`).
    pub st: i64,
    /// End time (`en`).
    pub en: i64,
    /// Host code (`fp`): the first 12 hex digits of the fingerprint, uppercase,
    /// no separators. See [`crate::transport::Fingerprint::host_code_hex`].
    pub fp: String,
}

fn valid_host_code_hex(s: &str) -> bool {
    s.len() == HOST_CODE_HEX_LEN
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
}

impl TxtRecord {
    /// The mDNS instance name: the event ID.
    pub fn instance_name(&self) -> &str {
        self.id.as_str()
    }

    /// Key-value pairs in [`keys::ALL`] order.
    pub fn to_pairs(&self) -> Vec<(&'static str, String)> {
        vec![
            (keys::V, self.v.to_string()),
            (keys::ID, self.id.to_string()),
            (keys::NAME, self.name.clone()),
            (keys::HOST, self.host.clone()),
            (keys::ACCESS, self.access.as_str().to_owned()),
            (keys::ST, self.st.to_string()),
            (keys::EN, self.en.to_string()),
            (keys::FP, self.fp.clone()),
        ]
    }

    /// Parses key-value pairs. Every key is required; unknown keys are ignored;
    /// a repeated key keeps the first value (RFC 6763 §6.4).
    pub fn from_pairs<I, K, V>(pairs: I) -> Result<Self, ParseError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut found: [Option<String>; 8] = Default::default();
        for (k, v) in pairs {
            if let Some(i) = keys::ALL.iter().position(|key| *key == k.as_ref()) {
                found[i].get_or_insert_with(|| v.as_ref().to_owned());
            }
        }
        let mut take = |i: usize| found[i].take().ok_or(ParseError::TxtRecord("missing key"));
        let v = take(0)?
            .parse()
            .map_err(|_| ParseError::TxtRecord("v is not a number"))?;
        let id = EventId::parse(&take(1)?).map_err(|_| ParseError::TxtRecord("invalid id"))?;
        let name = take(2)?;
        let host = take(3)?;
        let access: AccessType = take(4)?
            .parse()
            .map_err(|_| ParseError::TxtRecord("invalid access"))?;
        if !is_advertised(access) {
            return Err(ParseError::TxtRecord("access must be open or password"));
        }
        let st = take(5)?
            .parse()
            .map_err(|_| ParseError::TxtRecord("st is not a number"))?;
        let en = take(6)?
            .parse()
            .map_err(|_| ParseError::TxtRecord("en is not a number"))?;
        let fp = take(7)?;
        if !valid_host_code_hex(&fp) {
            return Err(ParseError::TxtRecord("fp must be 12 uppercase hex digits"));
        }
        Ok(TxtRecord {
            v,
            id,
            name,
            host,
            access,
            st,
            en,
            fp,
        })
    }
}
