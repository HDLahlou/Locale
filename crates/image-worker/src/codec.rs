//! The `ImageCodec` trait (DU-5) and the types it shares with the worker.
//!
//! One back end per operating system: `macos` (ImageIO) today, Windows in a
//! later task. Everything here runs inside the worker process; the host never
//! decodes untrusted bytes itself.

use std::fmt;

use serde::{Deserialize, Serialize};

pub use crate::sniff::ImageFormat;

/// Preview long edge in pixels (spec §10, fixed).
pub const PREVIEW_LONG_EDGE: u32 = 1024;
/// Preview JPEG quality, 0-100 (DU-14).
pub const PREVIEW_QUALITY: u8 = 80;
/// Compatible-copy JPEG quality, 0-100 (spec §10, fixed).
pub const COMPATIBLE_QUALITY: u8 = 92;

/// Facts recorded for an image that passed the full decode check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    /// Stored pixel width, before orientation is applied.
    pub width: u32,
    /// Stored pixel height, before orientation is applied.
    pub height: u32,
    pub format: ImageFormat,
    /// EXIF orientation 1-8; 1 when the file has none.
    pub orientation: u8,
    pub has_hdr_gain_map: bool,
}

/// An encoded output image (preview or compatible copy).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoded {
    /// JPEG bytes.
    pub bytes: Vec<u8>,
    /// Pixel width of the encoded image.
    pub width: u32,
    /// Pixel height of the encoded image.
    pub height: u32,
    /// Facts of the source image, from the decode check that ran first.
    pub facts: Facts,
}

/// Why a codec call failed. Carries no paths, names or image content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodecError {
    /// The bytes are not JPEG, PNG or HEIC/HEIF.
    NotAnImage,
    /// The content is an allowed type, but not the one the uploader claimed.
    FormatMismatch,
    /// The decoder reported any problem (fail closed).
    DecodeFailed,
    /// Encoding the output failed.
    EncodeFailed,
    /// This operating system has no codec back end yet.
    UnsupportedPlatform,
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CodecError::NotAnImage => "not an allowed image type",
            CodecError::FormatMismatch => "content does not match the claimed type",
            CodecError::DecodeFailed => "image failed to decode",
            CodecError::EncodeFailed => "image failed to encode",
            CodecError::UnsupportedPlatform => "no image codec on this platform",
        })
    }
}

impl std::error::Error for CodecError {}

/// Decode, inspect and convert images. Implementations must fail closed:
/// any decoder warning or error is an error.
pub trait ImageCodec {
    /// Full decode check. Sniffs the bytes, decodes every pixel of the primary
    /// image and returns its facts.
    fn decode(&self, bytes: &[u8]) -> Result<Facts, CodecError>;

    /// Preview: JPEG, quality [`PREVIEW_QUALITY`], long edge
    /// [`PREVIEW_LONG_EDGE`] (never upscaled), EXIF orientation applied to the
    /// pixels. Runs the full decode check first.
    fn preview(&self, bytes: &[u8]) -> Result<Encoded, CodecError>;

    /// Compatible copy: JPEG, quality [`COMPATIBLE_QUALITY`], full
    /// resolution, colour profile kept. Runs the full decode check first.
    fn compatible(&self, bytes: &[u8]) -> Result<Encoded, CodecError>;
}

/// The codec for the operating system this was built for.
pub fn platform_codec() -> Box<dyn ImageCodec> {
    #[cfg(target_os = "macos")]
    {
        Box::new(crate::macos::ImageIoCodec)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Box::new(Unsupported)
    }
}

/// Codec for operating systems without a back end: every call returns
/// [`CodecError::UnsupportedPlatform`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Unsupported;

impl ImageCodec for Unsupported {
    fn decode(&self, _bytes: &[u8]) -> Result<Facts, CodecError> {
        Err(CodecError::UnsupportedPlatform)
    }
    fn preview(&self, _bytes: &[u8]) -> Result<Encoded, CodecError> {
        Err(CodecError::UnsupportedPlatform)
    }
    fn compatible(&self, _bytes: &[u8]) -> Result<Encoded, CodecError> {
        Err(CodecError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_codec_returns_typed_error() {
        let c = Unsupported;
        assert_eq!(c.decode(b"x"), Err(CodecError::UnsupportedPlatform));
        assert_eq!(c.preview(b"x"), Err(CodecError::UnsupportedPlatform));
        assert_eq!(c.compatible(b"x"), Err(CodecError::UnsupportedPlatform));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn platform_codec_is_unsupported_here() {
        assert_eq!(
            platform_codec().decode(b"x"),
            Err(CodecError::UnsupportedPlatform)
        );
    }
}
