//! PNG location strip: `eXIf`, XMP in `iTXt`, and legacy raw-profile text
//! chunks. Each changed chunk keeps its length and gets a new CRC.

use super::{
    rawprofile, tiff, xmp, zlib, Format, LocationKind, MetaError, Plan, StripError, PNG_SIGNATURE,
};

const XMP_KEYWORD: &[u8] = b"XML:com.adobe.xmp";
const RAW_PROFILE_KEYWORD: &[u8] = b"Raw profile type ";

pub(super) fn plan(file: &[u8], plan: &mut Plan) -> Result<(), StripError> {
    let fail = |e: MetaError| e.at(Format::Png);
    let mut pos = PNG_SIGNATURE.len();
    loop {
        let len = super::be_u32(file, pos).ok_or(StripError::Truncated(Format::Png))?;
        if len > 0x7FFF_FFFF {
            return Err(StripError::Malformed {
                format: Format::Png,
                detail: "chunk length above 2^31-1",
            });
        }
        let data_start = pos + 8;
        let data_end = data_start + len as usize;
        let chunk_end = data_end + 4;
        if chunk_end > file.len() {
            return Err(StripError::Truncated(Format::Png));
        }
        let ctype = &file[pos + 4..pos + 8];
        let data = &file[data_start..data_end];
        let stripped = match ctype {
            b"IEND" => return Ok(()),
            b"eXIf" => tiff::strip(data)
                .map_err(fail)?
                .map(|d| (d, LocationKind::Exif)),
            b"tEXt" | b"zTXt" | b"iTXt" => text_chunk(ctype, data).map_err(fail)?,
            _ => None,
        };
        if let Some((new_data, kind)) = stripped {
            let mut body = Vec::with_capacity(len as usize + 8);
            body.extend_from_slice(ctype);
            body.extend_from_slice(&new_data);
            let crc = crc32(&body);
            body.extend_from_slice(&crc.to_be_bytes());
            plan.edit(file, pos + 4, body);
            plan.found(kind, pos..chunk_end);
        }
        pos = chunk_end;
    }
}

/// Where the text of a text chunk lives.
struct Text<'a> {
    keyword: &'a [u8],
    /// Offset of the text (or of the zlib stream) in the chunk data.
    at: usize,
    compressed: bool,
}

fn parse_text_chunk<'a>(ctype: &[u8], data: &'a [u8]) -> Result<Option<Text<'a>>, MetaError> {
    let Some(nul) = data.iter().position(|&b| b == 0) else {
        // No keyword separator: no structured metadata to find.
        return Ok(None);
    };
    let keyword = &data[..nul];
    let after = nul + 1;
    Ok(Some(match ctype {
        b"tEXt" => Text {
            keyword,
            at: after,
            compressed: false,
        },
        b"zTXt" => {
            if data.get(after) != Some(&0) {
                return Err(MetaError::Malformed("zTXt compression method"));
            }
            Text {
                keyword,
                at: after + 1,
                compressed: true,
            }
        }
        _ => {
            // iTXt: flag, method, language tag NUL, translated keyword NUL.
            let flag = *data
                .get(after)
                .ok_or(MetaError::Malformed("iTXt header truncated"))?;
            let method = *data
                .get(after + 1)
                .ok_or(MetaError::Malformed("iTXt header truncated"))?;
            let rest = after + 2;
            let lang_end = data[rest.min(data.len())..]
                .iter()
                .position(|&b| b == 0)
                .ok_or(MetaError::Malformed("iTXt language tag"))?
                + rest;
            let tk_end = data[lang_end + 1..]
                .iter()
                .position(|&b| b == 0)
                .ok_or(MetaError::Malformed("iTXt translated keyword"))?
                + lang_end
                + 1;
            let compressed = match (flag, method) {
                (0, _) => false,
                (1, 0) => true,
                _ => return Err(MetaError::Malformed("iTXt compression")),
            };
            Text {
                keyword,
                at: tk_end + 1,
                compressed,
            }
        }
    }))
}

