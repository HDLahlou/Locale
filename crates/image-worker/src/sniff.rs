//! Content sniffing by magic bytes. Pure Rust, no OS calls, so the phone
//! client can run the same check before it uploads.
//!
//! Only JPEG, PNG and HEIC/HEIF are accepted. For ISO-BMFF files the `ftyp`
//! box must come first and name an HEVC image brand, as the major brand or as
//! one of the compatible brands. Everything else is rejected.

use std::fmt;

use serde::{Deserialize, Serialize};

/// An allowed image format, as classified from the bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    Jpeg,
    Png,
    /// HEIC and HEVC-coded HEIF.
    Heic,
}

impl ImageFormat {
    /// Lower-case name, as used in JSON and telemetry.
    pub fn as_str(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::Png => "png",
            ImageFormat::Heic => "heic",
        }
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The bytes are not an allowed image type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAnImage;

impl fmt::Display for NotAnImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not an allowed image type")
    }
}

impl std::error::Error for NotAnImage {}

/// Number of leading bytes `sniff` needs at most. Callers that read only a
/// header should read at least this many bytes (or the whole file if shorter).
pub const SNIFF_LEN: usize = 256;

const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];
const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];

/// HEVC image brands (ISO/IEC 23008-12). A file is HEIC/HEIF for us only if
/// one of these appears as the major or a compatible brand. Generic `mif1`
/// alone is not enough: it is also used by AVIF and other codecs.
const HEVC_BRANDS: &[&[u8; 4]] = &[
    b"heic", b"heix", b"heim", b"heis", b"hevc", b"hevx", b"hevm", b"hevs",
];

/// Classify `bytes` by content. The file name and claimed type play no part.
pub fn sniff(bytes: &[u8]) -> Result<ImageFormat, NotAnImage> {
    if bytes.starts_with(JPEG_MAGIC) {
        return Ok(ImageFormat::Jpeg);
    }
    if bytes.starts_with(PNG_MAGIC) {
        return Ok(ImageFormat::Png);
    }
    if is_heic(bytes) {
        return Ok(ImageFormat::Heic);
    }
    Err(NotAnImage)
}

fn is_heic(bytes: &[u8]) -> bool {
    // ftyp box: u32 size, "ftyp", major brand, u32 minor version,
    // then compatible brands of 4 bytes each.
    if bytes.len() < 16 || &bytes[4..8] != b"ftyp" {
        return false;
    }
    let size = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    if size < 16 || !(size - 16).is_multiple_of(4) || size > bytes.len() || size > SNIFF_LEN {
        return false;
    }
    let major: [u8; 4] = [bytes[8], bytes[9], bytes[10], bytes[11]];
    let (compatible, _) = bytes[16..size].as_chunks::<4>();
    std::iter::once(&major)
        .chain(compatible)
        .any(|brand| HEVC_BRANDS.contains(&brand))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ftyp(major: &[u8; 4], compatible: &[&[u8; 4]]) -> Vec<u8> {
        let size = 16 + 4 * compatible.len();
        let mut v = (size as u32).to_be_bytes().to_vec();
        v.extend_from_slice(b"ftyp");
        v.extend_from_slice(major);
        v.extend_from_slice(&[0, 0, 0, 0]);
        for c in compatible {
            v.extend_from_slice(*c);
        }
        v.extend_from_slice(&[0, 0, 0, 8, b'm', b'e', b't', b'a']);
        v
    }

    #[test]
    fn magic_bytes() {
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Ok(ImageFormat::Jpeg));
        assert_eq!(sniff(PNG_MAGIC), Ok(ImageFormat::Png));
        assert_eq!(sniff(&[0xFF, 0xD8]), Err(NotAnImage));
        assert_eq!(sniff(b""), Err(NotAnImage));
        assert_eq!(sniff(b"GIF89a......"), Err(NotAnImage));
    }

    #[test]
    fn heic_brands() {
        assert_eq!(sniff(&ftyp(b"heic", &[b"mif1"])), Ok(ImageFormat::Heic));
        assert_eq!(sniff(&ftyp(b"mif1", &[b"heic"])), Ok(ImageFormat::Heic));
        assert_eq!(sniff(&ftyp(b"heix", &[])), Ok(ImageFormat::Heic));
        assert_eq!(sniff(&ftyp(b"mif1", &[b"miaf"])), Err(NotAnImage));
        assert_eq!(sniff(&ftyp(b"avif", &[b"mif1", b"miaf"])), Err(NotAnImage));
        assert_eq!(sniff(&ftyp(b"mp42", &[b"isom"])), Err(NotAnImage));
    }

    #[test]
    fn malformed_ftyp_is_rejected() {
        let mut v = ftyp(b"heic", &[b"mif1"]);
        v[3] = 200; // box larger than the data
        assert_eq!(sniff(&v), Err(NotAnImage));
        let mut v = ftyp(b"heic", &[b"mif1"]);
        v[3] = 18; // not a whole number of brands
        assert_eq!(sniff(&v), Err(NotAnImage));
        let mut v = ftyp(b"heic", &[]);
        v[3] = 8; // shorter than the header
        assert_eq!(sniff(&v), Err(NotAnImage));
    }
}
