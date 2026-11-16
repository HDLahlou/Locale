//! Legacy ImageMagick "raw profile" text (PNG `tEXt`/`zTXt`/`iTXt` chunks
//! with keyword `Raw profile type <name>`): a header line, a decimal length,
//! then the profile bytes in hex. The hex digits are rewritten in place, so the
//! text keeps its length and layout.

use super::{iptc, tiff, xmp, LocationKind, MetaError};

const XMP_APP1_HEADER: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const PHOTOSHOP_HEADER: &[u8] = b"Photoshop 3.0\0";

/// Strips location data from a raw-profile text of type `profile` (the part
/// of the keyword after `Raw profile type `). Profile types that cannot hold
/// location data (`icc`, `icm`, ...) are ignored. Returns the modified text and
/// the kind of location data found.
pub(super) fn strip(
    profile: &[u8],
    text: &[u8],
) -> Result<Option<(Vec<u8>, LocationKind)>, MetaError> {
    let profile = profile.to_ascii_lowercase();
    if !matches!(
        profile.as_slice(),
        b"exif" | b"app1" | b"xmp" | b"iptc" | b"8bim"
    ) {
        return Ok(None);
    }
    let digits = hex_digits(text)?;
    let decoded: Vec<u8> = digits
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| (hex_value(text[p[0]]) << 4) | hex_value(text[p[1]]))
        .collect();

    let Some((start, new, kind)) = strip_decoded(&profile, &decoded)? else {
        return Ok(None);
    };
    let mut out = text.to_vec();
    for (i, &byte) in new.iter().enumerate() {
        let at = start + i;
        if decoded[at] == byte {
            continue;
        }
        let (hi, lo) = (digits[2 * at], digits[2 * at + 1]);
        let upper = text[hi].is_ascii_uppercase() || text[lo].is_ascii_uppercase();
        out[hi] = hex_char(byte >> 4, upper);
        out[lo] = hex_char(byte & 0xF, upper);
    }
    Ok(Some((out, kind)))
}

/// Runs the right metadata strip on the decoded profile. Returns where the
/// stripped block starts in `decoded`, its new bytes, and its kind.
fn strip_decoded(
    profile: &[u8],
    d: &[u8],
) -> Result<Option<(usize, Vec<u8>, LocationKind)>, MetaError> {
    let tiff_at = if d.starts_with(b"Exif\0") && d.len() >= 6 {
        Some(6)
    } else if d.starts_with(b"II*\0") || d.starts_with(b"MM\0*") {
        Some(0)
    } else {
        None
    };
    let (start, result, kind) = match profile {
        b"exif" | b"app1" => {
            if let Some(at) = tiff_at {
                (at, tiff::strip(&d[at..])?, LocationKind::Exif)
            } else if d.starts_with(XMP_APP1_HEADER) {
                let at = XMP_APP1_HEADER.len();
                (at, xmp::strip(&d[at..])?, LocationKind::Xmp)
            } else {
                return Ok(None);
            }
        }
        b"xmp" => (0, xmp::strip(d)?, LocationKind::Xmp),
        _ => {
            // iptc or 8bim: bare IIM data, or Photoshop resources with or
            // without the APP13 header.
            if d.first() == Some(&0x1C) {
                (0, iptc::strip_iim(d)?, LocationKind::Iptc)
            } else if d.starts_with(PHOTOSHOP_HEADER) {
                let at = PHOTOSHOP_HEADER.len();
                (at, iptc::strip_irb(&d[at..])?, LocationKind::Iptc)
            } else if d.starts_with(b"8BIM") {
                (0, iptc::strip_irb(d)?, LocationKind::Iptc)
            } else {
                return Ok(None);
            }
        }
    };
    Ok(result.map(|new| (start, new, kind)))
}

/// Positions of the hex digits after the header (`\n<type>\n<len>\n`).
fn hex_digits(text: &[u8]) -> Result<Vec<usize>, MetaError> {
    const BAD: MetaError = MetaError::Malformed("raw profile header");
    if text.first() != Some(&b'\n') {
        return Err(BAD);
    }
    let type_end = text[1..].iter().position(|&b| b == b'\n').ok_or(BAD)? + 1;
    let mut i = type_end + 1;
    while text.get(i).is_some_and(|b| matches!(b, b' ' | b'\t')) {
        i += 1;
    }
    let digits_start = i;
    while text.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i == digits_start || text.get(i) != Some(&b'\n') {
        return Err(BAD);
    }
    let mut out = Vec::new();
    for (j, &b) in text.iter().enumerate().skip(i + 1) {
        if b.is_ascii_hexdigit() {
            out.push(j);
        } else if !b.is_ascii_whitespace() && b != 0 {
            return Err(MetaError::Malformed("raw profile hex data"));
        }
    }
    if out.len() % 2 != 0 {
        return Err(MetaError::Malformed("raw profile hex data"));
    }
    Ok(out)
}

