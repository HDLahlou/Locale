//! Container integrity checks in pure Rust, run before the OS decoder.
//!
//! ImageIO decodes a truncated JPEG or PNG without reporting an error (it
//! fills the missing rows), so fail-closed decoding needs its own check that
//! the container is complete: a JPEG must reach its end-of-image marker after
//! at least one scan, and a PNG must be a well-formed chain of chunks with
//! valid CRCs from `IHDR` to `IEND`. Data after the end is allowed (Ultra HDR
//! JPEGs append a gain-map image). HEIC needs no extra check: ImageIO refuses
//! incomplete HEIC files itself.

use crate::sniff::ImageFormat;

/// The container is incomplete or malformed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Malformed;

/// Check that `bytes`, already sniffed as `format`, form a complete container.
pub fn check(format: ImageFormat, bytes: &[u8]) -> Result<(), Malformed> {
    match format {
        ImageFormat::Jpeg => check_jpeg(bytes),
        ImageFormat::Png => check_png(bytes),
        ImageFormat::Heic => Ok(()),
    }
}

fn check_jpeg(b: &[u8]) -> Result<(), Malformed> {
    if !b.starts_with(&[0xFF, 0xD8]) {
        return Err(Malformed);
    }
    let mut pos = 2;
    let mut seen_frame = false;
    let mut seen_scan = false;
    loop {
        if *b.get(pos).ok_or(Malformed)? != 0xFF {
            return Err(Malformed);
        }
        // Skip fill bytes.
        while b.get(pos + 1) == Some(&0xFF) {
            pos += 1;
        }
        let marker = *b.get(pos + 1).ok_or(Malformed)?;
        pos += 2;
        match marker {
            0xD9 => {
                return if seen_frame && seen_scan {
                    Ok(())
                } else {
                    Err(Malformed)
                };
            }
            0x01 | 0xD0..=0xD7 => continue,
            0x00 | 0xD8 => return Err(Malformed),
            _ => {}
        }
        let len = usize::from(u16::from_be_bytes([
            *b.get(pos).ok_or(Malformed)?,
            *b.get(pos + 1).ok_or(Malformed)?,
        ]));
        if len < 2 || pos + len > b.len() {
            return Err(Malformed);
        }
        // SOF0-SOF15 except DHT (C4), JPG (C8) and DAC (CC).
        if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            seen_frame = true;
        }
        pos += len;
        if marker == 0xDA {
            seen_scan = true;
            // Entropy-coded data runs to the next marker that is not a
            // stuffed zero or a restart marker.
            loop {
                let ff = pos
                    + b.get(pos..)
                        .ok_or(Malformed)?
                        .iter()
                        .position(|&x| x == 0xFF)
                        .ok_or(Malformed)?;
                match b.get(ff + 1) {
                    None => return Err(Malformed),
                    Some(0x00) | Some(0xD0..=0xD7) => pos = ff + 2,
                    Some(_) => {
                        pos = ff;
                        break;
                    }
                }
            }
        }
    }
}

const PNG_SIGNATURE_LEN: usize = 8;

fn check_png(b: &[u8]) -> Result<(), Malformed> {
    let mut pos = PNG_SIGNATURE_LEN;
    let mut first = true;
    let mut seen_idat = false;
    loop {
        let header = b.get(pos..pos + 8).ok_or(Malformed)?;
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let kind = &header[4..8];
        if len > 0x7FFF_FFFF {
            return Err(Malformed);
        }
        let end = pos.checked_add(12 + len).ok_or(Malformed)?;
        let chunk = b.get(pos + 4..end).ok_or(Malformed)?;
        let (body, crc) = chunk.split_at(chunk.len() - 4);
        if crc32(body) != u32::from_be_bytes([crc[0], crc[1], crc[2], crc[3]]) {
            return Err(Malformed);
        }
        if first && kind != b"IHDR" {
            return Err(Malformed);
        }
        first = false;
        match kind {
            b"IDAT" => seen_idat = true,
            b"IEND" => return if seen_idat { Ok(()) } else { Err(Malformed) },
            _ => {}
        }
        pos = end;
    }
}

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &x in bytes {
        c = CRC_TABLE[((c ^ u32::from(x)) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_known_value() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }

    fn fixture(name: &str) -> Vec<u8> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/images/");
        std::fs::read(format!("{path}{name}")).unwrap()
    }

    #[test]
    fn complete_files_pass() {
        for name in ["plain.jpg", "orientation6.jpg", "p3.jpg", "gps_all.jpg"] {
            assert_eq!(check(ImageFormat::Jpeg, &fixture(name)), Ok(()), "{name}");
        }
        for name in ["plain.png", "gps_exif.png", "gps_xmp.png"] {
            assert_eq!(check(ImageFormat::Png, &fixture(name)), Ok(()), "{name}");
        }
    }

    #[test]
    fn truncated_files_fail() {
        assert_eq!(
            check(ImageFormat::Jpeg, &fixture("truncated.jpg")),
            Err(Malformed)
        );
        assert_eq!(
            check(ImageFormat::Png, &fixture("truncated.png")),
            Err(Malformed)
        );
        let jpg = fixture("plain.jpg");
        for cut in [3, 100, jpg.len() / 2, jpg.len() - 2, jpg.len() - 1] {
            assert_eq!(
                check(ImageFormat::Jpeg, &jpg[..cut]),
                Err(Malformed),
                "cut {cut}"
            );
        }
        let png = fixture("plain.png");
        for cut in [9, 40, png.len() / 2, png.len() - 1] {
            assert_eq!(
                check(ImageFormat::Png, &png[..cut]),
                Err(Malformed),
                "cut {cut}"
            );
        }
    }

    #[test]
    fn trailing_data_after_end_is_allowed() {
        let mut jpg = fixture("plain.jpg");
        jpg.extend_from_slice(&fixture("plain.jpg"));
        assert_eq!(check(ImageFormat::Jpeg, &jpg), Ok(()));
    }

    #[test]
    fn png_crc_error_fails() {
        let mut png = fixture("plain.png");
        let mid = png.len() / 2;
        png[mid] ^= 0x01;
        assert_eq!(check(ImageFormat::Png, &png), Err(Malformed));
    }
}
