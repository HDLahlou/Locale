//! XMP location strip: a small namespace-aware tag scanner (no XML library)
//! that finds location properties and replaces their text with spaces, so the
//! packet keeps its length and stays well-formed.

use std::ops::Range;

use super::MetaError;

const NS_EXIF: &[u8] = b"http://ns.adobe.com/exif/1.0/";
const NS_PHOTOSHOP: &[u8] = b"http://ns.adobe.com/photoshop/1.0/";
const NS_IPTC_CORE: &[u8] = b"http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/";
const NS_IPTC_EXT: &[u8] = b"http://iptc.org/std/Iptc4xmpExt/2008-02-29/";

/// Prefixes assumed when a packet uses one without declaring it, so a sloppy
/// packet that a lenient reader would still understand is stripped too.
const USUAL_PREFIXES: &[(&[u8], &[u8])] = &[
    (b"exif", NS_EXIF),
    (b"photoshop", NS_PHOTOSHOP),
    (b"Iptc4xmpCore", NS_IPTC_CORE),
    (b"Iptc4xmpExt", NS_IPTC_EXT),
];

/// Most open elements; deeper nesting is treated as malformed.
const MAX_DEPTH: usize = 256;

/// Removes location properties from an XMP packet (or an extended-XMP
/// serialisation). Returns the modified packet when any were found.
pub(super) fn strip(buf: &[u8]) -> Result<Option<Vec<u8>>, MetaError> {
    let blanks = find_location(buf)?;
    if blanks.is_empty() {
        return Ok(None);
    }
    let mut out = buf.to_vec();
    for r in blanks {
        out[r].fill(b' ');
    }
    Ok(Some(out))
}

