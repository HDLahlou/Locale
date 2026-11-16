//! Acceptance tests for the location strip (task M0a-5), against the fixtures
//! in `fixtures/` and their facts in `fixtures/manifest.json`.
//!
//! The checks here re-parse the output with small readers written for the
//! tests (EXIF IFD0, XMP text, IPTC data sets, container layout), so they do
//! not rely only on the strip's own parser.

mod strip {
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use image_worker::strip::{
        scan_location, strip_location, strip_location_in_place, Format, LocationKind, StripError,
        StripOutcome,
    };
    use serde_json::Value;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    fn read(rel: &str) -> Vec<u8> {
        std::fs::read(fixtures().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
    }

    fn manifest() -> Value {
        serde_json::from_slice(&read("manifest.json")).unwrap()
    }

    fn entry(m: &Value, path: &str) -> Value {
        m["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["path"] == path)
            .unwrap_or_else(|| panic!("{path}"))
            .clone()
    }

    const PLAIN: [&str; 5] = [
        "plain.jpg",
        "plain.png",
        "plain.heic",
        "orientation6.jpg",
        "p3.jpg",
    ];
    const WITH_LOCATION: [&str; 7] = [
        "gps_exif.jpg",
        "gps_xmp.jpg",
        "gps_iptc.jpg",
        "gps_all.jpg",
        "gps_exif.png",
        "gps_xmp.png",
        "gps_exif.heic",
    ];

    // ------------------------------------------------------------ test-side readers

    fn be16(b: &[u8], o: usize) -> usize {
        u16::from_be_bytes([b[o], b[o + 1]]) as usize
    }

    fn be32(b: &[u8], o: usize) -> usize {
        u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as usize
    }

    fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
        hay.windows(needle.len()).position(|w| w == needle)
    }

    fn contains(hay: &[u8], needle: &[u8]) -> bool {
        find(hay, needle).is_some()
    }

    fn format_of(b: &[u8]) -> Format {
        if b.starts_with(&[0xFF, 0xD8]) {
            Format::Jpeg
        } else if b.starts_with(b"\x89PNG") {
            Format::Png
        } else {
            Format::Heic
        }
    }

    /// JPEG marker segments of the primary image up to SOS:
    /// (marker, segment start, segment end).
    fn jpeg_segments(b: &[u8]) -> Vec<(u8, usize, usize)> {
        let mut out = Vec::new();
        let mut p = 2;
        while p + 4 <= b.len() && b[p] == 0xFF {
            let m = b[p + 1];
            let end = p + 2 + be16(b, p + 2);
            out.push((m, p, end));
            if m == 0xDA {
                break;
            }
            p = end;
        }
        out
    }

    /// PNG chunks: (type, chunk start, chunk end).
    fn png_chunks(b: &[u8]) -> Vec<([u8; 4], usize, usize)> {
        let mut out = Vec::new();
        let mut p = 8;
        while p + 12 <= b.len() {
            let len = be32(b, p);
            let mut t = [0u8; 4];
            t.copy_from_slice(&b[p + 4..p + 8]);
            out.push((t, p, p + 12 + len));
            if &t == b"IEND" {
                break;
            }
            p += 12 + len;
        }
        out
    }

    /// Top-level ISO-BMFF boxes: (type, start of the content, end).
    fn heic_boxes(b: &[u8]) -> Vec<([u8; 4], usize, usize)> {
        let mut out = Vec::new();
        let mut p = 0;
        while p + 8 <= b.len() {
            let size = be32(b, p);
            let mut t = [0u8; 4];
            t.copy_from_slice(&b[p + 4..p + 8]);
            let end = match size {
                0 => b.len(),
                1 => p + (be32(b, p + 8) << 32 | be32(b, p + 12)),
                n => p + n,
            };
            out.push((t, p + if size == 1 { 16 } else { 8 }, end));
            p = end;
        }
        out
    }

    /// The EXIF TIFF block of a file, if any.
    fn tiff_block(b: &[u8]) -> Option<&[u8]> {
        match format_of(b) {
            Format::Jpeg => jpeg_segments(b)
                .into_iter()
                .find(|&(m, s, _)| m == 0xE1 && b[s + 4..].starts_with(b"Exif\0"))
                .map(|(_, s, e)| &b[s + 10..e]),
            Format::Png => png_chunks(b)
                .into_iter()
                .find(|(t, _, _)| t == b"eXIf")
                .map(|(_, s, e)| &b[s + 8..e - 4]),
            // The Exif item: a 4-byte offset (6) then `Exif\0\0` and TIFF.
            Format::Heic => find(b, b"\0\0\0\x06Exif\0\0").map(|i| &b[i + 10..]),
        }
    }

    struct Exif {
        ifd0_tags: Vec<u16>,
        orientation: Option<u16>,
        capture_time: Option<String>,
    }

    fn read_exif(b: &[u8]) -> Option<Exif> {
        let t = tiff_block(b)?;
        let le = &t[0..2] == b"II";
        let u16_at = |o: usize| {
            let s = [t[o], t[o + 1]];
            if le {
                u16::from_le_bytes(s)
            } else {
                u16::from_be_bytes(s)
            }
        };
        let u32_at = |o: usize| -> usize {
            let s = [t[o], t[o + 1], t[o + 2], t[o + 3]];
            let v = if le {
                u32::from_le_bytes(s)
            } else {
                u32::from_be_bytes(s)
            };
            v as usize
        };
        let entries = |ifd: usize| (0..u16_at(ifd) as usize).map(move |i| ifd + 2 + 12 * i);
        let ifd0 = u32_at(4);
        let mut exif = Exif {
            ifd0_tags: Vec::new(),
            orientation: None,
            capture_time: None,
        };
        for e in entries(ifd0) {
            let tag = u16_at(e);
            exif.ifd0_tags.push(tag);
            if tag == 0x0112 {
                exif.orientation = Some(u16_at(e + 8));
            }
            if tag == 0x8769 {
                for x in entries(u32_at(e + 8)) {
                    if u16_at(x) == 0x9003 {
                        let at = u32_at(x + 8);
                        let s = String::from_utf8_lossy(&t[at..at + 19]).to_string();
                        exif.capture_time = Some(s);
                    }
                }
            }
        }
        Some(exif)
    }

    /// Capture time from EXIF, else from XMP (converted to EXIF's format).
    fn capture_time(b: &[u8]) -> Option<String> {
        if let Some(t) = read_exif(b).and_then(|e| e.capture_time) {
            return Some(t);
        }
        for key in [&b"exif:DateTimeOriginal=\""[..], b"<photoshop:DateCreated>"] {
            if let Some(i) = find(b, key) {
                let v = String::from_utf8_lossy(&b[i + key.len()..i + key.len() + 19]).to_string();
                return Some(v.replacen('-', ":", 2).replace('T', " "));
            }
        }
        None
    }

    /// Every IPTC (record, data set) in the file's 8BIM 0x0404 resources.
    fn iptc_datasets(b: &[u8]) -> Vec<(u8, u8)> {
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(i) = find(&b[from..], b"8BIM\x04\x04").map(|i| i + from) {
            let name_len = b[i + 6] as usize;
            let size_at = i + 6 + ((name_len + 2) & !1);
            let size = be32(b, size_at);
            let data = &b[size_at + 4..size_at + 4 + size];
            let mut p = 0;
            while p + 5 <= data.len() && data[p] == 0x1C {
                out.push((data[p + 1], data[p + 2]));
                p += 5 + be16(data, p + 3);
            }
            from = i + 6;
        }
        out
    }

    const XMP_LOCATION_NAMES: [&[u8]; 8] = [
        b"exif:GPS",
        b"photoshop:City",
        b"photoshop:State",
        b"photoshop:Country",
        b"Iptc4xmpCore:Location",
        b"Iptc4xmpCore:CountryCode",
        b"Iptc4xmpExt:LocationCreated",
        b"Iptc4xmpExt:LocationShown",
    ];
    const IPTC_LOCATION: [u8; 7] = [26, 27, 90, 92, 95, 100, 101];

    /// The manifest's synthetic location values in every encoding the
    /// fixtures use: ASCII text, and EXIF rationals in both byte orders.
    fn synthetic_values(m: &Value) -> Vec<(String, Vec<u8>)> {
        let loc = &m["location"];
        let mut out = Vec::new();
        for key in [
            "xmp_latitude",
            "xmp_longitude",
            "iptc_city",
            "iptc_sub_location",
            "iptc_country",
        ] {
            out.push((
                key.to_string(),
                loc[key].as_str().unwrap().as_bytes().to_vec(),
            ));
        }
        for key in ["latitude", "longitude"] {
            out.push((key.to_string(), loc[key].to_string().into_bytes()));
        }
        for key in ["exif_latitude_rationals", "exif_longitude_rationals"] {
            let rats: Vec<(u32, u32)> = loc[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| (r[0].as_u64().unwrap() as u32, r[1].as_u64().unwrap() as u32))
                .collect();
            let be: Vec<u8> = rats
                .iter()
                .flat_map(|(n, d)| [n.to_be_bytes(), d.to_be_bytes()].concat())
                .collect();
            let le: Vec<u8> = rats
                .iter()
                .flat_map(|(n, d)| [n.to_le_bytes(), d.to_le_bytes()].concat())
                .collect();
            out.push((format!("{key} (big-endian)"), be));
            out.push((format!("{key} (little-endian)"), le));
        }
        out
    }

    /// Asserts that every changed byte lies inside a reported region, and that
    /// every region is a metadata container (checked with the test readers),
    /// which also proves pixel data is byte-identical without a decoder.
    fn assert_changes_inside_metadata(name: &str, input: &[u8], out: &StripOutcome) {
        assert_eq!(out.bytes.len(), input.len(), "{name}: length");
        for (i, (a, b)) in input.iter().zip(&out.bytes).enumerate() {
            if a != b {
                assert!(
                    out.regions.iter().any(|r| r.range.contains(&i)),
                    "{name}: byte {i} changed outside metadata"
                );
            }
        }
        for r in &out.regions {
            let s = r.range.start;
            match out.format {
                Format::Jpeg => {
                    assert_eq!(input[s], 0xFF, "{name}: region at a marker");
                    assert!(
                        matches!(input[s + 1], 0xE1 | 0xED),
                        "{name}: region is APP1 or APP13"
                    );
                    assert_eq!(
                        r.range.len(),
                        2 + be16(input, s + 2),
                        "{name}: region is one segment"
                    );
                }
                Format::Png => {
                    assert!(
                        matches!(&input[s + 4..s + 8], b"eXIf" | b"iTXt" | b"tEXt" | b"zTXt"),
                        "{name}"
                    );
                    assert_eq!(
                        r.range.len(),
                        12 + be32(input, s),
                        "{name}: region is one chunk"
                    );
                }
                Format::Heic => {
                    let item = &input[r.range.clone()];
                    let mdat = heic_boxes(input)
                        .into_iter()
                        .find(|(t, _, _)| t == b"mdat")
                        .unwrap();
                    assert!(
                        r.range.start >= mdat.1 && r.range.end <= mdat.2,
                        "{name}: region {:?} inside mdat {:?}",
                        r.range,
                        mdat
                    );
                    assert!(
                        item.starts_with(b"\0\0\0\x06Exif\0\0") || contains(item, b"xmpmeta"),
                        "{name}"
                    );
                }
            }
        }
        // Pixel data compared directly as well.
        match out.format {
            Format::Jpeg => {
                let sos = jpeg_segments(input).last().unwrap().1;
                assert_eq!(&input[sos..], &out.bytes[sos..], "{name}: scan data");
            }
            Format::Png => {
                for (t, s, e) in png_chunks(input) {
                    if &t == b"IDAT" || &t == b"IHDR" {
                        assert_eq!(
                            &input[s..e],
                            &out.bytes[s..e],
                            "{name}: {}",
                            String::from_utf8_lossy(&t)
                        );
                    }
                }
            }
            Format::Heic => {
                let mdat = heic_boxes(input)
                    .into_iter()
                    .find(|(t, _, _)| t == b"mdat")
                    .unwrap();
                assert_eq!(
                    &input[..mdat.1],
                    &out.bytes[..mdat.1],
                    "{name}: ftyp and meta unchanged"
                );
            }
        }
    }

    fn assert_no_location_on_reparse(name: &str, b: &[u8]) {
        let scan = scan_location(b).unwrap();
        assert!(
            scan.kinds.is_empty() && !scan.changed,
            "{name}: strip's own re-parse"
        );
        if let Some(e) = read_exif(b) {
            assert!(
                !e.ifd0_tags.contains(&0x8825),
                "{name}: GPS IFD pointer left in IFD0"
            );
        }
        for n in XMP_LOCATION_NAMES {
            assert!(
                !contains(b, n),
                "{name}: XMP {} left",
                String::from_utf8_lossy(n)
            );
        }
        for (rec, ds) in iptc_datasets(b) {
            assert!(
                !(rec == 2 && IPTC_LOCATION.contains(&ds)),
                "{name}: IPTC 2:{ds} left"
            );
        }
    }

    // ------------------------------------------------------------ fixtures

    #[test]
    fn strip_plain_files_are_byte_identical() {
        for name in PLAIN {
            let input = read(&format!("images/{name}"));
            let out = strip_location(&input).unwrap();
            assert!(!out.changed, "{name}");
            assert!(out.kinds.is_empty() && out.regions.is_empty(), "{name}");
            assert_eq!(out.bytes, input, "{name}");
            let mut buf = input.clone();
            assert!(!strip_location_in_place(&mut buf).unwrap().changed);
            assert_eq!(buf, input, "{name}: in place");
        }
    }

    #[test]
    fn strip_removes_location_and_keeps_orientation_and_capture_time() {
        let m = manifest();
        let values = synthetic_values(&m);
        for name in WITH_LOCATION {
            let e = entry(&m, &format!("images/{name}"));
            let input = read(&format!("images/{name}"));
            let out = strip_location(&input).unwrap();
            assert!(out.changed, "{name}");
            assert_eq!(out.bytes.len(), input.len(), "{name}");

            let want: Vec<LocationKind> = e["location_kinds"]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| match k.as_str().unwrap() {
                    "exif" => LocationKind::Exif,
                    "xmp" => LocationKind::Xmp,
                    "iptc" => LocationKind::Iptc,
                    other => panic!("{other}"),
                })
                .collect();
            assert_eq!(out.kinds.to_vec(), want, "{name}: kinds found");

            let present: Vec<&String> = values
                .iter()
                .filter(|(_, v)| contains(&input, v))
                .map(|(k, _)| k)
                .collect();
            assert!(
                !present.is_empty(),
                "{name}: the input carries synthetic values"
            );
            for (key, v) in &values {
                assert!(
                    !contains(&out.bytes, v),
                    "{name}: {key} still in the output"
                );
            }

            assert_no_location_on_reparse(name, &out.bytes);

            let before = read_exif(&input);
            let after = read_exif(&out.bytes);
            assert_eq!(
                before.as_ref().map(|x| x.orientation),
                after.as_ref().map(|x| x.orientation),
                "{name}: orientation"
            );
            if let Some(o) = e["orientation"].as_u64() {
                assert_eq!(
                    after.and_then(|x| x.orientation),
                    Some(o as u16),
                    "{name}: orientation value"
                );
            }
            let time = e["capture_time"].as_str().map(str::to_string);
            assert_eq!(capture_time(&input), time, "{name}: capture time before");
            assert_eq!(capture_time(&out.bytes), time, "{name}: capture time after");

            let again = strip_location(&out.bytes).unwrap();
            assert!(!again.changed, "{name}: idempotent");
            assert_eq!(again.bytes, out.bytes, "{name}: idempotent bytes");
        }
    }

