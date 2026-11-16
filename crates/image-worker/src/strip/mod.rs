//! Lossless location-metadata strip for JPEG, PNG and HEIC (digest §4.4 steps 2
//! and 15, §8.5, DU-20, DU-37).
//!
//! The strip removes location data only and never re-encodes pixels. It is
//! size-preserving for every format: location bytes are overwritten or
//! neutralised inside their existing segment, chunk or item, and no byte that
//! follows them moves, because other structures point at absolute or relative
//! offsets (HEIC item locations, the JPEG multi-picture table that locates an
//! HDR gain map, TIFF offsets inside EXIF). A file with no location data comes
//! out byte-identical.
//!
//! What counts as location data:
//! - EXIF: every tag in the GPS IFD. The GPS IFD pointer (tag `0x8825`) is
//!   removed from whichever IFD holds it (IFD0, IFD1, the Exif IFD, the
//!   Interop IFD or a SubIFD) and the old GPS IFD bytes are zeroed.
//! - XMP: `exif:GPS*`, `photoshop:City`, `photoshop:State`,
//!   `photoshop:Country`, `Iptc4xmpCore:Location`, `Iptc4xmpCore:CountryCode`,
//!   and the `Iptc4xmpExt:LocationCreated` and `LocationShown` structures,
//!   matched by namespace URI. The removed text is replaced with spaces, so the
//!   packet keeps its length.
//! - IPTC-IIM: records 2:26, 2:27, 2:90, 2:92, 2:95, 2:100 and 2:101. The
//!   remaining data sets move up and the resource is padded with zero bytes,
//!   which IIM readers treat as the end of the data.
//!
//! Where the metadata lives:
//! - JPEG: EXIF in APP1, XMP in APP1 and extended XMP, IPTC in APP13
//!   (Photoshop resource `0x0404`), in the primary image and in every embedded
//!   secondary image (listed in the MPF table, or concatenated after the
//!   primary image's EOI, such as an HDR gain map).
//! - PNG: `eXIf`, XMP in `iTXt`, and legacy raw-profile text chunks (`tEXt`,
//!   `zTXt`, `iTXt` with keyword `Raw profile type exif|app1|xmp|iptc|8bim`).
//!   Chunk CRCs are recomputed. A compressed chunk that holds location data is
//!   re-compressed and padded with spaces inside the zlib stream so the chunk
//!   keeps its length; if it cannot fit, the strip fails closed.
//! - HEIC: every `Exif` item and every `mime` item of type
//!   `application/rdf+xml`, located through the `meta` box (`iinf`, `iloc`,
//!   `idat`). The GPS data is overwritten in place (DU-37).
//!
//! Errors, decided per case (the caller fails closed on any error):
//! - Empty input, unknown magic bytes, or a HEIF file without an image brand:
//!   [`StripError::UnsupportedType`].
//! - A container that runs past the end of the file (a truncated file, a
//!   segment, chunk or box length larger than what is left):
//!   [`StripError::Truncated`].
//! - Metadata whose structure cannot be parsed safely (an IFD whose entries run
//!   past its block, a broken XMP packet, an IPTC data set that overruns, item
//!   extents outside the file, an edit plan that overlaps itself):
//!   [`StripError::Malformed`].
//! - Location data, or metadata that may hold it, that cannot be removed
//!   without changing the size (XMP not in UTF-8, an encrypted or encoded
//!   metadata item, a compressed chunk whose new stream does not fit):
//!   [`StripError::Unstrippable`].
//! - Unreachable data is ignored, never an error: an IFD chain that loops or
//!   whose next pointer leaves the block ends there; a sub-IFD pointer outside
//!   the block is skipped; bytes after the last image of a JPEG or after the
//!   PNG `IEND` chunk are left alone.
//!
//! The strip detects the type from the magic bytes only; comparing it with the
//! claimed type is the content precheck's job (task M0a-3). Task M0a-3's
//! `sniff` module was not in this checkout when this module was written, so the
//! few magic-byte checks the strip needs live privately in [`detect_format`].
//!
//! The module is pure Rust with no OS calls and no `unsafe`, so it runs both in
//! the client precheck (phone and desktop) and in the host's worker.

mod heic;
mod iptc;
mod jpeg;
mod png;
mod rawprofile;
mod tiff;
mod xmp;
mod zlib;

use std::fmt;
use std::ops::Range;

/// An image container the strip understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// JPEG (JFIF or EXIF), including multi-picture files.
    Jpeg,
    /// PNG.
    Png,
    /// HEIC or HEIF (ISO-BMFF with an image brand).
    Heic,
}

impl Format {
    /// Lower-case name, as used in error messages.
    pub fn as_str(self) -> &'static str {
        match self {
            Format::Jpeg => "jpeg",
            Format::Png => "png",
            Format::Heic => "heic",
        }
    }
}

