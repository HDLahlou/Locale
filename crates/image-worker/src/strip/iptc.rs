//! IPTC-IIM location strip, inside Photoshop image resources (JPEG APP13, PNG
//! raw profiles) or as bare IIM data.

use super::MetaError;

/// Location data sets in record 2: 26 location code, 27 location name, 90
/// city, 92 sub-location, 95 province/state, 100 country code, 101 country.
const LOCATION_DATASETS: [u8; 7] = [26, 27, 90, 92, 95, 100, 101];

/// Photoshop resource holding IPTC-IIM data.
const RESOURCE_IPTC: u16 = 0x0404;

/// Strips IPTC location records from a Photoshop image-resource block (the
/// bytes after `Photoshop 3.0\0`). Returns the modified block when any were
/// found.
pub(super) fn strip_irb(buf: &[u8]) -> Result<Option<Vec<u8>>, MetaError> {
    let mut out: Option<Vec<u8>> = None;
    let mut pos = 0usize;
    while pos < buf.len() {
        if buf[pos..].iter().all(|&b| b == 0) {
            break;
        }
        let sig = buf
            .get(pos..pos + 4)
            .ok_or(MetaError::Malformed("Photoshop resource truncated"))?;
        if !matches!(sig, b"8BIM" | b"PHUT" | b"AgHg" | b"DCSR" | b"MeSa") {
            return Err(MetaError::Malformed("Photoshop resource signature"));
        }
        let id = super::be_u16(buf, pos + 4)
            .ok_or(MetaError::Malformed("Photoshop resource truncated"))?;
        let name_len =
            *buf.get(pos + 6)
                .ok_or(MetaError::Malformed("Photoshop resource truncated"))? as usize;
        // Pascal string padded to an even total length.
        let name_total = (1 + name_len + 1) & !1;
        let size_at = pos + 6 + name_total;
        let size = super::be_u32(buf, size_at)
            .ok_or(MetaError::Malformed("Photoshop resource truncated"))?
            as usize;
        let start = size_at + 4;
        let end = start
            .checked_add(size)
            .filter(|&e| e <= buf.len())
            .ok_or(MetaError::Malformed("Photoshop resource runs past the end"))?;
        if id == RESOURCE_IPTC && sig == b"8BIM" {
            if let Some(new) = strip_iim(&buf[start..end])? {
                out.get_or_insert_with(|| buf.to_vec())[start..end].copy_from_slice(&new);
            }
        }
        // Data is padded to an even length; the last pad byte may be missing.
        pos = end + (size & 1);
    }
    Ok(out)
}

/// Strips location data sets from IPTC-IIM data. Kept data sets move up in
/// order and the freed tail is filled with zero bytes. Returns the modified
/// data when any were found.
pub(super) fn strip_iim(buf: &[u8]) -> Result<Option<Vec<u8>>, MetaError> {
    let mut kept = Vec::with_capacity(buf.len());
    let mut removed = false;
    let mut pos = 0usize;
    while pos < buf.len() {
        if buf[pos] != 0x1C {
            // Zero padding ends the data (some writers pad, and so does this
            // strip); anything else cannot be parsed safely.
            if buf[pos..].iter().all(|&b| b == 0) {
                break;
            }
            return Err(MetaError::Malformed("IPTC data set marker"));
        }
        let head = buf
            .get(pos..pos + 5)
            .ok_or(MetaError::Malformed("IPTC data set truncated"))?;
        let (record, dataset) = (head[1], head[2]);
        let len = u16::from_be_bytes([head[3], head[4]]);
        let (header, data_len) = if len & 0x8000 != 0 {
            let n = (len & 0x7FFF) as usize;
            if n == 0 || n > 4 {
                return Err(MetaError::Malformed("IPTC extended length"));
            }
            let bytes = buf
                .get(pos + 5..pos + 5 + n)
                .ok_or(MetaError::Malformed("IPTC data set truncated"))?;
            let v = bytes.iter().fold(0usize, |acc, &b| (acc << 8) | b as usize);
            (5 + n, v)
        } else {
            (5, len as usize)
        };
        let end = (pos + header)
            .checked_add(data_len)
            .filter(|&e| e <= buf.len())
            .ok_or(MetaError::Malformed("IPTC data set runs past the end"))?;
        if record == 2 && LOCATION_DATASETS.contains(&dataset) {
            removed = true;
        } else {
            kept.extend_from_slice(&buf[pos..end]);
        }
        pos = end;
    }
    if !removed {
        return Ok(None);
    }
    kept.resize(buf.len(), 0);
    Ok(Some(kept))
}

