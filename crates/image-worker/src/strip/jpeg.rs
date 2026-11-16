//! JPEG location strip: EXIF and XMP (main and extended) in APP1, IPTC in
//! APP13, in the primary image and in every embedded secondary image.

use std::collections::HashSet;
use std::ops::Range;

use super::{be_u16, iptc, tiff, xmp, Format, LocationKind, Pieces, Plan, StripError};

const XMP_HEADER: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const XMP_EXT_HEADER: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";
const PHOTOSHOP_HEADER: &[u8] = b"Photoshop 3.0\0";
const MPF_HEADER: &[u8] = b"MPF\0";

/// Most images (primary plus secondary) read from one file.
const MAX_IMAGES: usize = 64;

const SOI: [u8; 3] = [0xFF, 0xD8, 0xFF];

fn malformed(detail: &'static str) -> StripError {
    StripError::Malformed {
        format: Format::Jpeg,
        detail,
    }
}

#[derive(Debug, Clone)]
struct Segment {
    marker: u8,
    /// From the `FF` of the marker to the end of the payload.
    range: Range<usize>,
    payload: Range<usize>,
}

#[derive(Debug)]
struct Image {
    segments: Vec<Segment>,
    /// First byte after the EOI marker.
    end: usize,
}

pub(super) fn plan(file: &[u8], plan: &mut Plan) -> Result<(), StripError> {
    let primary = walk(file, 0)?;
    let mut seen = HashSet::from([0usize]);
    process(file, &primary, plan)?;

    // Secondary images listed in the primary's MPF table.
    let mut starts: Vec<usize> = mpf_offsets(file, &primary)
        .into_iter()
        .filter(|&o| o >= primary.end && file.get(o..o + 3) == Some(&SOI))
        .collect();
    // Images concatenated after the primary's EOI.
    let mut pos = primary.end;
    while file.get(pos..pos + 3) == Some(&SOI) {
        if seen.len() >= MAX_IMAGES {
            return Err(malformed("too many embedded images"));
        }
        let img = walk(file, pos)?;
        seen.insert(pos);
        process(file, &img, plan)?;
        pos = img.end;
    }
    starts.sort_unstable();
    starts.dedup();
    for start in starts {
        if !seen.insert(start) {
            continue;
        }
        if seen.len() > MAX_IMAGES {
            return Err(malformed("too many embedded images"));
        }
        let img = walk(file, start)?;
        process(file, &img, plan)?;
    }
    Ok(())
}

/// Reads the marker segments of the image starting at `start` (an SOI) up to
/// and including its EOI, skipping entropy-coded data.
fn walk(file: &[u8], start: usize) -> Result<Image, StripError> {
    let truncated = StripError::Truncated(Format::Jpeg);
    let mut segments = Vec::new();
    let mut pos = start + 2;
    loop {
        let b = *file.get(pos).ok_or(truncated.clone())?;
        if b != 0xFF {
            return Err(malformed("marker expected"));
        }
        // Fill bytes.
        while file.get(pos + 1) == Some(&0xFF) {
            pos += 1;
        }
        let marker = *file.get(pos + 1).ok_or(truncated.clone())?;
        let seg_start = pos;
        pos += 2;
        match marker {
            0xD9 => return Ok(Image { segments, end: pos }),
            0xD8 => return Err(malformed("SOI inside an image")),
            0x00 => return Err(malformed("stuffed byte outside a scan")),
            0x01 | 0xD0..=0xD7 => continue,
            _ => {}
        }
        let len = be_u16(file, pos).ok_or(truncated.clone())? as usize;
        if len < 2 {
            return Err(malformed("segment length below 2"));
        }
        let end = pos + len;
        if end > file.len() {
            return Err(truncated);
        }
        segments.push(Segment {
            marker,
            range: seg_start..end,
            payload: pos + 2..end,
        });
        pos = end;
        if marker == 0xDA {
            pos = skip_entropy(file, pos)?;
        }
    }
}

/// Returns the position of the next marker after entropy-coded data.
fn skip_entropy(file: &[u8], mut pos: usize) -> Result<usize, StripError> {
    loop {
        let i = file
            .get(pos..)
            .and_then(|s| s.iter().position(|&b| b == 0xFF))
            .map(|i| i + pos)
            .ok_or(StripError::Truncated(Format::Jpeg))?;
        match file.get(i + 1) {
            None => return Err(StripError::Truncated(Format::Jpeg)),
            // Stuffed zero, restart marker, or fill byte: still in the scan.
            Some(0x00 | 0xD0..=0xD7) => pos = i + 2,
            Some(0xFF) => pos = i + 1,
            Some(_) => return Ok(i),
        }
    }
}