/// Strips location data from a text chunk; returns the new chunk data.
fn text_chunk(ctype: &[u8], data: &[u8]) -> Result<Option<(Vec<u8>, LocationKind)>, MetaError> {
    let Some(t) = parse_text_chunk(ctype, data)? else {
        return Ok(None);
    };
    let is_xmp = t.keyword == XMP_KEYWORD;
    let profile = t.keyword.strip_prefix(RAW_PROFILE_KEYWORD);
    if !is_xmp && profile.is_none() {
        return Ok(None);
    }
    let process = |text: &[u8]| -> Result<Option<(Vec<u8>, LocationKind)>, MetaError> {
        match profile {
            Some(p) => rawprofile::strip(p, text),
            None => Ok(xmp::strip(text)?.map(|t| (t, LocationKind::Xmp))),
        }
    };
    let body = &data[t.at..];
    if !t.compressed {
        return Ok(process(body)?.map(|(new, kind)| {
            let mut out = data[..t.at].to_vec();
            out.extend_from_slice(&new);
            (out, kind)
        }));
    }
    let text = zlib::inflate(body).ok_or(MetaError::Malformed("compressed text chunk"))?;
    let Some((new, kind)) = process(&text)? else {
        return Ok(None);
    };
    let stream = zlib::deflate_exact(&new, body.len()).ok_or(MetaError::Unstrippable(
        "re-compressed text chunk does not fit",
    ))?;
    let mut out = data[..t.at].to_vec();
    out.extend_from_slice(&stream);
    Ok(Some((out, kind)))
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

/// CRC-32 as used by PNG (ISO 3309), over chunk type and data.
fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = CRC_TABLE[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::super::{rawprofile, strip_location, zlib};
    use super::*;

    pub(crate) fn chunk(ctype: &[u8], data: &[u8]) -> Vec<u8> {
        let mut c = (data.len() as u32).to_be_bytes().to_vec();
        c.extend_from_slice(ctype);
        c.extend_from_slice(data);
        c.extend_from_slice(&crc32(&[ctype, data].concat()).to_be_bytes());
        c
    }

    fn png(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut f = PNG_SIGNATURE.to_vec();
        f.extend(chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]));
        for c in chunks {
            f.extend_from_slice(c);
        }
        f.extend(chunk(b"IDAT", b"pixels"));
        f.extend(chunk(b"IEND", b""));
        f
    }

    /// Checks every chunk CRC in a PNG.
    fn crcs_ok(f: &[u8]) -> bool {
        let mut pos = 8;
        while pos + 12 <= f.len() {
            let len = u32::from_be_bytes(f[pos..pos + 4].try_into().unwrap()) as usize;
            let body = &f[pos + 4..pos + 8 + len];
            let crc = u32::from_be_bytes(f[pos + 8 + len..pos + 12 + len].try_into().unwrap());
            if crc32(body) != crc {
                return false;
            }
            pos += 12 + len;
        }
        true
    }

    #[test]
    fn crc_matches_known_value() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }

    #[test]
    fn text_raw_profile_exif_is_stripped() {
        let mut data = b"Raw profile type exif\0".to_vec();
        data.extend(rawprofile::tests::encode(
            "exif",
            &rawprofile::tests::exif_with_gps(),
        ));
        let f = png(&[chunk(b"tEXt", &data)]);
        let out = strip_location(&f).unwrap();
        assert!(out.changed && out.kinds.exif);
        assert_eq!(out.bytes.len(), f.len());
        assert!(crcs_ok(&out.bytes));
        assert!(!strip_location(&out.bytes).unwrap().changed);
    }

    #[test]
    fn ztxt_raw_profile_is_recompressed_to_the_same_length() {
        let text = rawprofile::tests::encode("exif", &rawprofile::tests::exif_with_gps());
        let mut data = b"Raw profile type exif\0\0".to_vec();
        data.extend(zlib::tests::compress_normal(&text));
        let f = png(&[chunk(b"zTXt", &data)]);
        let out = strip_location(&f).unwrap();
        assert!(out.changed && out.kinds.exif);
        assert_eq!(out.bytes.len(), f.len());
        assert!(crcs_ok(&out.bytes));
        let again = strip_location(&out.bytes).unwrap();
        assert!(!again.changed, "idempotent after re-compression");
        assert_eq!(again.bytes, out.bytes);
    }

    /// Data of the first chunk of type `ctype`.
    fn chunk_data<'a>(f: &'a [u8], ctype: &[u8]) -> &'a [u8] {
        let mut pos = 8;
        loop {
            let len = u32::from_be_bytes(f[pos..pos + 4].try_into().unwrap()) as usize;
            if &f[pos + 4..pos + 8] == ctype {
                return &f[pos + 8..pos + 8 + len];
            }
            pos += 12 + len;
        }
    }

    #[test]
    fn compressed_itxt_xmp_is_stripped() {
        let x = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><r xmlns:exif="http://ns.adobe.com/exif/1.0/" exif:GPSLatitude="12,20.736N" exif:DateTimeOriginal="2026-01-02T03:04:05"/></x:xmpmeta>"#;
        let header = b"XML:com.adobe.xmp\0\x01\x00en\0\0";
        let mut data = header.to_vec();
        data.extend(zlib::tests::compress_normal(x));
        let f = png(&[chunk(b"iTXt", &data)]);
        let out = strip_location(&f).unwrap();
        assert!(out.changed && out.kinds.xmp);
        assert!(crcs_ok(&out.bytes));
        let back = zlib::inflate(&chunk_data(&out.bytes, b"iTXt")[header.len()..]).unwrap();
        let s = String::from_utf8(back).unwrap();
        assert!(!s.contains("12,20.736N"));
        assert!(s.contains(r#"exif:DateTimeOriginal="2026-01-02T03:04:05""#));
    }

    #[test]
    fn compressed_chunk_that_cannot_fit_fails_closed() {
        // Incompressible text with a tiny location attribute: blanking it saves
        // fewer bytes than the padding blocks cost, so the chunk cannot keep
        // its length.
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let noise: String = (0..3000)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
                    [(seed % 64) as usize] as char
            })
            .collect();
        let x = format!(r#"<a b="{noise}" exif:GPS="1"/>"#);
        let mut data = b"XML:com.adobe.xmp\0\x01\x00\0\0".to_vec();
        data.extend(zlib::tests::compress_normal(x.as_bytes()));
        let f = png(&[chunk(b"iTXt", &data)]);
        let err = strip_location(&f).unwrap_err();
        assert!(
            matches!(
                err,
                StripError::Unstrippable {
                    format: Format::Png,
                    ..
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn unrelated_text_chunks_are_untouched() {
        let f = png(&[
            chunk(b"tEXt", b"Comment\0GPSLatitude 12"),
            chunk(b"tEXt", b"nokeyword"),
        ]);
        let out = strip_location(&f).unwrap();
        assert!(!out.changed);
        assert_eq!(out.bytes, f);
    }

    #[test]
    fn structural_errors() {
        let mut f = png(&[]);
        f.truncate(f.len() - 12);
        assert_eq!(
            strip_location(&f),
            Err(StripError::Truncated(Format::Png)),
            "no IEND"
        );
        let mut f = png(&[]);
        f[8..12].copy_from_slice(&0xFFFF_FFF0u32.to_be_bytes());
        assert!(matches!(
            strip_location(&f),
            Err(StripError::Malformed { .. })
        ));
        let mut f = png(&[]);
        f[8..12].copy_from_slice(&0x7FFF_FFF0u32.to_be_bytes());
        assert_eq!(strip_location(&f), Err(StripError::Truncated(Format::Png)));
        let f = png(&[chunk(b"zTXt", b"Raw profile type exif\0\x00garbage")]);
        assert!(matches!(
            strip_location(&f),
            Err(StripError::Malformed { .. })
        ));
        let f = png(&[chunk(b"zTXt", b"Raw profile type exif\0\x01")]);
        assert!(matches!(
            strip_location(&f),
            Err(StripError::Malformed { .. })
        ));
        let f = png(&[chunk(b"iTXt", b"XML:com.adobe.xmp\0\x00\x00en")]);
        assert!(matches!(
            strip_location(&f),
            Err(StripError::Malformed { .. })
        ));
    }
}