/// A kind of metadata that held location data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LocationKind {
    /// EXIF GPS IFD.
    Exif,
    /// XMP location properties.
    Xmp,
    /// IPTC-IIM location records.
    Iptc,
}

impl LocationKind {
    /// `exif`, `xmp` or `iptc`.
    pub fn as_str(self) -> &'static str {
        match self {
            LocationKind::Exif => "exif",
            LocationKind::Xmp => "xmp",
            LocationKind::Iptc => "iptc",
        }
    }
}

/// Which kinds of location data were found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Kinds {
    /// An EXIF GPS IFD (or a pointer to one) was found.
    pub exif: bool,
    /// An XMP location property was found.
    pub xmp: bool,
    /// An IPTC location record was found.
    pub iptc: bool,
}

impl Kinds {
    /// True when no location data was found.
    pub fn is_empty(&self) -> bool {
        !(self.exif || self.xmp || self.iptc)
    }

    /// Marks `kind` as found.
    pub fn insert(&mut self, kind: LocationKind) {
        match kind {
            LocationKind::Exif => self.exif = true,
            LocationKind::Xmp => self.xmp = true,
            LocationKind::Iptc => self.iptc = true,
        }
    }

    /// True when `kind` was found.
    pub fn contains(&self, kind: LocationKind) -> bool {
        match kind {
            LocationKind::Exif => self.exif,
            LocationKind::Xmp => self.xmp,
            LocationKind::Iptc => self.iptc,
        }
    }

    /// The found kinds, in the order exif, xmp, iptc.
    pub fn to_vec(&self) -> Vec<LocationKind> {
        [LocationKind::Exif, LocationKind::Xmp, LocationKind::Iptc]
            .into_iter()
            .filter(|k| self.contains(*k))
            .collect()
    }
}

/// A metadata container (JPEG segment, PNG chunk or HEIC item extent) that held
/// location data. Every byte the strip changes lies inside one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// Which kind of location data the container held.
    pub kind: LocationKind,
    /// Byte range of the whole container in the file: for JPEG the segment from
    /// its `FF xx` marker, for PNG the chunk from its length field to its CRC,
    /// for HEIC one extent of the item.
    pub range: Range<usize>,
}

/// What a strip found and did, without the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripReport {
    /// Container detected from the magic bytes.
    pub format: Format,
    /// True when the output differs from the input.
    pub changed: bool,
    /// Kinds of location data found (and removed).
    pub kinds: Kinds,
    /// Containers that held location data, in file order.
    pub regions: Vec<Region>,
}

/// Result of [`strip_location`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StripOutcome {
    /// The stripped file. Always the same length as the input.
    pub bytes: Vec<u8>,
    /// Container detected from the magic bytes.
    pub format: Format,
    /// True when `bytes` differs from the input.
    pub changed: bool,
    /// Kinds of location data found (and removed).
    pub kinds: Kinds,
    /// Containers that held location data, in file order.
    pub regions: Vec<Region>,
}

/// Why a file could not be stripped. The caller fails closed on every variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StripError {
    /// Empty input, magic bytes that are not JPEG, PNG or HEIF, or a HEIF file
    /// whose brands are not an image brand.
    UnsupportedType,
    /// A container runs past the end of the file.
    Truncated(Format),
    /// Metadata whose structure cannot be parsed safely.
    Malformed {
        /// Container.
        format: Format,
        /// What was wrong, for logs.
        detail: &'static str,
    },
    /// Location data (or metadata that may hold it) cannot be removed without
    /// changing the file's size.
    Unstrippable {
        /// Container.
        format: Format,
        /// What could not be stripped, for logs.
        detail: &'static str,
    },
}

impl fmt::Display for StripError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StripError::UnsupportedType => write!(f, "not a JPEG, PNG or HEIC image"),
            StripError::Truncated(format) => write!(f, "{} file is truncated", format.as_str()),
            StripError::Malformed { format, detail } => {
                write!(f, "{} metadata is malformed: {detail}", format.as_str())
            }
            StripError::Unstrippable { format, detail } => {
                write!(
                    f,
                    "{} location data cannot be stripped losslessly: {detail}",
                    format.as_str()
                )
            }
        }
    }
}

impl std::error::Error for StripError {}

/// Strips location metadata from a JPEG, PNG or HEIC file.
///
/// Returns the output bytes (always the input's length), whether anything
/// changed, and which kinds of location data were found. A file without
/// location data comes back byte-identical with `changed == false`.
pub fn strip_location(bytes: &[u8]) -> Result<StripOutcome, StripError> {
    let (format, plan) = plan(bytes)?;
    let mut out = bytes.to_vec();
    let changed = plan.apply(&mut out);
    Ok(StripOutcome {
        bytes: out,
        format,
        changed,
        kinds: plan.kinds,
        regions: plan.regions,
    })
}