fn hex_value(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

fn hex_char(v: u8, upper: bool) -> u8 {
    let lower = b"0123456789abcdef"[v as usize];
    if upper {
        lower.to_ascii_uppercase()
    } else {
        lower
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Encodes bytes as an ImageMagick raw profile (72 hex digits per line).
    pub(crate) fn encode(profile: &str, data: &[u8]) -> Vec<u8> {
        let mut s = format!("\n{profile}\n{:8}\n", data.len());
        for line in data.chunks(36) {
            for b in line {
                s.push_str(&format!("{b:02x}"));
            }
            s.push('\n');
        }
        s.into_bytes()
    }

    pub(crate) fn exif_with_gps() -> Vec<u8> {
        let mut d = b"Exif\0\0".to_vec();
        d.extend(tiff::tests::build(
            80,
            8,
            &[
                (8, vec![(0x0112, 3, 1, 6), (0x8825, 4, 1, 40)], 0),
                (40, vec![(1, 2, 2, 0x4E00_0000)], 0),
            ],
            &[],
        ));
        d
    }

    #[test]
    fn exif_profile_is_stripped_in_place() {
        let text = encode("exif", &exif_with_gps());
        let (out, kind) = strip(b"exif", &text).unwrap().unwrap();
        assert_eq!(kind, LocationKind::Exif);
        assert_eq!(out.len(), text.len());
        // Same layout: newlines in the same places.
        let nl = |t: &[u8]| {
            t.iter()
                .enumerate()
                .filter(|(_, &b)| b == b'\n')
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        };
        assert_eq!(nl(&out), nl(&text));
        assert_eq!(strip(b"exif", &out).unwrap(), None);
    }

    #[test]
    fn upper_case_hex_stays_upper_case() {
        let text = encode("exif", &exif_with_gps()).to_ascii_uppercase();
        let (out, _) = strip(b"EXIF", &text).unwrap().unwrap();
        assert!(!out.iter().any(u8::is_ascii_lowercase));
    }

    #[test]
    fn xmp_and_iptc_profiles() {
        let x = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><r exif:GPSLatitude="1,2N"/></x:xmpmeta>"#;
        let (_, kind) = strip(b"xmp", &encode("xmp", x)).unwrap().unwrap();
        assert_eq!(kind, LocationKind::Xmp);
        let mut app1 = XMP_APP1_HEADER.to_vec();
        app1.extend_from_slice(x);
        assert!(strip(b"APP1", &encode("APP1", &app1)).unwrap().is_some());

        let iim = [0x1C, 2, 90, 0, 3, b'a', b'b', b'c'];
        let (out, kind) = strip(b"iptc", &encode("iptc", &iim)).unwrap().unwrap();
        assert_eq!(kind, LocationKind::Iptc);
        assert!(!String::from_utf8_lossy(&out).contains("616263"));
        let mut irb = PHOTOSHOP_HEADER.to_vec();
        irb.extend_from_slice(b"8BIM\x04\x04\x00\x00\x00\x00\x00\x08");
        irb.extend_from_slice(&iim);
        assert!(strip(b"8bim", &encode("8bim", &irb)).unwrap().is_some());
    }

    #[test]
    fn other_profiles_and_unknown_payloads_are_ignored() {
        assert_eq!(strip(b"icc", b"garbage").unwrap(), None);
        assert_eq!(
            strip(b"exif", &encode("exif", b"nothing here")).unwrap(),
            None
        );
    }

    #[test]
    fn malformed_profiles_fail_closed() {
        assert!(strip(b"exif", b"no header").is_err());
        assert!(strip(b"exif", b"\nexif\n  x\n00").is_err());
        assert!(
            strip(b"exif", b"\nexif\n  1\n0").is_err(),
            "odd digit count"
        );
        assert!(strip(b"exif", b"\nexif\n  1\n0g").is_err(), "not hex");
    }
}
