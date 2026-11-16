//! EXIF (TIFF structure) location strip: removes every GPS IFD pointer and
//! zeroes the GPS IFD it pointed at, without moving any other byte.

use std::collections::HashSet;
use std::ops::Range;

use super::MetaError;

const TAG_GPS_IFD: u16 = 0x8825;
const TAG_EXIF_IFD: u16 = 0x8769;
const TAG_INTEROP_IFD: u16 = 0xA005;
const TAG_SUB_IFDS: u16 = 0x014A;
const TAG_THUMB_OFFSET: u16 = 0x0201;
const TAG_THUMB_LENGTH: u16 = 0x0202;

const TYPE_LONG: u16 = 4;
const TYPE_IFD: u16 = 13;

/// Most IFDs followed in one TIFF block; more is treated as malformed.
const MAX_IFDS: usize = 256;

/// Strips the GPS IFD from a TIFF block (offsets relative to its first byte).
/// Returns the modified block when location data was found.
pub(super) fn strip(buf: &[u8]) -> Result<Option<Vec<u8>>, MetaError> {
    let t = Tiff::new(buf)?;
    let walk = t.walk()?;
    if walk.removals.is_empty() {
        return Ok(None);
    }

    let mut out = buf.to_vec();
    for removal in &walk.removals {
        t.remove_entries(&mut out, removal);
    }

    let mut kept = walk.kept;
    kept.sort_by_key(|r| r.start);
    let mut done = HashSet::new();
    for &gps in &walk.gps_ifds {
        // Only a well-formed GPS IFD is zeroed; a dangling or overrunning
        // pointer is simply removed.
        if gps < 8 || t.entries(gps).is_none() || !done.insert(gps) {
            continue;
        }
        for r in t.ifd_ranges(gps) {
            zero_outside(&mut out, r, &kept);
        }
    }
    Ok(Some(out))
}

/// Reads orientation (IFD0 tag 0x0112) and DateTimeOriginal (Exif IFD tag
/// 0x9003) from a TIFF block. Used by tests to prove they survive a strip.
#[cfg(test)]
pub(super) fn orientation_and_capture_time(buf: &[u8]) -> (Option<u16>, Option<String>) {
    let Ok(t) = Tiff::new(buf) else {
        return (None, None);
    };
    let Some(ifd0) = t.u32(4).map(|v| v as usize) else {
        return (None, None);
    };
    let Some(entries) = t.entries(ifd0) else {
        return (None, None);
    };
    let mut orientation = None;
    let mut time = None;
    for e in &entries {
        if e.tag == 0x0112 && e.typ == 3 {
            orientation = t.u16(e.field);
        }
        if e.tag == TAG_EXIF_IFD {
            let exif = t.u32(e.field).map(|v| v as usize).unwrap_or(usize::MAX);
            for x in t.entries(exif).unwrap_or_default() {
                if x.tag == 0x9003 && x.typ == 2 {
                    if let Some(r) = t.value_range(&x) {
                        let s = String::from_utf8_lossy(&buf[r]);
                        time = Some(s.trim_end_matches('\0').to_string());
                    }
                }
            }
        }
    }
    (orientation, time)
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    tag: u16,
    typ: u16,
    count: u32,
    /// Offset of the 4-byte value-or-offset field.
    field: usize,
}

#[derive(Debug)]
struct Removal {
    ifd: usize,
    count: usize,
    remove: Vec<usize>,
}

#[derive(Debug, Default)]
struct Walk {
    removals: Vec<Removal>,
    gps_ifds: Vec<usize>,
    /// Bytes that belong to IFDs and values that are kept.
    kept: Vec<Range<usize>>,
}

struct Tiff<'a> {
    b: &'a [u8],
    le: bool,
}

impl<'a> Tiff<'a> {
    fn new(b: &'a [u8]) -> Result<Self, MetaError> {
        let le = match b.get(0..2) {
            Some(b"II") => true,
            Some(b"MM") => false,
            _ => return Err(MetaError::Malformed("TIFF byte order mark")),
        };
        let t = Tiff { b, le };
        if t.u16(2) != Some(42) {
            return Err(MetaError::Malformed("TIFF magic number"));
        }
        Ok(t)
    }