/// Strips location metadata in place, for large files on a phone (no second
/// copy of the file). On error the buffer is left untouched: the whole edit
/// plan is built from the unmodified bytes before any byte is written.
pub fn strip_location_in_place(buf: &mut [u8]) -> Result<StripReport, StripError> {
    let (format, plan) = plan(buf)?;
    let changed = plan.apply(buf);
    Ok(StripReport {
        format,
        changed,
        kinds: plan.kinds,
        regions: plan.regions,
    })
}

/// Reports what [`strip_location`] would find and change, without writing.
/// After a strip, this finds nothing and reports `changed == false`.
pub fn scan_location(bytes: &[u8]) -> Result<StripReport, StripError> {
    let (format, plan) = plan(bytes)?;
    Ok(StripReport {
        format,
        changed: !plan.edits.is_empty(),
        kinds: plan.kinds,
        regions: plan.regions,
    })
}

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// HEIF brands that mark an image file (ISO/IEC 23008-12). Any of them as the
/// major or a compatible brand makes the file an image HEIF.
const HEIF_IMAGE_BRANDS: &[&[u8; 4]] = &[
    b"heic", b"heix", b"heim", b"heis", b"hevc", b"hevx", b"hevm", b"hevs", b"mif1", b"mif2",
    b"msf1",
];

/// Detects the container from the magic bytes (private stand-in for M0a-3's
/// `sniff` module).
fn detect_format(bytes: &[u8]) -> Result<Format, StripError> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok(Format::Jpeg);
    }
    if bytes.starts_with(PNG_SIGNATURE) {
        return Ok(Format::Png);
    }
    if bytes.get(4..8) == Some(b"ftyp") {
        let size = be_u32(bytes, 0).ok_or(StripError::UnsupportedType)? as usize;
        let ftyp = bytes.get(8..size.min(bytes.len())).unwrap_or(&[]);
        if size < 16 {
            return Err(StripError::UnsupportedType);
        }
        // Major brand, minor version (skipped), compatible brands.
        let mut brands = ftyp
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 1)
            .map(|(_, b)| b);
        if brands.any(|b| HEIF_IMAGE_BRANDS.contains(&b)) {
            return Ok(Format::Heic);
        }
    }
    Err(StripError::UnsupportedType)
}

fn plan(bytes: &[u8]) -> Result<(Format, Plan), StripError> {
    let format = detect_format(bytes)?;
    let mut plan = Plan::default();
    match format {
        Format::Jpeg => jpeg::plan(bytes, &mut plan)?,
        Format::Png => png::plan(bytes, &mut plan)?,
        Format::Heic => heic::plan(bytes, &mut plan)?,
    }
    plan.finish(format)?;
    Ok((format, plan))
}

/// Edits to make, built entirely from the unmodified file.
#[derive(Debug, Default)]
struct Plan {
    edits: Vec<(usize, Vec<u8>)>,
    kinds: Kinds,
    regions: Vec<Region>,
}

impl Plan {
    /// Records a container that held location data.
    fn found(&mut self, kind: LocationKind, range: Range<usize>) {
        self.kinds.insert(kind);
        self.regions.push(Region { kind, range });
    }

    /// Plans `file[offset..offset + new.len()] = new`, if that changes bytes.
    fn edit(&mut self, file: &[u8], offset: usize, new: Vec<u8>) {
        let same = offset
            .checked_add(new.len())
            .and_then(|end| file.get(offset..end))
            .is_some_and(|old| old == new.as_slice());
        if !same {
            self.edits.push((offset, new));
        }
    }

    /// Sorts the edits and checks that they stay inside the file and do not
    /// overlap (identical duplicates are merged).
    fn finish(&mut self, format: Format) -> Result<(), StripError> {
        self.edits.sort_by_key(|(off, _)| *off);
        self.edits.dedup();
        self.regions.sort_by_key(|r| (r.range.start, r.range.end));
        self.regions.dedup();
        let mut prev_end = 0;
        for (off, bytes) in &self.edits {
            if *off < prev_end {
                return Err(StripError::Malformed {
                    format,
                    detail: "metadata containers overlap",
                });
            }
            prev_end = off + bytes.len();
        }
        Ok(())
    }

    /// Writes the edits; returns true when any byte changed.
    fn apply(&self, buf: &mut [u8]) -> bool {
        let mut changed = false;
        for (off, bytes) in &self.edits {
            if let Some(dst) = buf.get_mut(*off..off + bytes.len()) {
                changed |= dst != bytes.as_slice();
                dst.copy_from_slice(bytes);
            }
        }
        changed
    }
}