    #[test]
    fn strip_changes_stay_inside_metadata_containers() {
        for name in PLAIN.iter().chain(WITH_LOCATION.iter()) {
            let input = read(&format!("images/{name}"));
            let out = strip_location(&input).unwrap();
            assert_changes_inside_metadata(name, &input, &out);
            let mut buf = input.clone();
            let report = strip_location_in_place(&mut buf).unwrap();
            assert_eq!(buf, out.bytes, "{name}: in-place output matches");
            assert_eq!(report.regions, out.regions);
        }
    }

    #[test]
    fn strip_gps_all_reports_every_kind_and_region() {
        let input = read("images/gps_all.jpg");
        let out = strip_location(&input).unwrap();
        assert!(out.kinds.exif && out.kinds.xmp && out.kinds.iptc);
        let kinds: Vec<LocationKind> = out.regions.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            vec![LocationKind::Exif, LocationKind::Xmp, LocationKind::Iptc]
        );
        // XMP keeps its length and its non-location properties.
        let x = find(&out.bytes, b"<x:xmpmeta").unwrap();
        let xmp = String::from_utf8_lossy(&out.bytes[x..x + 400]);
        assert!(xmp.contains(r#"tiff:Orientation="6""#) && xmp.contains("exif:DateTimeOriginal"));
    }

    // ------------------------------------------------------------ malformed input

    #[test]
    fn strip_malformed_fixtures_give_documented_results() {
        let cases: [(&str, Result<(), StripError>); 8] = [
            ("not_an_image.jpg", Err(StripError::UnsupportedType)),
            ("empty.jpg", Err(StripError::UnsupportedType)),
            ("heic_bad_brand.heic", Err(StripError::UnsupportedType)),
            ("truncated.jpg", Err(StripError::Truncated(Format::Jpeg))),
            ("truncated.png", Err(StripError::Truncated(Format::Png))),
            ("truncated.heic", Err(StripError::Truncated(Format::Heic))),
            // The strip goes by magic bytes; the claimed-type check is the
            // precheck's. These are valid files without location data.
            ("wrong_magic.jpg", Ok(())),
            ("wrong_magic.heic", Ok(())),
        ];
        for (name, want) in cases {
            let input = read(&format!("images/{name}"));
            let got = strip_location(&input);
            match (&got, &want) {
                (Ok(out), Ok(())) => {
                    assert!(!out.changed, "{name}");
                    assert_eq!(out.bytes, input, "{name}");
                }
                (Err(g), Err(w)) => assert_eq!(g, w, "{name}"),
                _ => panic!("{name}: got {got:?}, want {want:?}"),
            }
            let mut buf = input.clone();
            let _ = strip_location_in_place(&mut buf);
            assert_eq!(buf, input, "{name}: buffer untouched");
        }
    }

    #[test]
    fn strip_handcrafted_malformed_jpegs() {
        let base = read("images/gps_exif.jpg");
        // APP1 Exif segment starts at 20; TIFF header at 30.
        assert_eq!(&base[20..22], &[0xFF, 0xE1]);
        let tiff = 30;

        // IFD loop: IFD0's next pointer points back at IFD0 -> chain ends.
        let mut f = base.clone();
        let count = be16(&f, tiff + 8);
        let next_at = tiff + 8 + 2 + 12 * count;
        f[next_at..next_at + 4].copy_from_slice(&8u32.to_be_bytes());
        let out = strip_location(&f).unwrap();
        assert!(out.kinds.exif);

        // IFD0 offset outside the segment -> malformed.
        let mut f = base.clone();
        f[tiff + 4..tiff + 8].copy_from_slice(&0x00FF_0000u32.to_be_bytes());
        assert!(matches!(
            strip_location(&f),
            Err(StripError::Malformed {
                format: Format::Jpeg,
                ..
            })
        ));

        // GPS IFD pointer outside the segment -> pointer removed anyway.
        let mut f = base.clone();
        let gps_entry = (0..count)
            .map(|i| tiff + 8 + 2 + 12 * i)
            .find(|&e| be16(&f, e) == 0x8825)
            .unwrap();
        f[gps_entry + 8..gps_entry + 12].copy_from_slice(&0xFFFF_FFF0u32.to_be_bytes());
        let out = strip_location(&f).unwrap();
        assert!(out.kinds.exif);
        assert!(!read_exif(&out.bytes).unwrap().ifd0_tags.contains(&0x8825));

        // Zero and oversized segment lengths.
        let mut f = base.clone();
        f[22..24].copy_from_slice(&[0, 0]);
        assert!(matches!(
            strip_location(&f),
            Err(StripError::Malformed { .. })
        ));
        let mut f = base.clone();
        f[4..6].copy_from_slice(&[0xFF, 0xFF]);
        let r = strip_location(&f);
        assert!(r.is_err(), "{r:?}");
    }

    /// Small deterministic generator for the mutation test.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            if n == 0 {
                0
            } else {
                (self.next() % n as u64) as usize
            }
        }
    }

    fn mutate(rng: &mut Rng, input: &[u8]) -> Vec<u8> {
        let mut m = input.to_vec();
        // Most structure (and all fixture metadata) is near the start.
        let head = m.len().min(2048);
        match rng.below(6) {
            0 => {
                for _ in 0..1 + rng.below(8) {
                    let i = rng.below(head);
                    m[i] = rng.next() as u8;
                }
            }
            1 => {
                for _ in 0..1 + rng.below(8) {
                    let i = rng.below(m.len());
                    m[i] ^= 1 << rng.below(8);
                }
            }
            2 => m.truncate(rng.below(m.len())),
            3 => {
                let i = rng.below(head.saturating_sub(4));
                let v: [u8; 4] = match rng.below(4) {
                    0 => [0, 0, 0, 0],
                    1 => [0xFF; 4],
                    2 => [0x7F, 0xFF, 0xFF, 0xFF],
                    _ => (rng.next() as u32).to_be_bytes(),
                };
                let n = if rng.below(2) == 0 { 2 } else { 4 };
                m[i..i + n].copy_from_slice(&v[..n]);
            }
            4 => {
                let i = rng.below(head);
                let n = 1 + rng.below(16);
                let bytes: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
                m.splice(i..i, bytes);
            }
            _ => {
                let i = rng.below(head);
                let n = (1 + rng.below(16)).min(m.len() - i);
                m.drain(i..i + n);
            }
        }
        m
    }

    #[test]
    fn strip_random_mutations_never_panic_and_keep_length() {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        let names = PLAIN.iter().chain(WITH_LOCATION.iter());
        let mut ok = 0;
        let mut err = 0;
        for name in names {
            let input = read(&format!("images/{name}"));
            for iteration in 0..250 {
                let m = mutate(&mut rng, &input);
                let result = std::panic::catch_unwind(|| strip_location(&m));
                let result =
                    result.unwrap_or_else(|_| panic!("{name}: panic at iteration {iteration}"));
                let mut buf = m.clone();
                let in_place = strip_location_in_place(&mut buf);
                match result {
                    Ok(out) => {
                        ok += 1;
                        assert_eq!(out.bytes.len(), m.len(), "{name}: iteration {iteration}");
                        assert_eq!(buf, out.bytes, "{name}: iteration {iteration}: in place");
                    }
                    Err(e) => {
                        err += 1;
                        assert_eq!(in_place.unwrap_err(), e);
                        assert_eq!(buf, m, "{name}: iteration {iteration}: untouched on error");
                    }
                }
            }
        }
        println!("mutations: {ok} stripped, {err} rejected");
        assert!(ok > 0 && err > 0, "the mutations exercise both outcomes");
    }

    // ------------------------------------------------------------ device photos

    #[test]
    fn strip_device_photos_when_present() {
        let m = manifest();
        let mut waiting = Vec::new();
        for e in m["files"].as_array().unwrap() {
            if e["source"] != "device" {
                continue;
            }
            let rel = e["path"].as_str().unwrap();
            let path = fixtures().join(rel);
            if e["present"] != true || !path.exists() {
                waiting.push(rel.to_string());
                continue;
            }
            let input = std::fs::read(&path).unwrap();
            let out = strip_location(&input).unwrap_or_else(|err| panic!("{rel}: {err}"));
            assert_changes_inside_metadata(rel, &input, &out);
            assert_no_location_on_reparse(rel, &out.bytes);
            let again = strip_location(&out.bytes).unwrap();
            assert!(!again.changed, "{rel}: idempotent");
            if rel.contains("hdr") {
                assert_gain_map_unchanged(rel, &input, &out.bytes);
            }
        }
        if !waiting.is_empty() {
            println!(
                "strip device test waiting for a person to add: {}",
                waiting.join(", ")
            );
        }
    }

    /// The gain map is a secondary JPEG after the primary's EOI (Ultra HDR) or
    /// an auxiliary image item in `mdat` (iPhone HEIC). Its coded data must be
    /// unchanged.
    fn assert_gain_map_unchanged(name: &str, input: &[u8], out: &[u8]) {
        match format_of(input) {
            Format::Jpeg => {
                let primary_sos = jpeg_segments(input).last().unwrap().1;
                let eoi = primary_sos
                    + find(&input[primary_sos..], &[0xFF, 0xD8, 0xFF]).expect("secondary image");
                let second = &input[eoi..];
                let sos = eoi + jpeg_segments(second).last().unwrap().1;
                assert_eq!(&input[sos..], &out[sos..], "{name}: gain map scan data");
            }
            _ => {
                let mdat = heic_boxes(input)
                    .into_iter()
                    .find(|(t, _, _)| t == b"mdat")
                    .unwrap();
                let regions = strip_location(input).unwrap().regions;
                for i in mdat.1..mdat.2 {
                    if input[i] != out[i] {
                        assert!(
                            regions.iter().any(|r| r.range.contains(&i)),
                            "{name}: image data changed"
                        );
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ 50 MB

    #[test]
    fn strip_large_50mb_file_is_fast() {
        let path = fixtures().join("generated/large_50mb.jpg");
        let Ok(mut buf) = std::fs::read(&path) else {
            println!("strip large test waiting for fixtures/generated/large_50mb.jpg (run `just fixtures-verify`)");
            return;
        };
        let start = Instant::now();
        let report = strip_location_in_place(&mut buf).unwrap();
        let elapsed = start.elapsed();
        println!(
            "large_50mb.jpg stripped in {elapsed:?} (release: {})",
            !cfg!(debug_assertions)
        );
        assert!(!report.changed);
        let limit = if cfg!(debug_assertions) { 20.0 } else { 2.0 };
        assert!(elapsed.as_secs_f64() < limit, "{elapsed:?}");
    }
}