    fn u16(&self, off: usize) -> Option<u16> {
        let s = self.b.get(off..off.checked_add(2)?)?;
        let a = [s[0], s[1]];
        Some(if self.le {
            u16::from_le_bytes(a)
        } else {
            u16::from_be_bytes(a)
        })
    }

    fn u32(&self, off: usize) -> Option<u32> {
        let s = self.b.get(off..off.checked_add(4)?)?;
        let a = [s[0], s[1], s[2], s[3]];
        Some(if self.le {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        })
    }

    fn put_u16(&self, out: &mut [u8], off: usize, v: u16) {
        let a = if self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        };
        out[off..off + 2].copy_from_slice(&a);
    }

    /// Reads the entries of the IFD at `off`, or `None` when the IFD is not
    /// entirely inside the block.
    fn entries(&self, off: usize) -> Option<Vec<Entry>> {
        let count = self.u16(off)? as usize;
        let end = off.checked_add(2 + 12 * count)?;
        if end > self.b.len() {
            return None;
        }
        Some(
            (0..count)
                .map(|i| {
                    let e = off + 2 + 12 * i;
                    Entry {
                        tag: self.u16(e).unwrap_or(0),
                        typ: self.u16(e + 2).unwrap_or(0),
                        count: self.u32(e + 4).unwrap_or(0),
                        field: e + 8,
                    }
                })
                .collect(),
        )
    }

    /// Byte range of an entry's value when it is stored outside the entry and
    /// lies inside the block.
    fn value_range(&self, e: &Entry) -> Option<Range<usize>> {
        let size = u64::from(type_size(e.typ)?) * u64::from(e.count);
        if size <= 4 {
            return None;
        }
        let start = self.u32(e.field)? as u64;
        let end = start.checked_add(size)?;
        if end > self.b.len() as u64 {
            return None;
        }
        Some(start as usize..end as usize)
    }

    /// Offsets an IFD-pointer entry points at (one, or several for SubIFDs).
    fn pointers(&self, e: &Entry) -> Vec<usize> {
        if e.typ != TYPE_LONG && e.typ != TYPE_IFD {
            return Vec::new();
        }
        if e.count == 1 {
            return self
                .u32(e.field)
                .map(|v| vec![v as usize])
                .unwrap_or_default();
        }
        match self.value_range(e) {
            Some(r) => r
                .step_by(4)
                .filter_map(|o| self.u32(o))
                .map(|v| v as usize)
                .collect(),
            None => Vec::new(),
        }
    }

    fn walk(&self) -> Result<Walk, MetaError> {
        let mut w = Walk::default();
        let ifd0 = self.u32(4).ok_or(MetaError::Malformed("TIFF header"))? as usize;
        if ifd0 < 8 || ifd0 >= self.b.len() {
            return Err(MetaError::Malformed("IFD0 offset outside the block"));
        }
        w.kept.push(0..8);

        let mut visited = HashSet::new();
        // (offset, follow the next-IFD chain)
        let mut queue = vec![(ifd0, true)];
        while let Some((off, chain)) = queue.pop() {
            if off < 8 || off >= self.b.len() || !visited.insert(off) {
                // Unreachable or already seen: a loop or a dangling pointer
                // ends here.
                continue;
            }
            if visited.len() > MAX_IFDS {
                return Err(MetaError::Malformed("too many IFDs"));
            }
            let entries = self
                .entries(off)
                .ok_or(MetaError::Malformed("IFD runs past the end of the block"))?;
            let count = entries.len();
            let ifd_end = (off + 2 + 12 * count + 4).min(self.b.len());
            w.kept.push(off..ifd_end);

            let mut remove = Vec::new();
            let mut thumb = (None, None);
            for (i, e) in entries.iter().enumerate() {
                match e.tag {
                    TAG_GPS_IFD => {
                        remove.push(i);
                        if e.count == 1 && (e.typ == TYPE_LONG || e.typ == TYPE_IFD) {
                            if let Some(p) = self.u32(e.field) {
                                w.gps_ifds.push(p as usize);
                            }
                        }
                        continue;
                    }
                    TAG_EXIF_IFD | TAG_INTEROP_IFD | TAG_SUB_IFDS => {
                        for p in self.pointers(e) {
                            queue.push((p, false));
                        }
                    }
                    TAG_THUMB_OFFSET => thumb.0 = self.u32(e.field),
                    TAG_THUMB_LENGTH => thumb.1 = self.u32(e.field),
                    _ => {}
                }
                if let Some(r) = self.value_range(e) {
                    w.kept.push(r);
                }
            }
            if let (Some(start), Some(len)) = thumb {
                let start = start as usize;
                w.kept.push(start..start.saturating_add(len as usize));
            }
            if !remove.is_empty() {
                w.removals.push(Removal {
                    ifd: off,
                    count,
                    remove,
                });
            }
            if chain {
                if let Some(next) = self.u32(off + 2 + 12 * count) {
                    if next != 0 {
                        queue.push((next as usize, true));
                    }
                }
            }
        }
        Ok(w)
    }

    /// Removes entries from an IFD in place: the kept entries move up, the
    /// count drops, the next-IFD pointer follows the last kept entry and the
    /// freed bytes at the end of the old IFD are zeroed.
    fn remove_entries(&self, out: &mut [u8], r: &Removal) {
        let base = r.ifd + 2;
        let old_end = (base + 12 * r.count + 4).min(self.b.len());
        let mut kept: Vec<u8> = Vec::with_capacity(12 * r.count + 4);
        for i in 0..r.count {
            if !r.remove.contains(&i) {
                kept.extend_from_slice(&self.b[base + 12 * i..base + 12 * i + 12]);
            }
        }
        let next = self
            .b
            .get(base + 12 * r.count..base + 12 * r.count + 4)
            .unwrap_or(&[0, 0, 0, 0]);
        kept.extend_from_slice(next);
        let new_count = r.count - r.remove.len();
        self.put_u16(out, r.ifd, new_count as u16);
        let region = &mut out[base..old_end];
        region.fill(0);
        let n = kept.len().min(region.len());
        region[..n].copy_from_slice(&kept[..n]);
    }

    /// Byte ranges of the IFD at `off` and of its out-of-line values, clipped to
    /// the block.
    fn ifd_ranges(&self, off: usize) -> Vec<Range<usize>> {
        let Some(count) = self.u16(off) else {
            return Vec::new();
        };
        let count = count as usize;
        let end = (off + 2 + 12 * count + 4).min(self.b.len());
        let mut out = vec![off..end];
        for i in 0..count {
            let e = off + 2 + 12 * i;
            if e + 12 > self.b.len() {
                break;
            }
            let entry = Entry {
                tag: 0,
                typ: self.u16(e + 2).unwrap_or(0),
                count: self.u32(e + 4).unwrap_or(0),
                field: e + 8,
            };
            if let Some(r) = self.value_range(&entry) {
                out.push(r);
            }
        }
        out
    }
}