/// Lists (record, data set) pairs in IPTC-IIM data, for tests.
#[cfg(test)]
pub(super) fn datasets(buf: &[u8]) -> Vec<(u8, u8)> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos + 5 <= buf.len() && buf[pos] == 0x1C {
        out.push((buf[pos + 1], buf[pos + 2]));
        pos += 5 + u16::from_be_bytes([buf[pos + 3], buf[pos + 4]]) as usize;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(record: u8, dataset: u8, value: &[u8]) -> Vec<u8> {
        let mut v = vec![0x1C, record, dataset];
        v.extend_from_slice(&(value.len() as u16).to_be_bytes());
        v.extend_from_slice(value);
        v
    }

    fn irb(id: u16, name: &[u8], data: &[u8]) -> Vec<u8> {
        let mut v = b"8BIM".to_vec();
        v.extend_from_slice(&id.to_be_bytes());
        v.push(name.len() as u8);
        v.extend_from_slice(name);
        if (1 + name.len()) % 2 == 1 {
            v.push(0);
        }
        v.extend_from_slice(&(data.len() as u32).to_be_bytes());
        v.extend_from_slice(data);
        if data.len() % 2 == 1 {
            v.push(0);
        }
        v
    }

    #[test]
    fn location_records_removed_others_kept_in_order() {
        let mut iim = ds(1, 90, b"\x1b%G");
        iim.extend(ds(2, 0, b"\x00\x04"));
        for (d, v) in [
            (26, &b"LC"[..]),
            (27, b"Name"),
            (90, b"Testville"),
            (92, b"Test Quarter"),
        ] {
            iim.extend(ds(2, d, v));
        }
        iim.extend(ds(2, 55, b"20260102"));
        for (d, v) in [(95, &b"State"[..]), (100, b"TL"), (101, b"Testland")] {
            iim.extend(ds(2, d, v));
        }
        let out = strip_iim(&iim).unwrap().unwrap();
        assert_eq!(out.len(), iim.len());
        assert_eq!(datasets(&out), vec![(1, 90), (2, 0), (2, 55)]);
        let text = String::from_utf8_lossy(&out);
        assert!(!text.contains("Testville") && !text.contains("Testland"));
        assert_eq!(strip_iim(&out).unwrap(), None, "idempotent");
    }

    #[test]
    fn irb_with_iptc_resource_and_neighbours() {
        let mut iim = ds(2, 90, b"Testville");
        iim.extend(ds(2, 120, b"caption"));
        let mut block = irb(0x0425, b"", &[7; 16]);
        block.extend(irb(RESOURCE_IPTC, b"ab", &iim));
        block.extend(irb(0x040C, b"", b"odd"));
        let out = strip_irb(&block).unwrap().unwrap();
        assert_eq!(out.len(), block.len());
        assert!(!String::from_utf8_lossy(&out).contains("Testville"));
        assert!(String::from_utf8_lossy(&out).contains("caption"));
        assert_eq!(strip_irb(&out).unwrap(), None);
    }

    #[test]
    fn extended_length_data_sets_are_handled() {
        let mut iim = vec![0x1C, 2, 90, 0x80, 0x02, 0x00, 0x03];
        iim.extend_from_slice(b"abc");
        iim.extend(ds(2, 120, b"x"));
        let out = strip_iim(&iim).unwrap().unwrap();
        assert_eq!(datasets(&out), vec![(2, 120)]);
    }

    #[test]
    fn malformed_data_fails_closed() {
        assert!(strip_iim(&[0x1C, 2, 90, 0, 9, b'a']).is_err(), "overrun");
        assert!(strip_iim(&[0x1C, 2, 90]).is_err(), "short header");
        assert!(
            strip_iim(&[0x1C, 2, 90, 0x80, 0x00]).is_err(),
            "zero extended length"
        );
        assert!(strip_iim(&[0x1C, 2, 90, 0x80, 0x05, 0, 0, 0, 0, 0]).is_err());
        assert!(
            strip_iim(&[0x1C, 2, 0, 0, 0, 0x55]).is_err(),
            "junk after data"
        );
        assert_eq!(
            strip_iim(&[0x1C, 2, 0, 0, 0, 0, 0]).unwrap(),
            None,
            "zero padding"
        );
        assert!(strip_irb(b"8BIM\x04\x04\x00\x00\x00\x00\x00\x10").is_err());
        assert!(strip_irb(b"XXXX\x04\x04\x00\x00\x00\x00\x00\x00").is_err());
        assert!(strip_irb(b"8BIM\x04").is_err());
        assert_eq!(strip_irb(b"\0\0\0").unwrap(), None);
    }
}
