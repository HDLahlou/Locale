//! HEIC/HEIF location strip: finds the `Exif` and XMP items through the
//! `meta` box and overwrites their GPS data in place, so no box offset or item
//! location changes (DU-37).

use std::ops::Range;

use super::{
    be_u16, be_u32, be_u64, be_uint, tiff, xmp, Format, LocationKind, MetaError, Pieces, Plan,
    StripError,
};

/// Most boxes read at one level; more is treated as malformed.
const MAX_BOXES: usize = 100_000;

fn malformed(detail: &'static str) -> StripError {
    StripError::Malformed {
        format: Format::Heic,
        detail,
    }
}

fn unstrippable(detail: &'static str) -> StripError {
    StripError::Unstrippable {
        format: Format::Heic,
        detail,
    }
}

#[derive(Debug, Clone)]
struct BoxRef {
    btype: [u8; 4],
    /// Content after the header.
    body: Range<usize>,
}

/// Reads the boxes in `file[range]`. A box that runs past the range is
/// [`StripError::Truncated`] at the top level and malformed inside a box.
fn boxes(file: &[u8], range: Range<usize>, top: bool) -> Result<Vec<BoxRef>, StripError> {
    let overrun = || {
        if top {
            StripError::Truncated(Format::Heic)
        } else {
            malformed("box runs past its parent")
        }
    };
    let mut out = Vec::new();
    let mut pos = range.start;
    while pos < range.end {
        if out.len() >= MAX_BOXES {
            return Err(malformed("too many boxes"));
        }
        let size32 = be_u32(file, pos)
            .filter(|_| pos + 8 <= range.end)
            .ok_or_else(overrun)?;
        let mut btype = [0u8; 4];
        btype.copy_from_slice(&file[pos + 4..pos + 8]);
        let (size, header) = match size32 {
            0 => ((range.end - pos) as u64, 8),
            1 => (
                be_u64(file, pos + 8)
                    .filter(|_| pos + 16 <= range.end)
                    .ok_or_else(overrun)?,
                16,
            ),
            n => (u64::from(n), 8),
        };
        if size < header as u64 {
            return Err(malformed("box size smaller than its header"));
        }
        let end = (pos as u64)
            .checked_add(size)
            .filter(|&e| e <= range.end as u64)
            .ok_or_else(overrun)? as usize;
        out.push(BoxRef {
            btype,
            body: pos + header..end,
        });
        pos = end;
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Exif,
    Xmp,
}

#[derive(Debug)]
struct Item {
    id: u32,
    kind: ItemKind,
}

#[derive(Debug)]
struct Location {
    method: u8,
    data_ref: u16,
    base: u64,
    extents: Vec<(u64, u64)>,
}

pub(super) fn plan(file: &[u8], plan: &mut Plan) -> Result<(), StripError> {
    let top = boxes(file, 0..file.len(), true)?;
    let meta = top
        .iter()
        .find(|b| &b.btype == b"meta")
        .ok_or_else(|| malformed("no meta box"))?;
    // meta is a full box: 4 bytes of version and flags.
    let children = boxes(file, meta.body.start + 4..meta.body.end, false)?;
    let find = |t: &[u8; 4]| children.iter().find(|b| &b.btype == t);

    let hdlr = find(b"hdlr").ok_or_else(|| malformed("no hdlr box"))?;
    if file.get(hdlr.body.start + 8..hdlr.body.start + 12) != Some(b"pict") {
        return Err(malformed("meta handler is not pict"));
    }
    let items = match find(b"iinf") {
        Some(b) => item_infos(file, b)?,
        None => Vec::new(),
    };
    if items.is_empty() {
        return Ok(());
    }
    let iloc = find(b"iloc").ok_or_else(|| malformed("no iloc box"))?;
    let locations = item_locations(file, iloc)?;
    let idat = find(b"idat").map(|b| b.body.clone());

    for item in &items {
        let loc = locations
            .iter()
            .find(|(id, _)| *id == item.id)
            .map(|(_, l)| l)
            .ok_or_else(|| malformed("metadata item has no location"))?;
        let pieces = item_pieces(file, loc, idat.as_ref())?;
        let data = pieces.gather(file);
        let stripped = match item.kind {
            ItemKind::Exif => strip_exif_item(&data).map_err(|e| e.at(Format::Heic))?,
            ItemKind::Xmp => xmp::strip(&data).map_err(|e| e.at(Format::Heic))?,
        };
        if let Some(new) = stripped {
            pieces.scatter(file, &new, plan);
            let kind = match item.kind {
                ItemKind::Exif => LocationKind::Exif,
                ItemKind::Xmp => LocationKind::Xmp,
            };
            for r in &pieces.0 {
                plan.found(kind, r.clone());
            }
        }
    }
    Ok(())
}

/// The Exif item starts with a 4-byte offset from the end of that field to the
/// TIFF header (usually 6, skipping `Exif\0\0`).
fn strip_exif_item(data: &[u8]) -> Result<Option<Vec<u8>>, MetaError> {
    let skip = be_u32(data, 0).ok_or(MetaError::Malformed("Exif item too short"))? as usize;
    let start = 4usize
        .checked_add(skip)
        .filter(|&s| s < data.len())
        .ok_or(MetaError::Malformed("Exif item TIFF offset"))?;
    Ok(tiff::strip(&data[start..])?.map(|new| {
        let mut out = data[..start].to_vec();
        out.extend_from_slice(&new);
        out
    }))
}

/// Reads `iinf` and returns the metadata items (Exif, and `mime` items of type
/// `application/rdf+xml`).
fn item_infos(file: &[u8], iinf: &BoxRef) -> Result<Vec<Item>, StripError> {
    let b = &iinf.body;
    let version = *file
        .get(b.start)
        .ok_or_else(|| malformed("iinf truncated"))?;
    let first = if version == 0 {
        b.start + 6
    } else {
        b.start + 8
    };
    if first > b.end {
        return Err(malformed("iinf truncated"));
    }
    let mut out = Vec::new();
    for infe in boxes(file, first..b.end, false)? {
        if &infe.btype != b"infe" {
            continue;
        }
        if let Some(item) = item_info(file, &infe)? {
            out.push(item);
        }
    }
    Ok(out)
}

fn cstr(file: &[u8], at: usize, end: usize) -> Option<(&[u8], usize)> {
    let s = file.get(at..end)?;
    let n = s.iter().position(|&c| c == 0)?;
    Some((&s[..n], at + n + 1))
}

fn item_info(file: &[u8], infe: &BoxRef) -> Result<Option<Item>, StripError> {
    let b = &infe.body;
    let bad = || malformed("infe truncated");
    let version = *file.get(b.start).ok_or_else(bad)?;
    let mut p = b.start + 4;
    let (id, protection, kind) = if version >= 2 {
        let id = if version == 2 {
            let v = be_u16(file, p).ok_or_else(bad)?;
            p += 2;
            u32::from(v)
        } else {
            let v = be_u32(file, p).ok_or_else(bad)?;
            p += 4;
            v
        };
        let protection = be_u16(file, p).ok_or_else(bad)?;
        let item_type = file.get(p + 2..p + 6).ok_or_else(bad)?;
        p += 6;
        let kind = match item_type {
            b"Exif" => Some(ItemKind::Exif),
            b"mime" => {
                let (_name, q) = cstr(file, p, b.end).ok_or_else(bad)?;
                let (content_type, q) = cstr(file, q, b.end).ok_or_else(bad)?;
                let encoding = cstr(file, q, b.end).map(|(e, _)| e).unwrap_or(b"");
                if content_type == b"application/rdf+xml" {
                    if !encoding.is_empty() {
                        return Err(unstrippable("encoded XMP item"));
                    }
                    Some(ItemKind::Xmp)
                } else {
                    None
                }
            }
            _ => None,
        };
        (id, protection, kind)
    } else {
        // Versions 0 and 1: item_ID, protection index, name, content type.
        let id = u32::from(be_u16(file, p).ok_or_else(bad)?);
        let protection = be_u16(file, p + 2).ok_or_else(bad)?;
        let (_name, q) = cstr(file, p + 4, b.end).ok_or_else(bad)?;
        let (content_type, q) = cstr(file, q, b.end).ok_or_else(bad)?;
        let encoding = cstr(file, q, b.end).map(|(e, _)| e).unwrap_or(b"");
        let kind = if content_type == b"application/rdf+xml" {
            if !encoding.is_empty() {
                return Err(unstrippable("encoded XMP item"));
            }
            Some(ItemKind::Xmp)
        } else {
            None
        };
        (id, protection, kind)
    };
    match kind {
        Some(_) if protection != 0 => Err(unstrippable("protected metadata item")),
        Some(kind) => Ok(Some(Item { id, kind })),
        None => Ok(None),
    }
}

/// Reads `iloc` (versions 0, 1 and 2).
fn item_locations(file: &[u8], iloc: &BoxRef) -> Result<Vec<(u32, Location)>, StripError> {
    let b = &iloc.body;
    let bad = || malformed("iloc truncated");
    let version = *file.get(b.start).ok_or_else(bad)?;
    if version > 2 {
        return Err(malformed("iloc version"));
    }
    let sizes = *file.get(b.start + 4).ok_or_else(bad)?;
    let sizes2 = *file.get(b.start + 5).ok_or_else(bad)?;
    let (off_size, len_size) = ((sizes >> 4) as usize, (sizes & 0xF) as usize);
    let base_size = (sizes2 >> 4) as usize;
    let index_size = if version >= 1 {
        (sizes2 & 0xF) as usize
    } else {
        0
    };
    for s in [off_size, len_size, base_size, index_size] {
        if !matches!(s, 0 | 4 | 8) {
            return Err(malformed("iloc field size"));
        }
    }
    let mut p = b.start + 6;
    let count = if version < 2 {
        let c = u32::from(be_u16(file, p).ok_or_else(bad)?);
        p += 2;
        c
    } else {
        let c = be_u32(file, p).ok_or_else(bad)?;
        p += 4;
        c
    };
    let read = |p: &mut usize, size: usize| -> Result<u64, StripError> {
        if *p + size > b.end {
            return Err(bad());
        }
        let v = be_uint(file, *p, size).ok_or_else(bad)?;
        *p += size;
        Ok(v)
    };
    let mut out = Vec::new();
    for _ in 0..count {
        let id = if version < 2 {
            read(&mut p, 2)? as u32
        } else {
            read(&mut p, 4)? as u32
        };
        let method = if version >= 1 {
            (read(&mut p, 2)? & 0xF) as u8
        } else {
            0
        };
        let data_ref = read(&mut p, 2)? as u16;
        let base = read(&mut p, base_size)?;
        let extent_count = read(&mut p, 2)?;
        let mut extents = Vec::new();
        for _ in 0..extent_count {
            if index_size > 0 {
                read(&mut p, index_size)?;
            }
            let off = read(&mut p, off_size)?;
            let len = read(&mut p, len_size)?;
            extents.push((off, len));
        }
        out.push((
            id,
            Location {
                method,
                data_ref,
                base,
                extents,
            },
        ));
    }
    Ok(out)
}

/// File ranges holding an item's data.
fn item_pieces(
    file: &[u8],
    loc: &Location,
    idat: Option<&Range<usize>>,
) -> Result<Pieces, StripError> {
    if loc.data_ref != 0 {
        return Err(unstrippable("metadata item in another file"));
    }
    let area: Range<usize> = match loc.method {
        0 => 0..file.len(),
        1 => idat
            .cloned()
            .ok_or_else(|| malformed("item in idat without an idat box"))?,
        _ => return Err(unstrippable("metadata item built from other items")),
    };
    if loc.extents.is_empty() {
        return Err(malformed("metadata item has no extents"));
    }
    let mut pieces = Pieces::default();
    for &(off, len) in &loc.extents {
        let start = (area.start as u64)
            .checked_add(loc.base)
            .and_then(|s| s.checked_add(off))
            .filter(|&s| s <= area.end as u64)
            .ok_or_else(|| malformed("item extent outside the file"))?;
        // A zero length means "to the end" and is only valid for one extent.
        let end = if len == 0 && loc.extents.len() == 1 {
            area.end as u64
        } else {
            start
                .checked_add(len)
                .filter(|&e| e <= area.end as u64)
                .ok_or_else(|| malformed("item extent outside the file"))?
        };
        pieces.0.push(start as usize..end as usize);
    }
    Ok(pieces)
}

#[cfg(test)]
mod tests {
    use super::super::strip_location;
    use super::*;

    fn bx(t: &[u8], body: &[u8]) -> Vec<u8> {
        let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(t);
        v.extend_from_slice(body);
        v
    }

    fn full(t: &[u8], version: u8, body: &[u8]) -> Vec<u8> {
        let mut b = vec![version, 0, 0, 0];
        b.extend_from_slice(body);
        bx(t, &b)
    }

    fn exif_payload() -> Vec<u8> {
        let mut d = vec![0, 0, 0, 6];
        d.extend_from_slice(b"Exif\0\0");
        d.extend(tiff::tests::build(
            80,
            8,
            &[
                (8, vec![(0x0112, 3, 1, 6), (0x8825, 4, 1, 40)], 0),
                (40, vec![(2, 5, 1, 60)], 0),
            ],
            &[(60, &[0, 0, 0, 12, 0, 0, 0, 1])],
        ));
        d
    }

    /// A minimal HEIF: item 1 (hvc1 pixels), item 2 (Exif, split into two
    /// extents in mdat), item 3 (XMP in idat).
    fn heif(xmp: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let exif = exif_payload();
        let ftyp = bx(b"ftyp", b"heic\0\0\0\0mif1heic");
        let mut infe = Vec::new();
        infe.extend(full(
            b"infe",
            2,
            &[&[0, 1, 0, 0][..], b"hvc1", b"\0"].concat(),
        ));
        infe.extend(full(
            b"infe",
            2,
            &[&[0, 2, 0, 0][..], b"Exif", b"\0"].concat(),
        ));
        infe.extend(full(
            b"infe",
            2,
            &[&[0, 3, 0, 0][..], b"mime", b"\0application/rdf+xml\0"].concat(),
        ));
        let iinf = full(b"iinf", 0, &[&[0, 3][..], &infe].concat());
        let hdlr = full(b"hdlr", 0, &[&[0u8; 4][..], b"pict", &[0u8; 13]].concat());
        let idat = bx(b"idat", xmp);
        let pixels = b"PIXELDATA".to_vec();
        // iloc v1, offset 4, length 4, base 0, index 0; three items.
        let build_iloc = |mdat_start: u32| {
            let split = 20u32;
            let mut b = vec![0x44, 0x00, 0, 3];
            // item 1: method 0, one extent
            b.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 1]);
            b.extend_from_slice(&mdat_start.to_be_bytes());
            b.extend_from_slice(&(pixels.len() as u32).to_be_bytes());
            // item 2: two extents
            b.extend_from_slice(&[0, 2, 0, 0, 0, 0, 0, 2]);
            b.extend_from_slice(&(mdat_start + pixels.len() as u32).to_be_bytes());
            b.extend_from_slice(&split.to_be_bytes());
            b.extend_from_slice(&(mdat_start + pixels.len() as u32 + split).to_be_bytes());
            b.extend_from_slice(&(exif.len() as u32 - split).to_be_bytes());
            // item 3: method 1 (idat)
            b.extend_from_slice(&[0, 3, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0]);
            b.extend_from_slice(&(xmp.len() as u32).to_be_bytes());
            full(b"iloc", 1, &b)
        };
        let meta_len =
            |iloc: &[u8]| full(b"meta", 0, &[&hdlr[..], &iinf, &idat, iloc].concat()).len();
        let probe = build_iloc(0);
        let mdat_start = (ftyp.len() + meta_len(&probe) + 8) as u32;
        let iloc = build_iloc(mdat_start);
        let meta = full(b"meta", 0, &[&hdlr[..], &iinf, &idat, &iloc].concat());
        let mdat = bx(b"mdat", &[&pixels[..], &exif].concat());
        let file = [ftyp, meta, mdat].concat();
        (file, exif)
    }

    #[test]
    fn multi_extent_exif_and_idat_xmp_are_stripped_in_place() {
        let xmp = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><d xmlns:exif="http://ns.adobe.com/exif/1.0/" exif:GPSLatitude="12,20.736N"/></x:xmpmeta>"#;
        let (file, _) = heif(xmp);
        let out = strip_location(&file).unwrap();
        assert!(out.changed && out.kinds.exif && out.kinds.xmp);
        assert_eq!(out.bytes.len(), file.len());
        let s = String::from_utf8_lossy(&out.bytes);
        assert!(!s.contains("12,20.736N"));
        assert!(s.contains("PIXELDATA"));
        // Exif item regions: two extents; XMP region inside idat.
        assert_eq!(
            out.regions
                .iter()
                .filter(|r| r.kind == LocationKind::Exif)
                .count(),
            2
        );
        let again = strip_location(&out.bytes).unwrap();
        assert!(!again.changed);
        assert_eq!(again.bytes, out.bytes);
        // Every changed byte is inside a reported region.
        for (i, (a, b)) in file.iter().zip(&out.bytes).enumerate() {
            if a != b {
                assert!(out.regions.iter().any(|r| r.range.contains(&i)), "byte {i}");
            }
        }
    }

    #[test]
    fn structural_errors() {
        let (file, _) = heif(b"<a/>");
        // Item extent outside the file: patch the length of the Exif item's
        // second extent (iloc body: 4 version/flags, 2 sizes, 2 count, item 1
        // is 16 bytes, item 2 header 8 bytes, first extent 8 bytes, offset 4).
        let iloc = file.windows(4).position(|w| w == b"iloc").unwrap() + 4;
        let len_at = iloc + 8 + 16 + 8 + 8 + 4;
        let mut bad = file.clone();
        bad[len_at..len_at + 4].copy_from_slice(&0x00FF_FFFFu32.to_be_bytes());
        assert_eq!(
            strip_location(&bad),
            Err(StripError::Malformed {
                format: Format::Heic,
                detail: "item extent outside the file"
            })
        );
        // Item without a location.
        let mut bad = file.clone();
        bad[iloc + 8 + 16 + 1] = 9;
        assert_eq!(
            strip_location(&bad),
            Err(StripError::Malformed {
                format: Format::Heic,
                detail: "metadata item has no location"
            })
        );
        // Top-level box past the end.
        let mut cut = file.clone();
        cut.truncate(file.len() - 1);
        assert_eq!(
            strip_location(&cut),
            Err(StripError::Truncated(Format::Heic))
        );
        // No meta box.
        let only_ftyp = bx(b"ftyp", b"heic\0\0\0\0mif1heic");
        assert!(matches!(
            strip_location(&only_ftyp),
            Err(StripError::Malformed { .. })
        ));
        // Box size smaller than its header.
        let mut tiny = only_ftyp.clone();
        tiny.extend_from_slice(&[0, 0, 0, 4, b'f', b'r', b'e', b'e']);
        assert!(matches!(
            strip_location(&tiny),
            Err(StripError::Malformed { .. })
        ));
    }

    #[test]
    fn exif_item_offset_outside_is_malformed() {
        assert!(strip_exif_item(&[0, 0, 0, 9, 1, 2]).is_err());
        assert!(strip_exif_item(&[0, 0]).is_err());
        assert!(strip_exif_item(&[0xFF, 0xFF, 0xFF, 0xFF, 1]).is_err());
    }
}