struct ExtPart {
    guid: [u8; 32],
    full_len: u32,
    offset: u32,
    data: Range<usize>,
    segment: Range<usize>,
}

/// Plans the strip of one image's metadata segments.
fn process(file: &[u8], img: &Image, plan: &mut Plan) -> Result<(), StripError> {
    let fail = |e: super::MetaError| e.at(Format::Jpeg);
    let mut photoshop = Pieces::default();
    let mut photoshop_segments = Vec::new();
    let mut ext_parts = Vec::new();

    for seg in &img.segments {
        let payload = &file[seg.payload.clone()];
        match seg.marker {
            0xE1 if payload.starts_with(b"Exif\0") && payload.len() >= 6 => {
                let at = seg.payload.start + 6;
                if let Some(new) = tiff::strip(&file[at..seg.payload.end]).map_err(fail)? {
                    plan.edit(file, at, new);
                    plan.found(LocationKind::Exif, seg.range.clone());
                }
            }
            0xE1 if payload.starts_with(XMP_HEADER) => {
                let at = seg.payload.start + XMP_HEADER.len();
                if let Some(new) = xmp::strip(&file[at..seg.payload.end]).map_err(fail)? {
                    plan.edit(file, at, new);
                    plan.found(LocationKind::Xmp, seg.range.clone());
                }
            }
            0xE1 if payload.starts_with(XMP_EXT_HEADER) => {
                let h = XMP_EXT_HEADER.len();
                if payload.len() < h + 40 {
                    return Err(malformed("extended XMP header"));
                }
                let mut guid = [0u8; 32];
                guid.copy_from_slice(&payload[h..h + 32]);
                let full_len = super::be_u32(payload, h + 32).unwrap_or(0);
                let offset = super::be_u32(payload, h + 36).unwrap_or(0);
                ext_parts.push(ExtPart {
                    guid,
                    full_len,
                    offset,
                    data: seg.payload.start + h + 40..seg.payload.end,
                    segment: seg.range.clone(),
                });
            }
            0xED if payload.starts_with(PHOTOSHOP_HEADER) => {
                photoshop
                    .0
                    .push(seg.payload.start + PHOTOSHOP_HEADER.len()..seg.payload.end);
                photoshop_segments.push(seg.range.clone());
            }
            _ => {}
        }
    }

    // Photoshop resources may span several APP13 segments: read them as one.
    if !photoshop.0.is_empty() {
        let block = photoshop.gather(file);
        if let Some(new) = iptc::strip_irb(&block).map_err(fail)? {
            photoshop.scatter(file, &new, plan);
            for r in photoshop_segments {
                plan.found(LocationKind::Iptc, r);
            }
        }
    }

    // Extended XMP: parts grouped by GUID, ordered by offset, must tile the
    // whole serialisation.
    let mut guids: Vec<[u8; 32]> = ext_parts.iter().map(|p| p.guid).collect();
    guids.sort_unstable();
    guids.dedup();
    for guid in guids {
        let mut parts: Vec<&ExtPart> = ext_parts.iter().filter(|p| p.guid == guid).collect();
        parts.sort_by_key(|p| p.offset);
        let full_len = parts[0].full_len as usize;
        let mut expect = 0usize;
        for p in &parts {
            if p.full_len as usize != full_len || p.offset as usize != expect {
                return Err(malformed("extended XMP parts do not fit together"));
            }
            expect += p.data.len();
        }
        if expect != full_len {
            return Err(malformed("extended XMP parts do not fit together"));
        }
        let pieces = Pieces(parts.iter().map(|p| p.data.clone()).collect());
        let block = pieces.gather(file);
        if let Some(new) = xmp::strip(&block).map_err(fail)? {
            pieces.scatter(file, &new, plan);
            for p in &parts {
                plan.found(LocationKind::Xmp, p.segment.clone());
            }
        }
    }
    Ok(())
}

