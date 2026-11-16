//! macOS back end: ImageIO through the `objc2` bindings, plus the process
//! memory reading the parent uses to enforce the worker's memory limit.
//!
//! This is the only module in the crate allowed to use `unsafe`.

use objc2_core_foundation::{
    CFBoolean, CFData, CFDictionary, CFMutableData, CFNumber, CFRetained, CFString, CFType,
};
use objc2_core_graphics::{CGDataProvider, CGImage};
use objc2_image_io::{
    kCGImageAuxiliaryDataTypeHDRGainMap, kCGImageDestinationLossyCompressionQuality,
    kCGImagePropertyOrientation, kCGImagePropertyPixelHeight, kCGImagePropertyPixelWidth,
    kCGImageSourceCreateThumbnailFromImageAlways, kCGImageSourceCreateThumbnailWithTransform,
    kCGImageSourceShouldCache, kCGImageSourceShouldCacheImmediately,
    kCGImageSourceThumbnailMaxPixelSize, kCGImageSourceTypeIdentifierHint, CGImageDestination,
    CGImageSource, CGImageSourceStatus,
};

use crate::codec::{
    CodecError, Encoded, Facts, ImageCodec, ImageFormat, COMPATIBLE_QUALITY, PREVIEW_LONG_EDGE,
    PREVIEW_QUALITY,
};
use crate::sniff::sniff;
use crate::structure;

/// The ImageIO codec.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImageIoCodec;

const UTI_JPEG: &str = "public.jpeg";

/// `kCGImageAuxiliaryDataTypeISOGainMap` exists only from macOS 15; linking
/// the symbol would stop the worker loading on macOS 13 and 14, so the key is
/// built from its documented string value instead.
const ISO_GAIN_MAP_KEY: &str = "kCGImageAuxiliaryDataTypeISOGainMap";

fn allowed_utis(format: ImageFormat) -> &'static [&'static str] {
    match format {
        ImageFormat::Jpeg => &["public.jpeg"],
        ImageFormat::Png => &["public.png"],
        ImageFormat::Heic => &["public.heic", "public.heif"],
    }
}

type Options = CFRetained<CFDictionary<CFString, CFType>>;

fn options(pairs: &[(&CFString, &CFType)]) -> Options {
    let keys: Vec<&CFString> = pairs.iter().map(|(k, _)| *k).collect();
    let values: Vec<&CFType> = pairs.iter().map(|(_, v)| *v).collect();
    CFDictionary::from_slices(&keys, &values)
}

fn yes() -> &'static CFType {
    CFBoolean::new(true)
}

fn no() -> &'static CFType {
    CFBoolean::new(false)
}

/// An opened, checked image source.
struct Source {
    source: CFRetained<CGImageSource>,
    index: usize,
    facts: Facts,
}