/// A metadata block stored as several byte ranges of the file (extended XMP
/// parts, multi-segment Photoshop resources, multi-extent HEIC items).
#[derive(Debug, Default)]
struct Pieces(Vec<Range<usize>>);

impl Pieces {
    fn gather(&self, file: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for r in &self.0 {
            out.extend_from_slice(&file[r.clone()]);
        }
        out
    }

    /// Plans writing `new` (same length as the gathered block) back to the
    /// pieces it came from.
    fn scatter(&self, file: &[u8], new: &[u8], plan: &mut Plan) {
        let mut at = 0;
        for r in &self.0 {
            let len = r.len();
            if let Some(part) = new.get(at..at + len) {
                plan.edit(file, r.start, part.to_vec());
            }
            at += len;
        }
    }
}

/// Error from a format-agnostic metadata parser (TIFF, XMP, IPTC), mapped to a
/// [`StripError`] by the container that called it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetaError {
    Malformed(&'static str),
    Unstrippable(&'static str),
}

impl MetaError {
    fn at(self, format: Format) -> StripError {
        match self {
            MetaError::Malformed(detail) => StripError::Malformed { format, detail },
            MetaError::Unstrippable(detail) => StripError::Unstrippable { format, detail },
        }
    }
}

fn be_u16(b: &[u8], off: usize) -> Option<u16> {
    let s = b.get(off..off.checked_add(2)?)?;
    Some(u16::from_be_bytes([s[0], s[1]]))
}

fn be_u32(b: &[u8], off: usize) -> Option<u32> {
    let s = b.get(off..off.checked_add(4)?)?;
    Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

fn be_u64(b: &[u8], off: usize) -> Option<u64> {
    let s = b.get(off..off.checked_add(8)?)?;
    let mut a = [0u8; 8];
    a.copy_from_slice(s);
    Some(u64::from_be_bytes(a))
}

/// Reads a big-endian unsigned integer of `size` bytes (0, 4 or 8, as in
/// `iloc`).
fn be_uint(b: &[u8], off: usize, size: usize) -> Option<u64> {
    match size {
        0 => Some(0),
        2 => be_u16(b, off).map(u64::from),
        4 => be_u32(b, off).map(u64::from),
        8 => be_u64(b, off),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_formats_by_magic() {
        assert_eq!(detect_format(b"\xFF\xD8\xFF\xE0"), Ok(Format::Jpeg));
        assert_eq!(detect_format(PNG_SIGNATURE), Ok(Format::Png));
        let heic = b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00mif1heic";
        assert_eq!(detect_format(heic), Ok(Format::Heic));
        let avif_with_mif1 = b"\x00\x00\x00\x18ftypavif\x00\x00\x00\x00mif1miaf";
        assert_eq!(detect_format(avif_with_mif1), Ok(Format::Heic));
        let mp4 = b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00mp42isom";
        assert_eq!(detect_format(mp4), Err(StripError::UnsupportedType));
        // The minor version is not a brand.
        let minor = b"\x00\x00\x00\x14ftypmp42heicisom";
        assert_eq!(detect_format(minor), Err(StripError::UnsupportedType));
        assert_eq!(detect_format(b""), Err(StripError::UnsupportedType));
        assert_eq!(detect_format(b"\xFF\xD8"), Err(StripError::UnsupportedType));
        assert_eq!(
            detect_format(b"\x00\x00\x00\x04ftyp"),
            Err(StripError::UnsupportedType)
        );
    }

    #[test]
    fn overlapping_edits_are_rejected() {
        let mut plan = Plan::default();
        let file = [0u8; 8];
        plan.edit(&file, 0, vec![1, 1, 1]);
        plan.edit(&file, 2, vec![2, 2]);
        assert!(matches!(
            plan.finish(Format::Heic),
            Err(StripError::Malformed { .. })
        ));
    }

    #[test]
    fn identical_edits_are_merged_and_no_op_edits_dropped() {
        let mut plan = Plan::default();
        let file = [0u8; 8];
        plan.edit(&file, 1, vec![1, 1]);
        plan.edit(&file, 1, vec![1, 1]);
        plan.edit(&file, 4, vec![0, 0]);
        assert!(plan.finish(Format::Jpeg).is_ok());
        assert_eq!(plan.edits, vec![(1, vec![1, 1])]);
    }

    #[test]
    fn error_messages_name_the_format() {
        let e = StripError::Malformed {
            format: Format::Png,
            detail: "x",
        };
        assert_eq!(e.to_string(), "png metadata is malformed: x");
        assert_eq!(
            StripError::Truncated(Format::Heic).to_string(),
            "heic file is truncated"
        );
    }
}