/// Absolute offsets of the secondary images listed in the image's MPF (APP2)
/// table. The table is not location data, so a broken one is ignored (images
/// concatenated after the primary are still found by walking).
fn mpf_offsets(file: &[u8], img: &Image) -> Vec<usize> {
    let Some(seg) = img
        .segments
        .iter()
        .find(|s| s.marker == 0xE2 && file[s.payload.clone()].starts_with(MPF_HEADER))
    else {
        return Vec::new();
    };
    let base = seg.payload.start + MPF_HEADER.len();
    let t = &file[base..seg.payload.end];
    let le = match t.get(0..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return Vec::new(),
    };
    let u16_at = |o: usize| {
        t.get(o..o + 2).map(|s| {
            if le {
                u16::from_le_bytes([s[0], s[1]])
            } else {
                u16::from_be_bytes([s[0], s[1]])
            }
        })
    };
    let u32_at = |o: usize| {
        t.get(o..o + 4).map(|s| {
            let a = [s[0], s[1], s[2], s[3]];
            if le {
                u32::from_le_bytes(a)
            } else {
                u32::from_be_bytes(a)
            }
        })
    };
    let Some(ifd) = u32_at(4).map(|v| v as usize) else {
        return Vec::new();
    };
    let Some(count) = u16_at(ifd) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..count as usize {
        let e = ifd + 2 + 12 * i;
        if u16_at(e) != Some(0xB002) {
            continue;
        }
        let n = u32_at(e + 4).unwrap_or(0) as usize / 16;
        let Some(at) = u32_at(e + 8).map(|v| v as usize) else {
            break;
        };
        for k in 0..n.min(MAX_IMAGES) {
            if let Some(off) = u32_at(at + 16 * k + 8) {
                if off != 0 {
                    out.push(base + off as usize);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::{rawprofile, strip_location};
    use super::*;

    fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![0xFF, marker];
        v.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        v.extend_from_slice(payload);
        v
    }

    /// A tiny JPEG: SOI, the given segments, SOF0, SOS with scan data, EOI.
    fn jpeg(segments: &[Vec<u8>]) -> Vec<u8> {
        let mut f = vec![0xFF, 0xD8];
        for s in segments {
            f.extend_from_slice(s);
        }
        f.extend(seg(0xC0, &[8, 0, 1, 0, 1, 1, 1, 0x11, 0]));
        f.extend(seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]));
        f.extend_from_slice(&[0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD0, 0x56, 0xFF, 0xFF, 0xD9]);
        f
    }

    fn exif_seg() -> Vec<u8> {
        seg(0xE1, &rawprofile::tests::exif_with_gps())
    }

    fn changed_inside_regions(before: &[u8], after: &[u8], regions: &[super::super::Region]) {
        assert_eq!(before.len(), after.len());
        for (i, (a, b)) in before.iter().zip(after).enumerate() {
            if a != b {
                assert!(
                    regions.iter().any(|r| r.range.contains(&i)),
                    "byte {i} changed outside"
                );
            }
        }
    }

    #[test]
    fn exif_in_primary_and_secondary_images() {
        let primary = jpeg(&[exif_seg()]);
        let secondary = jpeg(&[exif_seg()]);
        let f = [primary.clone(), secondary].concat();
        let out = strip_location(&f).unwrap();
        assert!(out.changed && out.kinds.exif);
        assert_eq!(out.regions.len(), 2, "{:?}", out.regions);
        assert!(out.regions[1].range.start >= primary.len());
        changed_inside_regions(&f, &out.bytes, &out.regions);
        assert!(!strip_location(&out.bytes).unwrap().changed);
    }

    #[test]
    fn secondary_image_found_through_mpf_after_padding() {
        // Primary with an MPF table pointing at a gain-map image that sits
        // after some padding (so only the MPF table finds it).
        let mut mpf = MPF_HEADER.to_vec();
        let tiff_at = 0usize;
        let mut t = b"MM\0\x2A\0\0\0\x08".to_vec();
        t.extend_from_slice(&[0, 1]);
        t.extend_from_slice(&[0xB0, 0x02, 0, 7, 0, 0, 0, 32, 0, 0, 0, 26]);
        t.extend_from_slice(&[0, 0, 0, 0]);
        // Two MP entries: primary (offset 0) and the gain map (patched below).
        t.extend_from_slice(&[0u8; 32]);
        mpf.extend_from_slice(&t);
        let mpf_seg_probe = seg(0xE2, &mpf);
        let primary_probe = jpeg(&[mpf_seg_probe]);
        let padding = 7usize;
        let gain_map = jpeg(&[exif_seg()]);
        // MPF offsets are relative to the MPF TIFF header.
        let mpf_base = 2 + 4 + MPF_HEADER.len() + tiff_at;
        let gain_at = primary_probe.len() + padding;
        let rel = (gain_at - mpf_base) as u32;
        let entry2 = MPF_HEADER.len() + 26 + 16 + 8;
        mpf[entry2..entry2 + 4].copy_from_slice(&rel.to_be_bytes());
        let primary = jpeg(&[seg(0xE2, &mpf)]);
        let f = [primary, vec![0u8; padding], gain_map].concat();
        let out = strip_location(&f).unwrap();
        assert!(out.kinds.exif, "gain map EXIF found through MPF");
        assert!(out.regions[0].range.start >= gain_at);
        changed_inside_regions(&f, &out.bytes, &out.regions);
    }

    #[test]
    fn extended_xmp_spanning_two_segments() {
        let body = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="r"><rdf:Description xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/"><photoshop:City>Testville</photoshop:City><photoshop:Headline>keep</photoshop:Headline></rdf:Description></rdf:RDF></x:xmpmeta>"#;
        let guid = [b'A'; 32];
        // Split inside "Testville".
        let split = body.windows(9).position(|w| w == b"Testville").unwrap() + 4;
        let part = |off: usize, data: &[u8]| {
            let mut p = XMP_EXT_HEADER.to_vec();
            p.extend_from_slice(&guid);
            p.extend_from_slice(&(body.len() as u32).to_be_bytes());
            p.extend_from_slice(&(off as u32).to_be_bytes());
            p.extend_from_slice(data);
            seg(0xE1, &p)
        };
        // Parts out of order on purpose.
        let f = jpeg(&[part(split, &body[split..]), part(0, &body[..split])]);
        let out = strip_location(&f).unwrap();
        assert!(out.kinds.xmp);
        let s = String::from_utf8_lossy(&out.bytes);
        assert!(!s.contains("Test") && !s.contains("ville"));
        assert!(s.contains("<photoshop:Headline>keep</photoshop:Headline>"));
        changed_inside_regions(&f, &out.bytes, &out.regions);
        assert!(!strip_location(&out.bytes).unwrap().changed);

        let broken = jpeg(&[part(split + 1, &body[split..]), part(0, &body[..split])]);
        assert!(matches!(
            strip_location(&broken),
            Err(StripError::Malformed { .. })
        ));
    }

    #[test]
    fn photoshop_resources_spanning_two_app13_segments() {
        let mut iim = vec![0x1C, 2, 90, 0, 9];
        iim.extend_from_slice(b"Testville");
        let mut irb = b"8BIM\x04\x04\0\0".to_vec();
        irb.extend_from_slice(&(iim.len() as u32).to_be_bytes());
        irb.extend_from_slice(&iim);
        irb.push(0);
        let (a, b) = irb.split_at(15);
        let s1 = seg(0xED, &[PHOTOSHOP_HEADER, a].concat());
        let s2 = seg(0xED, &[PHOTOSHOP_HEADER, b].concat());
        let f = jpeg(&[s1, s2]);
        let out = strip_location(&f).unwrap();
        assert!(out.kinds.iptc);
        assert!(!String::from_utf8_lossy(&out.bytes).contains("ville"));
        changed_inside_regions(&f, &out.bytes, &out.regions);
    }

    #[test]
    fn structural_errors() {
        let ok = jpeg(&[]);
        assert!(!strip_location(&ok).unwrap().changed);
        // Zero and one segment lengths.
        for len in [0u8, 1] {
            let mut f = ok.clone();
            f[4..6].copy_from_slice(&[0, len]);
            assert_eq!(strip_location(&f), Err(malformed("segment length below 2")));
        }
        // Oversized segment length.
        let mut f = ok.clone();
        f[4..6].copy_from_slice(&[0xFF, 0xFF]);
        assert_eq!(strip_location(&f), Err(StripError::Truncated(Format::Jpeg)));
        // No EOI.
        let mut f = ok.clone();
        f.truncate(f.len() - 2);
        assert_eq!(strip_location(&f), Err(StripError::Truncated(Format::Jpeg)));
        // Garbage where a marker should be.
        let mut f = ok.clone();
        f[15] = 0x00;
        assert_eq!(strip_location(&f), Err(malformed("marker expected")));
        // Trailing bytes that are not an image are left alone.
        let mut f = ok.clone();
        f.extend_from_slice(b"trailer Exif\0\0MM");
        let out = strip_location(&f).unwrap();
        assert_eq!(out.bytes, f);
        // A concatenated image that is truncated fails closed.
        let mut f = ok.clone();
        f.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00]);
        assert_eq!(strip_location(&f), Err(StripError::Truncated(Format::Jpeg)));
        // A broken EXIF block fails closed.
        let f = jpeg(&[seg(0xE1, b"Exif\0\0MM\0\x2A\0\0\0\x08\0\x40")]);
        assert!(matches!(
            strip_location(&f),
            Err(StripError::Malformed { .. })
        ));
        // Short extended XMP header.
        let f = jpeg(&[seg(0xE1, &[XMP_EXT_HEADER, b"short"].concat())]);
        assert_eq!(strip_location(&f), Err(malformed("extended XMP header")));
    }
}