impl Source {
    /// Sniff, open and fully decode the primary image. Any irregularity is
    /// `DecodeFailed`.
    fn open(bytes: &[u8]) -> Result<Source, CodecError> {
        let format = sniff(bytes).map_err(|_| CodecError::NotAnImage)?;
        structure::check(format, bytes).map_err(|_| CodecError::DecodeFailed)?;
        let data = CFData::from_bytes(bytes);
        let hint = CFString::from_static_str(allowed_utis(format)[0]);
        // SAFETY: the extern statics are immutable CFString constants exported
        // by ImageIO.
        let opts = unsafe {
            options(&[
                (kCGImageSourceShouldCache, no()),
                (kCGImageSourceTypeIdentifierHint, &hint),
            ])
        };
        // SAFETY: `opts` maps CFString keys to CF values, as ImageIO expects.
        let source = unsafe { CGImageSource::with_data(&data, Some(opts.as_opaque())) }
            .ok_or(CodecError::DecodeFailed)?;

        // The decoder ImageIO picked must agree with the sniffed format.
        // SAFETY: `source` is a valid image source.
        let uti = unsafe { source.r#type() }.ok_or(CodecError::DecodeFailed)?;
        let uti = uti.to_string();
        if !allowed_utis(format).contains(&uti.as_str()) {
            return Err(CodecError::DecodeFailed);
        }
        // SAFETY: `source` is a valid image source.
        let (count, status, index) = unsafe {
            (
                source.count(),
                source.status(),
                source.primary_image_index(),
            )
        };
        if count == 0 || index >= count || status != CGImageSourceStatus::StatusComplete {
            return Err(CodecError::DecodeFailed);
        }

        // SAFETY: `source` is valid and `index < count`; no options.
        let props =
            unsafe { source.properties_at_index(index, None) }.ok_or(CodecError::DecodeFailed)?;
        // SAFETY: image properties dictionaries have CFString keys.
        let props: &CFDictionary<CFString, CFType> = unsafe { props.cast_unchecked() };
        // SAFETY: immutable CFString constants exported by ImageIO.
        let (wkey, hkey, okey) = unsafe {
            (
                kCGImagePropertyPixelWidth,
                kCGImagePropertyPixelHeight,
                kCGImagePropertyOrientation,
            )
        };
        let width = number(props, wkey).ok_or(CodecError::DecodeFailed)?;
        let height = number(props, hkey).ok_or(CodecError::DecodeFailed)?;
        let orientation = match number(props, okey) {
            None => 1,
            Some(o @ 1..=8) => o as u8,
            Some(_) => return Err(CodecError::DecodeFailed),
        };
        if width == 0 || height == 0 {
            return Err(CodecError::DecodeFailed);
        }

        // Decode every pixel now, then make sure the decoder still reports a
        // complete image (a truncated stream leaves it incomplete).
        // SAFETY: immutable CFString constants exported by ImageIO.
        let decode_opts = unsafe {
            options(&[
                (kCGImageSourceShouldCache, yes()),
                (kCGImageSourceShouldCacheImmediately, yes()),
            ])
        };
        // SAFETY: valid source, `index < count`, options as ImageIO expects.
        let image = unsafe { source.image_at_index(index, Some(decode_opts.as_opaque())) }
            .ok_or(CodecError::DecodeFailed)?;
        if CGImage::width(Some(&image)) != width as usize
            || CGImage::height(Some(&image)) != height as usize
        {
            return Err(CodecError::DecodeFailed);
        }
        let provider = CGImage::data_provider(Some(&image)).ok_or(CodecError::DecodeFailed)?;
        let pixels = CGDataProvider::data(Some(&provider)).ok_or(CodecError::DecodeFailed)?;
        if pixels.is_empty() {
            return Err(CodecError::DecodeFailed);
        }
        drop(pixels);
        // SAFETY: valid source, `index < count`.
        let (status, status_at) = unsafe { (source.status(), source.status_at_index(index)) };
        if status != CGImageSourceStatus::StatusComplete
            || status_at != CGImageSourceStatus::StatusComplete
        {
            return Err(CodecError::DecodeFailed);
        }

        let has_hdr_gain_map = has_gain_map(&source, index);
        Ok(Source {
            source,
            index,
            facts: Facts {
                width,
                height,
                format,
                orientation,
                has_hdr_gain_map,
            },
        })
    }
}

fn number(props: &CFDictionary<CFString, CFType>, key: &CFString) -> Option<u32> {
    let value = props.get(key)?;
    let n = value.downcast::<CFNumber>().ok()?;
    let v = n.as_i64()?;
    u32::try_from(v).ok()
}

fn has_gain_map(source: &CGImageSource, index: usize) -> bool {
    let iso = CFString::from_static_str(ISO_GAIN_MAP_KEY);
    // SAFETY: the constant is an immutable CFString exported by ImageIO
    // (macOS 11+).
    let apple = unsafe { kCGImageAuxiliaryDataTypeHDRGainMap };
    [apple, &*iso].iter().any(|key| {
        // SAFETY: valid source and index; the key is a CFString.
        unsafe { source.auxiliary_data_info_at_index(index, key) }.is_some()
    })
}

fn encode_jpeg(
    facts: Facts,
    image: &CGImage,
    quality: u8,
    orientation: Option<u8>,
) -> Result<Encoded, CodecError> {
    let out = CFMutableData::new(None, 0).ok_or(CodecError::EncodeFailed)?;
    let uti = CFString::from_static_str(UTI_JPEG);
    // SAFETY: `out` is a valid mutable data object; one image; no options.
    let dest = unsafe { CGImageDestination::with_data(&out, &uti, 1, None) }
        .ok_or(CodecError::EncodeFailed)?;
    let q = CFNumber::new_f64(f64::from(quality) / 100.0);
    let o = orientation.map(|o| CFNumber::new_i32(i32::from(o)));
    // SAFETY: immutable CFString constants exported by ImageIO.
    let (qkey, okey) = unsafe {
        (
            kCGImageDestinationLossyCompressionQuality,
            kCGImagePropertyOrientation,
        )
    };
    let mut pairs: Vec<(&CFString, &CFType)> = vec![(qkey, &q)];
    if let Some(o) = &o {
        pairs.push((okey, o));
    }
    let props = options(&pairs);
    // SAFETY: valid destination and image; properties map CFString to CF
    // values as ImageIO expects.
    let ok = unsafe {
        dest.add_image(image, Some(props.as_opaque()));
        dest.finalize()
    };
    if !ok {
        return Err(CodecError::EncodeFailed);
    }
    drop(dest);
    let bytes = out.to_vec();
    if bytes.is_empty() {
        return Err(CodecError::EncodeFailed);
    }
    Ok(Encoded {
        bytes,
        width: CGImage::width(Some(image)) as u32,
        height: CGImage::height(Some(image)) as u32,
        facts,
    })
}

impl ImageCodec for ImageIoCodec {
    fn decode(&self, bytes: &[u8]) -> Result<Facts, CodecError> {
        Source::open(bytes).map(|s| s.facts)
    }