/// Zeroes `r` in `out`, except bytes inside `kept` (sorted by start).
fn zero_outside(out: &mut [u8], r: Range<usize>, kept: &[Range<usize>]) {
    let mut at = r.start;
    for k in kept {
        if k.end <= at {
            continue;
        }
        if k.start >= r.end {
            break;
        }
        if k.start > at {
            out[at..k.start].fill(0);
        }
        at = at.max(k.end);
        if at >= r.end {
            return;
        }
    }
    if at < r.end {
        out[at..r.end].fill(0);
    }
}

fn type_size(typ: u16) -> Option<u32> {
    Some(match typ {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 => 8,
        _ => return None,
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Builds a big-endian TIFF block from IFDs given as lists of
    /// (tag, type, count, value-or-offset). IFD `i` starts at `offsets[i]`;
    /// `next[i]` is its next-IFD pointer. `extra` bytes are written at their
    /// offsets.
    pub(crate) fn build(
        len: usize,
        ifd0: u32,
        ifds: &[(usize, Vec<(u16, u16, u32, u32)>, u32)],
        extra: &[(usize, &[u8])],
    ) -> Vec<u8> {
        let mut b = vec![0u8; len];
        b[0..4].copy_from_slice(b"MM\x00\x2A");
        b[4..8].copy_from_slice(&ifd0.to_be_bytes());
        for (off, entries, next) in ifds {
            let mut p = *off;
            b[p..p + 2].copy_from_slice(&(entries.len() as u16).to_be_bytes());
            p += 2;
            for (tag, typ, count, val) in entries {
                b[p..p + 2].copy_from_slice(&tag.to_be_bytes());
                b[p + 2..p + 4].copy_from_slice(&typ.to_be_bytes());
                b[p + 4..p + 8].copy_from_slice(&count.to_be_bytes());
                if *typ == 3 && *count == 1 {
                    b[p + 8..p + 10].copy_from_slice(&(*val as u16).to_be_bytes());
                } else {
                    b[p + 8..p + 12].copy_from_slice(&val.to_be_bytes());
                }
                p += 12;
            }
            b[p..p + 4].copy_from_slice(&next.to_be_bytes());
        }
        for (off, bytes) in extra {
            b[*off..off + bytes.len()].copy_from_slice(bytes);
        }
        b
    }

    fn tags(buf: &[u8], off: usize) -> Vec<u16> {
        let t = Tiff::new(buf).unwrap();
        t.entries(off).unwrap().iter().map(|e| e.tag).collect()
    }

    const LAT: [u8; 24] = [
        0, 0, 0, 12, 0, 0, 0, 1, 0, 0, 0, 20, 0, 0, 0, 1, 0, 0, 0x11, 0x40, 0, 0, 0, 100,
    ];

    #[test]
    fn gps_pointer_removed_from_ifd0_and_gps_ifd_zeroed() {
        // IFD0 at 8: orientation, Exif pointer, GPS pointer. Exif IFD at 60,
        // GPS IFD at 100 with a latitude rational at 140.
        let b = build(
            200,
            8,
            &[
                (
                    8,
                    vec![
                        (0x0112, 3, 1, 6),
                        (TAG_EXIF_IFD, 4, 1, 60),
                        (TAG_GPS_IFD, 4, 1, 100),
                    ],
                    0,
                ),
                (60, vec![(0x9003, 2, 20, 170)], 0),
                (100, vec![(1, 2, 2, 0x4E00_0000), (2, 5, 3, 140)], 0),
            ],
            &[(140, &LAT), (170, b"2026:01:02 03:04:05\0")],
        );
        let out = strip(&b).unwrap().expect("location found");
        assert_eq!(out.len(), b.len());
        assert_eq!(tags(&out, 8), vec![0x0112, TAG_EXIF_IFD]);
        assert!(
            out[100..164].iter().all(|&x| x == 0),
            "GPS IFD and value zeroed"
        );
        assert_eq!(
            orientation_and_capture_time(&out),
            (Some(6), Some("2026:01:02 03:04:05".to_string()))
        );
        // Next-IFD pointer of IFD0 now follows the two kept entries.
        assert_eq!(&out[8 + 2 + 24..8 + 2 + 24 + 4], &[0, 0, 0, 0]);
        // Idempotent.
        assert_eq!(strip(&out).unwrap(), None);
    }

    #[test]
    fn next_pointer_moves_with_the_entries() {
        // IFD0 -> IFD1 at 80; the GPS pointer is the first entry of IFD0.
        let b = build(
            120,
            8,
            &[
                (8, vec![(TAG_GPS_IFD, 4, 1, 0), (0x0112, 3, 1, 1)], 80),
                (80, vec![(0x0103, 3, 1, 6)], 0),
            ],
            &[],
        );
        let out = strip(&b).unwrap().unwrap();
        let t = Tiff::new(&out).unwrap();
        assert_eq!(t.u16(8), Some(1));
        assert_eq!(t.u32(8 + 2 + 12), Some(80), "IFD1 still linked");
        assert_eq!(tags(&out, 80), vec![0x0103]);
    }

    #[test]
    fn gps_pointer_in_ifd1_exif_and_sub_ifds_removed() {
        let b = build(
            300,
            8,
            &[
                (
                    8,
                    vec![(TAG_SUB_IFDS, 4, 1, 120), (TAG_EXIF_IFD, 4, 1, 60)],
                    200,
                ),
                (
                    60,
                    vec![(TAG_GPS_IFD, 4, 1, 250), (TAG_INTEROP_IFD, 4, 1, 90)],
                    0,
                ),
                (90, vec![(TAG_GPS_IFD, 4, 1, 250)], 0),
                (120, vec![(TAG_GPS_IFD, 4, 1, 250)], 0),
                (200, vec![(TAG_GPS_IFD, 4, 1, 250), (0x0103, 3, 1, 6)], 0),
                (250, vec![(2, 5, 3, 270)], 0),
            ],
            &[(270, &LAT)],
        );
        let out = strip(&b).unwrap().unwrap();
        for ifd in [60, 90, 120, 200] {
            assert!(!tags(&out, ifd).contains(&TAG_GPS_IFD), "IFD at {ifd}");
        }
        assert!(out[250..294].iter().all(|&x| x == 0));
        assert_eq!(strip(&out).unwrap(), None);
    }

    #[test]
    fn ifd_loop_and_dangling_pointers_end_the_walk() {
        // IFD0 -> IFD1 -> IFD0 (loop); Exif pointer far outside the block.
        let b = build(
            100,
            8,
            &[
                (8, vec![(TAG_EXIF_IFD, 4, 1, 0xFFFF_FF00)], 40),
                (40, vec![(0x0103, 3, 1, 6)], 8),
            ],
            &[],
        );
        assert_eq!(strip(&b).unwrap(), None);
        // GPS pointer outside the block: the pointer is still removed.
        let b = build(
            60,
            8,
            &[(8, vec![(TAG_GPS_IFD, 4, 1, 0xFFFF_FF00)], 0)],
            &[],
        );
        let out = strip(&b).unwrap().unwrap();
        assert_eq!(tags(&out, 8), Vec::<u16>::new());
    }

    #[test]
    fn shared_value_bytes_are_not_zeroed() {
        // The GPS IFD's value offset points into the Exif IFD's date string.
        let b = build(
            200,
            8,
            &[
                (
                    8,
                    vec![(TAG_EXIF_IFD, 4, 1, 40), (TAG_GPS_IFD, 4, 1, 100)],
                    0,
                ),
                (40, vec![(0x9003, 2, 20, 150)], 0),
                (100, vec![(0x1D, 2, 11, 150)], 0),
            ],
            &[(150, b"2026:01:02 03:04:05\0")],
        );
        let out = strip(&b).unwrap().unwrap();
        assert_eq!(&out[150..170], b"2026:01:02 03:04:05\0");
        assert!(out[100..118].iter().all(|&x| x == 0));
    }

    #[test]
    fn malformed_blocks_are_errors() {
        assert!(strip(b"XX\x00\x2a\x00\x00\x00\x08").is_err());
        assert!(strip(b"MM\x00\x2b\x00\x00\x00\x08").is_err());
        assert!(
            strip(b"MM\x00\x2a\x00\x00\x00\x40").is_err(),
            "IFD0 outside"
        );
        assert!(
            strip(b"MM\x00\x2a\x00\x00\x00\x08\x00\x09").is_err(),
            "IFD0 entries overrun"
        );
        assert!(strip(b"MM").is_err());
        let mut b = build(40, 8, &[(8, vec![(TAG_EXIF_IFD, 4, 1, 30)], 0)], &[]);
        b[30..32].copy_from_slice(&[0x10, 0x00]);
        assert!(
            strip(&b).is_err(),
            "sub-IFD inside the block whose entries overrun"
        );
    }

    #[test]
    fn little_endian_blocks_work() {
        let mut b = vec![0u8; 64];
        b[0..8].copy_from_slice(b"II\x2a\x00\x08\x00\x00\x00");
        b[8..10].copy_from_slice(&1u16.to_le_bytes());
        b[10..12].copy_from_slice(&TAG_GPS_IFD.to_le_bytes());
        b[12..14].copy_from_slice(&4u16.to_le_bytes());
        b[14..18].copy_from_slice(&1u32.to_le_bytes());
        b[18..22].copy_from_slice(&30u32.to_le_bytes());
        b[30..32].copy_from_slice(&1u16.to_le_bytes());
        b[32..34].copy_from_slice(&1u16.to_le_bytes());
        b[34..36].copy_from_slice(&2u16.to_le_bytes());
        b[36..40].copy_from_slice(&2u32.to_le_bytes());
        b[40..42].copy_from_slice(b"N\0");
        let out = strip(&b).unwrap().unwrap();
        assert_eq!(&out[8..10], &[0, 0]);
        assert!(out[30..48].iter().all(|&x| x == 0));
    }
}