fn is_location(uri: &[u8], local: &[u8]) -> bool {
    match uri {
        NS_EXIF => local.starts_with(b"GPS"),
        NS_PHOTOSHOP => matches!(local, b"City" | b"State" | b"Country"),
        NS_IPTC_CORE => matches!(local, b"Location" | b"CountryCode"),
        NS_IPTC_EXT => matches!(local, b"LocationCreated" | b"LocationShown"),
        _ => false,
    }
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn find(text: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if from > text.len() {
        return None;
    }
    text[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn skip_ws(text: &[u8], mut i: usize) -> usize {
    while text.get(i).is_some_and(|&b| is_ws(b)) {
        i += 1;
    }
    i
}

fn read_name(text: &[u8], start: usize) -> Result<(Range<usize>, usize), MetaError> {
    let mut i = start;
    while let Some(&b) = text.get(i) {
        if is_ws(b) || matches!(b, b'/' | b'>' | b'=' | b'<') {
            break;
        }
        i += 1;
    }
    if i == start {
        return Err(MetaError::Malformed("XMP name expected"));
    }
    Ok((start..i, i))
}

struct Attr {
    name: Range<usize>,
    /// From the first byte of the name to the closing quote.
    whole: Range<usize>,
    value: Range<usize>,
}

/// Finds the byte ranges to blank: whole location elements and location
/// attributes.
fn find_location(buf: &[u8]) -> Result<Vec<Range<usize>>, MetaError> {
    // Trailing NUL padding is allowed; NUL elsewhere means UTF-16 or UTF-32,
    // which this byte scanner cannot read, so fail closed.
    let end = buf.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let text = &buf[..end];
    if text.starts_with(&[0xFE, 0xFF]) || text.starts_with(&[0xFF, 0xFE]) || text.contains(&0) {
        return Err(MetaError::Unstrippable("XMP packet is not UTF-8"));
    }

    let mut ns: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut stack: Vec<(Range<usize>, usize)> = Vec::new();
    let mut removing: Option<(usize, usize)> = None;
    let mut blanks = Vec::new();
    let mut pos = 0;

    while let Some(lt) = find(text, pos, b"<") {
        let rest = &text[lt..];
        if rest.starts_with(b"<!--") {
            pos = find(text, lt + 4, b"-->")
                .ok_or(MetaError::Malformed("XMP comment not closed"))?
                + 3;
            continue;
        }
        if rest.starts_with(b"<![CDATA[") {
            pos =
                find(text, lt + 9, b"]]>").ok_or(MetaError::Malformed("XMP CDATA not closed"))? + 3;
            continue;
        }
        if rest.starts_with(b"<?") {
            pos = find(text, lt + 2, b"?>").ok_or(MetaError::Malformed(
                "XMP processing instruction not closed",
            ))? + 2;
            continue;
        }
        if rest.starts_with(b"<!") {
            // A DOCTYPE could declare entities; XMP forbids it.
            return Err(MetaError::Unstrippable("XMP packet has a DOCTYPE"));
        }
        if rest.starts_with(b"</") {
            let (name, i) = read_name(text, lt + 2)?;
            let i = skip_ws(text, i);
            if text.get(i) != Some(&b'>') {
                return Err(MetaError::Malformed("XMP end tag not closed"));
            }
            let (open, ns_len) = stack
                .pop()
                .ok_or(MetaError::Malformed("XMP end tag without start tag"))?;
            if text[open] != text[name] {
                return Err(MetaError::Malformed("XMP tags do not match"));
            }
            ns.truncate(ns_len);
            if let Some((start, depth)) = removing {
                if stack.len() == depth {
                    blanks.push(start..i + 1);
                    removing = None;
                }
            }
            pos = i + 1;
            continue;
        }

        // Start tag.
        let (name, mut i) = read_name(text, lt + 1)?;
        let mut attrs = Vec::new();
        let empty = loop {
            let j = skip_ws(text, i);
            match text.get(j) {
                None => return Err(MetaError::Malformed("XMP start tag not closed")),
                Some(b'>') => {
                    i = j + 1;
                    break false;
                }
                Some(b'/') => {
                    if text.get(j + 1) != Some(&b'>') {
                        return Err(MetaError::Malformed("XMP start tag not closed"));
                    }
                    i = j + 2;
                    break true;
                }
                Some(_) => {
                    if j == i {
                        return Err(MetaError::Malformed("XMP attribute without white space"));
                    }
                    let (an, k) = read_name(text, j)?;
                    let k = skip_ws(text, k);
                    if text.get(k) != Some(&b'=') {
                        return Err(MetaError::Malformed("XMP attribute without value"));
                    }
                    let k = skip_ws(text, k + 1);
                    let q = match text.get(k) {
                        Some(&q @ (b'"' | b'\'')) => q,
                        _ => return Err(MetaError::Malformed("XMP attribute value not quoted")),
                    };
                    let close = find(text, k + 1, &[q])
                        .ok_or(MetaError::Malformed("XMP attribute value not closed"))?;
                    attrs.push(Attr {
                        name: an,
                        whole: j..close + 1,
                        value: k + 1..close,
                    });
                    i = close + 1;
                }
            }
        };

        let ns_len = ns.len();
        for a in &attrs {
            let n = &text[a.name.clone()];
            if n == b"xmlns" {
                ns.push((Vec::new(), decode_entities(&text[a.value.clone()])));
            } else if let Some(p) = n.strip_prefix(b"xmlns:") {
                ns.push((p.to_vec(), decode_entities(&text[a.value.clone()])));
            }
        }

        if removing.is_none() {
            if matches_location(&ns, &text[name.clone()], true) {
                if empty {
                    blanks.push(lt..i);
                } else {
                    removing = Some((lt, stack.len()));
                }
            } else {
                for a in &attrs {
                    let n = &text[a.name.clone()];
                    if n.starts_with(b"xmlns") {
                        continue;
                    }
                    if matches_location(&ns, n, false) {
                        blanks.push(a.whole.clone());
                    }
                }
            }
        }

        if empty {
            ns.truncate(ns_len);
        } else {
            stack.push((name, ns_len));
            if stack.len() > MAX_DEPTH {
                return Err(MetaError::Malformed("XMP nesting too deep"));
            }
        }
        pos = i;
    }

    if !stack.is_empty() {
        return Err(MetaError::Malformed("XMP element not closed"));
    }
    Ok(blanks)
}

/// Resolves a qualified name and checks it against the location list.
fn matches_location(ns: &[(Vec<u8>, Vec<u8>)], qname: &[u8], element: bool) -> bool {
    let (prefix, local): (&[u8], &[u8]) = match qname.iter().position(|&b| b == b':') {
        Some(c) => (&qname[..c], &qname[c + 1..]),
        // Unprefixed attributes have no namespace; elements take the default.
        None if element => (b"", qname),
        None => return false,
    };
    let uri = ns
        .iter()
        .rev()
        .find(|(p, _)| p.as_slice() == prefix)
        .map(|(_, u)| u.as_slice());
    let uri = match uri {
        Some(u) => u,
        None => match USUAL_PREFIXES.iter().find(|(p, _)| *p == prefix) {
            Some((_, u)) => u,
            None => return false,
        },
    };
    is_location(uri, local)
}

/// Decodes the five predefined entities and character references in a
/// namespace URI. Unknown references are kept as written.
fn decode_entities(v: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len());
    let mut i = 0;
    while i < v.len() {
        if v[i] == b'&' {
            if let Some(semi) = v[i..].iter().position(|&b| b == b';') {
                let ent = &v[i + 1..i + semi];
                let decoded: Option<u32> = match ent {
                    b"amp" => Some(u32::from(b'&')),
                    b"lt" => Some(u32::from(b'<')),
                    b"gt" => Some(u32::from(b'>')),
                    b"quot" => Some(u32::from(b'"')),
                    b"apos" => Some(u32::from(b'\'')),
                    _ if ent.starts_with(b"#x") => std::str::from_utf8(&ent[2..])
                        .ok()
                        .and_then(|s| u32::from_str_radix(s, 16).ok()),
                    _ if ent.starts_with(b"#") => std::str::from_utf8(&ent[1..])
                        .ok()
                        .and_then(|s| s.parse().ok()),
                    _ => None,
                };
                if let Some(c) = decoded.and_then(char::from_u32) {
                    let mut tmp = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut tmp).as_bytes());
                    i += semi + 1;
                    continue;
                }
            }
        }
        out.push(v[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(s: &str) -> String {
        let out = strip(s.as_bytes())
            .unwrap()
            .unwrap_or_else(|| s.as_bytes().to_vec());
        assert_eq!(out.len(), s.len());
        String::from_utf8(out).unwrap()
    }

    const HEAD: &str = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?><x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">"#;
    const TAIL: &str = r#"</rdf:RDF></x:xmpmeta><?xpacket end="w"?>"#;

    fn packet(body: &str) -> String {
        format!("{HEAD}{body}{TAIL}")
    }

    #[test]
    fn attribute_properties_are_blanked() {
        let p = packet(
            r#"<rdf:Description rdf:about="" xmlns:exif="http://ns.adobe.com/exif/1.0/" xmlns:tiff="http://ns.adobe.com/tiff/1.0/" exif:GPSLatitude="12,20.736N" exif:GPSLongitude='98,45.924E' exif:DateTimeOriginal="2026-01-02T03:04:05" tiff:Orientation="6"/>"#,
        );
        let out = run(&p);
        assert!(!out.contains("GPS"));
        assert!(!out.contains("12,20.736N"));
        assert!(out.contains(r#"exif:DateTimeOriginal="2026-01-02T03:04:05""#));
        assert!(out.contains(r#"tiff:Orientation="6""#));
        assert_eq!(strip(out.as_bytes()).unwrap(), None, "idempotent");
    }

    #[test]
    fn element_properties_and_structures_are_removed() {
        let p = packet(concat!(
            r#"<rdf:Description rdf:about="" xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/" "#,
            r#"xmlns:Iptc4xmpCore="http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/" "#,
            r#"xmlns:Iptc4xmpExt="http://iptc.org/std/Iptc4xmpExt/2008-02-29/">"#,
            r#"<photoshop:City>Testville</photoshop:City>"#,
            r#"<photoshop:State>Teststate</photoshop:State>"#,
            r#"<photoshop:Country>Testland</photoshop:Country>"#,
            r#"<photoshop:DateCreated>2026-01-02T03:04:05</photoshop:DateCreated>"#,
            r#"<Iptc4xmpCore:Location>Test Quarter</Iptc4xmpCore:Location>"#,
            r#"<Iptc4xmpCore:CountryCode>TL</Iptc4xmpCore:CountryCode>"#,
            r#"<Iptc4xmpCore:IntellectualGenre>keep</Iptc4xmpCore:IntellectualGenre>"#,
            r#"<Iptc4xmpExt:LocationShown><rdf:Bag><rdf:li rdf:parseType="Resource">"#,
            r#"<Iptc4xmpExt:City>Testville</Iptc4xmpExt:City></rdf:li></rdf:Bag></Iptc4xmpExt:LocationShown>"#,
            r#"<Iptc4xmpExt:LocationCreated rdf:parseType="Resource"><Iptc4xmpExt:Sublocation>Test Quarter</Iptc4xmpExt:Sublocation></Iptc4xmpExt:LocationCreated>"#,
            r#"<Iptc4xmpExt:LocationCreated/>"#,
            r#"</rdf:Description>"#,
        ));
        let out = run(&p);
        for gone in [
            "Testville",
            "Teststate",
            "Testland",
            "Test Quarter",
            "TL<",
            "Location",
        ] {
            assert!(!out.contains(gone), "{gone} left in {out}");
        }
        assert!(out.contains("<photoshop:DateCreated>2026-01-02T03:04:05</photoshop:DateCreated>"));
        assert!(out.contains("IntellectualGenre>keep<"));
        assert_eq!(strip(out.as_bytes()).unwrap(), None);
    }

    #[test]
    fn namespaces_are_matched_by_uri_not_prefix() {
        let p = packet(
            r#"<rdf:Description xmlns:e="http://ns.adobe.com/exif/1.0/" xmlns:exif="http://example.com/other/" e:GPSAltitude="10/1" exif:GPSLatitude="kept"><e:GPSLongitude>98,45.924E</e:GPSLongitude></rdf:Description>"#,
        );
        let out = run(&p);
        assert!(!out.contains("10/1"));
        assert!(!out.contains("98,45.924E"));
        assert!(out.contains(r#"exif:GPSLatitude="kept""#));
    }

    #[test]
    fn default_namespace_and_entity_encoded_uri() {
        let p = packet(
            r#"<rdf:Description><GPSLatitude xmlns="http&#58;//ns.adobe.com/exif/1.0/">1,2N</GPSLatitude></rdf:Description>"#,
        );
        assert!(!run(&p).contains("1,2N"));
    }

    #[test]
    fn undeclared_usual_prefix_is_still_stripped() {
        let p = packet(r#"<rdf:Description exif:GPSLatitude="12,20.736N"/>"#);
        assert!(!run(&p).contains("12,20.736N"));
    }

    #[test]
    fn comments_cdata_and_gt_in_values_are_skipped() {
        let p = packet(concat!(
            r#"<!-- <exif:GPSLatitude> --><rdf:Description xmlns:exif="http://ns.adobe.com/exif/1.0/" a="x>y">"#,
            r#"<exif:UserComment><![CDATA[</exif:UserComment>]]></exif:UserComment>"#,
            r#"<exif:GPSLatitude>12,20.736N</exif:GPSLatitude></rdf:Description>"#,
        ));
        let out = run(&p);
        assert!(!out.contains("12,20.736N"));
        assert!(out.contains("<![CDATA[</exif:UserComment>]]>"));
    }

    #[test]
    fn packets_without_location_are_untouched() {
        let p = packet(
            r#"<rdf:Description xmlns:tiff="http://ns.adobe.com/tiff/1.0/" tiff:Orientation="6"/>"#,
        );
        assert_eq!(strip(p.as_bytes()).unwrap(), None);
        assert_eq!(strip(b"").unwrap(), None);
        assert_eq!(strip(b"   \0\0\0").unwrap(), None);
    }

    #[test]
    fn broken_packets_fail_closed() {
        for bad in [
            "<a><b></a>",
            "<a>",
            "</a>",
            "<a b='1></a>",
            "<a b></a>",
            "<a b=1></a>",
            "<!-- open",
            "<a/ >",
            r#"<a x="1"y="2"/>"#,
            "<!DOCTYPE x [<!ENTITY e 'v'>]><a/>",
        ] {
            assert!(strip(bad.as_bytes()).is_err(), "{bad}");
        }
        let utf16: Vec<u8> = "<a/>".bytes().flat_map(|b| [b, 0]).collect();
        assert_eq!(
            strip(&utf16),
            Err(MetaError::Unstrippable("XMP packet is not UTF-8"))
        );
        let deep = "<a>".repeat(MAX_DEPTH + 1);
        assert!(strip(deep.as_bytes()).is_err());
    }
}