    fn preview(&self, bytes: &[u8]) -> Result<Encoded, CodecError> {
        let src = Source::open(bytes)?;
        let long_edge = src.facts.width.max(src.facts.height);
        let max = CFNumber::new_i32(long_edge.min(PREVIEW_LONG_EDGE) as i32);
        // SAFETY: immutable CFString constants exported by ImageIO.
        let opts = unsafe {
            options(&[
                (kCGImageSourceCreateThumbnailFromImageAlways, yes()),
                (kCGImageSourceCreateThumbnailWithTransform, yes()),
                (kCGImageSourceThumbnailMaxPixelSize, &max),
                (kCGImageSourceShouldCacheImmediately, yes()),
            ])
        };
        // SAFETY: valid source and index; options as ImageIO expects.
        let thumb = unsafe {
            src.source
                .thumbnail_at_index(src.index, Some(opts.as_opaque()))
        }
        .ok_or(CodecError::DecodeFailed)?;
        // Orientation is now in the pixels, so no orientation tag is written.
        encode_jpeg(src.facts, &thumb, PREVIEW_QUALITY, None)
    }

    fn compatible(&self, bytes: &[u8]) -> Result<Encoded, CodecError> {
        let src = Source::open(bytes)?;
        // SAFETY: immutable CFString constant exported by ImageIO.
        let opts = unsafe { options(&[(kCGImageSourceShouldCacheImmediately, yes())]) };
        // SAFETY: valid source and index; options as ImageIO expects.
        let image = unsafe { src.source.image_at_index(src.index, Some(opts.as_opaque())) }
            .ok_or(CodecError::DecodeFailed)?;
        // The CGImage carries its colour space, which the JPEG encoder embeds
        // as an ICC profile. Pixels stay as stored, so the orientation tag is
        // carried over; no other metadata is copied.
        let orientation = (src.facts.orientation != 1).then_some(src.facts.orientation);
        encode_jpeg(src.facts, &image, COMPATIBLE_QUALITY, orientation)
    }
}

/// Physical memory footprint of process `pid` in bytes (what Activity
/// Monitor calls "Memory"), or `None` if it cannot be read (for example the
/// process has exited).
pub(crate) fn phys_footprint(pid: u32) -> Option<u64> {
    let pid = libc::c_int::try_from(pid).ok()?;
    // SAFETY: `rusage_info_v2` is plain old data; all-zero is a valid value.
    let mut info: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
    let ptr: *mut libc::rusage_info_v2 = &mut info;
    // SAFETY: `ptr` points to a live, writable `rusage_info_v2`, which is the
    // structure `RUSAGE_INFO_V2` asks the kernel to fill.
    let rc = unsafe { libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V2, ptr.cast()) };
    (rc == 0).then_some(info.ri_phys_footprint)
}
