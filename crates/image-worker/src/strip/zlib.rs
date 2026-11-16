//! zlib helpers for compressed PNG text chunks: inflate with a size limit, and
//! deflate to an exact length by padding with spaces inside the stream.

use miniz_oxide::deflate::core::{
    compress, create_comp_flags_from_zip_params, CompressorOxide, TDEFLFlush, TDEFLStatus,
};

/// Most bytes a compressed metadata chunk may inflate to; more is treated as
/// malformed (a decompression bomb).
pub(super) const INFLATE_LIMIT: usize = 64 << 20;

/// Inflates a zlib stream; `None` when it is invalid or larger than
/// [`INFLATE_LIMIT`]. Bytes after the end of the stream are ignored.
pub(super) fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    inflate_with_limit(data, INFLATE_LIMIT)
}

fn inflate_with_limit(data: &[u8], limit: usize) -> Option<Vec<u8>> {
    miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, limit).ok()
}

/// Compresses `text` into a zlib stream of exactly `target` bytes. The deflate
/// data is sync-flushed, then stored blocks of spaces fill the remaining room,
/// so the stream inflates to `text` followed by spaces. `None` when the
/// compressed text does not fit.
pub(super) fn deflate_exact(text: &[u8], target: usize) -> Option<Vec<u8>> {
    // Level 10 is miniz's best; negative window bits mean raw deflate.
    let flags = create_comp_flags_from_zip_params(10, -15, 0);
    let mut c = Box::new(CompressorOxide::new(flags));
    let mut raw = vec![0u8; text.len() + text.len() / 8 + 1024];
    let (status, used_in, used_out) = compress(&mut c, text, &mut raw, TDEFLFlush::Sync);
    if status != TDEFLStatus::Okay || used_in != text.len() || used_out == raw.len() {
        return None;
    }
    raw.truncate(used_out);

    let mut out = Vec::with_capacity(target);
    out.extend_from_slice(&[0x78, 0xDA]);
    out.extend_from_slice(&raw);
    // Room for the padding blocks: everything but the Adler-32 trailer. At
    // least one (final) stored block of 5 header bytes is needed.
    let mut room = target.checked_sub(out.len() + 4)?;
    if room < 5 {
        return None;
    }
    let mut adler = Adler32::default();
    adler.update(text);
    while room > 0 {
        let (payload, last) = if room <= 65_535 + 5 {
            (room - 5, true)
        } else {
            (65_535.min(room - 10), false)
        };
        out.push(u8::from(last));
        out.extend_from_slice(&(payload as u16).to_le_bytes());
        out.extend_from_slice(&(!(payload as u16)).to_le_bytes());
        let spaces = vec![b' '; payload];
        out.extend_from_slice(&spaces);
        adler.update(&spaces);
        room -= payload + 5;
    }
    out.extend_from_slice(&adler.finish().to_be_bytes());
    (out.len() == target).then_some(out)
}

struct Adler32 {
    a: u32,
    b: u32,
}

impl Default for Adler32 {
    fn default() -> Self {
        Adler32 { a: 1, b: 0 }
    }
}

impl Adler32 {
    fn update(&mut self, data: &[u8]) {
        for chunk in data.chunks(5552) {
            for &x in chunk {
                self.a += u32::from(x);
                self.b += self.a;
            }
            self.a %= 65_521;
            self.b %= 65_521;
        }
    }

    fn finish(&self) -> u32 {
        (self.b << 16) | self.a
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// zlib-compresses `text` normally (for building test chunks).
    pub(crate) fn compress_normal(text: &[u8]) -> Vec<u8> {
        miniz_oxide::deflate::compress_to_vec_zlib(text, 6)
    }

    #[test]
    fn exact_length_streams_inflate_to_text_plus_spaces() {
        let text = b"\nexif\n      20\n0000000000aaaaaaaaaa0000000000aaaaaaaaaa\n".repeat(30);
        let normal = compress_normal(&text);
        for extra in [0usize, 1, 7, 100, 70_000, 140_000] {
            let target = normal.len() + 20 + extra;
            let out = deflate_exact(&text, target).expect("fits");
            assert_eq!(out.len(), target);
            let back = inflate(&out).unwrap();
            assert_eq!(&back[..text.len()], &text[..]);
            assert!(back[text.len()..].iter().all(|&b| b == b' '));
        }
        assert_eq!(deflate_exact(&text, 10), None);
    }

    #[test]
    fn adler_matches_known_value() {
        let mut a = Adler32::default();
        a.update(b"Wikipedia");
        assert_eq!(a.finish(), 0x11E6_0398);
    }

    #[test]
    fn inflate_rejects_garbage_and_bombs() {
        assert_eq!(inflate(b"not zlib"), None);
        let big = compress_normal(&[0u8; 4097]);
        assert_eq!(inflate_with_limit(&big, 4096), None);
        assert_eq!(inflate_with_limit(&big, 4097).map(|v| v.len()), Some(4097));
    }
}
