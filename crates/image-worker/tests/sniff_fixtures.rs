//! The pure-Rust sniffer against every committed fixture image. Runs on every
//! platform.

mod common;

use image_worker::{sniff, ImageFormat, NotAnImage};

#[test]
fn every_fixture_image_is_classified_by_content() {
    let images = std::fs::read_dir(common::fixture("images")).unwrap();
    let mut seen = 0;
    for file in images {
        let file = file.unwrap();
        let name = file.file_name().into_string().unwrap();
        let entry = common::entry(&format!("images/{name}"));
        let bytes = std::fs::read(file.path()).unwrap();
        let got = sniff(&bytes).ok();
        assert_eq!(got, entry.format, "{name}");
        seen += 1;
    }
    assert_eq!(seen, 20, "fixtures/images changed; update this test");
}

#[test]
fn named_cases() {
    let read = |n: &str| std::fs::read(common::fixture(&format!("images/{n}"))).unwrap();
    // Classified by content, not by name.
    assert_eq!(sniff(&read("wrong_magic.jpg")), Ok(ImageFormat::Png));
    assert_eq!(sniff(&read("wrong_magic.heic")), Ok(ImageFormat::Jpeg));
    // Rejected.
    assert_eq!(sniff(&read("heic_bad_brand.heic")), Err(NotAnImage));
    assert_eq!(sniff(&read("not_an_image.jpg")), Err(NotAnImage));
    assert_eq!(sniff(&read("empty.jpg")), Err(NotAnImage));
    // Accepted.
    assert_eq!(sniff(&read("plain.jpg")), Ok(ImageFormat::Jpeg));
    assert_eq!(sniff(&read("plain.png")), Ok(ImageFormat::Png));
    assert_eq!(sniff(&read("plain.heic")), Ok(ImageFormat::Heic));
}
