//! Byte-to-text helpers: unpadded base64url and hex.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

/// Encodes bytes as base64url without padding.
pub(crate) fn b64_encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes unpadded base64url. Padding and non-canonical trailing bits are rejected,
/// so every accepted string has exactly one byte form and round-trips unchanged.
pub(crate) fn b64_decode(text: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(text).ok()
}

/// Decodes unpadded base64url into exactly `N` bytes.
pub(crate) fn b64_decode_array<const N: usize>(text: &str) -> Option<[u8; N]> {
    b64_decode(text)?.try_into().ok()
}

/// Lowercase hex.
pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

/// Implements `Serialize`, `Deserialize` and `JsonSchema` for a type with
/// `Display` and `FromStr<Err = ParseError>`, as a JSON string with a pattern.
macro_rules! string_serde {
    ($ty:ty, $name:literal, $pattern:literal, $desc:literal) => {
        impl ::serde::Serialize for $ty {
            fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }
        impl<'de> ::serde::Deserialize<'de> for $ty {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = <::std::string::String as ::serde::Deserialize>::deserialize(d)?;
                s.parse().map_err(::serde::de::Error::custom)
            }
        }
        impl ::schemars::JsonSchema for $ty {
            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                $name.into()
            }
            fn json_schema(_: &mut ::schemars::SchemaGenerator) -> ::schemars::Schema {
                ::schemars::json_schema!({
                    "type": "string",
                    "pattern": $pattern,
                    "description": $desc,
                })
            }
        }
    };
}
pub(crate) use string_serde;
