//! Worker-process tests on macOS (ImageIO back end). Every image test goes
//! through a real `image-worker` child process.

#![cfg(target_os = "macos")]

mod common;

use std::path::Path;
use std::time::Instant;

use image_worker::{sniff, ImageFormat, InvalidImage, SizeBucket, Worker, WorkerError};

fn worker() -> Worker {
    Worker::new(env!("CARGO_BIN_EXE_image-worker")).expect("absolute worker path")
}

fn decode_file(w: &Worker, path: &Path, claimed: ImageFormat) -> image_worker::Facts {
    w.decode(path, claimed).expect("decode")
}

#[test]
fn plain_images_decode_with_manifest_dimensions() {
    let w = worker();
    for name in ["plain.jpg", "plain.png", "plain.heic"] {
        let e = common::entry(&format!("images/{name}"));
        let facts = decode_file(&w, &common::fixture(&e.path), e.claimed);
        assert_eq!(Some(facts.width), e.width, "{name}");
        assert_eq!(Some(facts.height), e.height, "{name}");
        assert_eq!(Some(facts.format), e.format, "{name}");
        assert_eq!(facts.orientation, 1, "{name}");
        assert!(!facts.has_hdr_gain_map, "{name}");
    }
}

#[test]
fn every_valid_committed_fixture_decodes_with_manifest_facts() {
    let w = worker();
    for e in common::manifest()
        .into_iter()
        .filter(|e| e.valid && e.committed)
    {
        let facts = decode_file(&w, &common::fixture(&e.path), e.claimed);
        assert_eq!(Some(facts.width), e.width, "{}", e.path);
        assert_eq!(Some(facts.height), e.height, "{}", e.path);
        assert_eq!(Some(facts.format), e.format, "{}", e.path);
        // PNG eXIf orientation is not read by ImageIO; JPEG must match.
        if e.format == Some(ImageFormat::Jpeg) {
            assert_eq!(facts.orientation, e.orientation.unwrap_or(1), "{}", e.path);
        }
    }
}

#[test]
fn previews_are_jpeg_with_1024_long_edge() {
    let w = worker();
    let dir = tempfile::tempdir().unwrap();
    for name in ["plain.jpg", "plain.png", "plain.heic"] {
        let e = common::entry(&format!("images/{name}"));
        let out = dir.path().join(format!("{name}.preview.jpg"));
        let job = w
            .preview(&common::fixture(&e.path), e.claimed, &out)
            .expect("preview");
        let size = job.output.expect("output size");
        assert_eq!(size.width.max(size.height), 1024, "{name}");
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(sniff(&bytes), Ok(ImageFormat::Jpeg), "{name}");
        // Read it back through the worker.
        let back = decode_file(&w, &out, ImageFormat::Jpeg);
        assert_eq!(back.width, 1024, "{name}");
        // 1800x1200 scaled to 1024 wide: 682.67 high, rounded either way.
        assert!(matches!(back.height, 682 | 683), "{name}: {}", back.height);
        assert_eq!(job.facts.width, 1800, "{name}: facts of the source");
    }
}

#[test]
fn preview_applies_orientation_to_pixels() {
    let w = worker();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("o6.jpg");
    let job = w
        .preview(
            &common::fixture("images/orientation6.jpg"),
            ImageFormat::Jpeg,
            &out,
        )
        .unwrap();
    assert_eq!(job.facts.orientation, 6);
    let back = decode_file(&w, &out, ImageFormat::Jpeg);
    // Stored 1800x1200 rotated 90 degrees: portrait.
    assert_eq!((back.width, back.height), (683, 1024));
    assert_eq!(back.orientation, 1, "no orientation tag left to apply");
}

#[test]
fn preview_never_upscales() {
    let w = worker();
    let dir = tempfile::tempdir().unwrap();
    let small = dir.path().join("small.png");
    std::fs::write(&small, tiny_png(40, 20)).unwrap();
    let out = dir.path().join("small.jpg");
    let job = w.preview(&small, ImageFormat::Png, &out).unwrap();
    assert_eq!(job.output.map(|o| (o.width, o.height)), Some((40, 20)));
    let back = decode_file(&w, &out, ImageFormat::Jpeg);
    assert_eq!((back.width, back.height), (40, 20));
}

#[test]
fn compatible_copy_of_heic_is_full_resolution_jpeg() {
    let w = worker();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("compat.jpg");
    let job = w
        .compatible(
            &common::fixture("images/plain.heic"),
            ImageFormat::Heic,
            &out,
        )
        .unwrap();
    assert_eq!(job.output.map(|o| (o.width, o.height)), Some((1800, 1200)));
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(sniff(&bytes), Ok(ImageFormat::Jpeg));
    let back = decode_file(&w, &out, ImageFormat::Jpeg);
    assert_eq!((back.width, back.height), (1800, 1200));
}

#[test]
fn compatible_copy_keeps_display_p3_profile() {
    let w = worker();
    let dir = tempfile::tempdir().unwrap();
    let src = common::fixture("images/p3.jpg");
    assert!(
        is_display_p3(&icc_profile(&std::fs::read(&src).unwrap()).expect("source ICC")),
        "fixture should carry Display P3"
    );
    let out = dir.path().join("p3.jpg");
    w.compatible(&src, ImageFormat::Jpeg, &out).unwrap();
    let bytes = std::fs::read(&out).unwrap();
    let icc = icc_profile(&bytes).expect("output has an ICC profile");
    assert!(is_display_p3(&icc), "output profile is Display P3");
}

#[test]
fn compatible_copy_keeps_orientation_tag() {
    let w = worker();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("o6.jpg");
    w.compatible(
        &common::fixture("images/orientation6.jpg"),
        ImageFormat::Jpeg,
        &out,
    )
    .unwrap();
    let back = decode_file(&w, &out, ImageFormat::Jpeg);
    assert_eq!((back.width, back.height, back.orientation), (1800, 1200, 6));
}

#[test]
fn invalid_files_fail_the_decode_check() {
    let w = worker();
    let names = [
        "truncated.jpg",
        "truncated.png",
        "truncated.heic",
        "wrong_magic.jpg",
        "wrong_magic.heic",
        "not_an_image.jpg",
        "empty.jpg",
        "heic_bad_brand.heic",
    ];
    for name in names {
        let e = common::entry(&format!("images/{name}"));
        assert!(!e.valid);
        let err = w.decode(&common::fixture(&e.path), e.claimed).unwrap_err();
        assert_eq!(
            err,
            WorkerError::InvalidImage(InvalidImage {
                format: e.claimed,
                size_bucket: SizeBucket::Under5Mb,
            }),
            "{name}"
        );
    }
}

#[test]
fn invalid_files_make_no_preview_or_compatible_output() {
    let w = worker();
    let dir = tempfile::tempdir().unwrap();
    for name in ["truncated.jpg", "wrong_magic.jpg", "empty.jpg"] {
        let e = common::entry(&format!("images/{name}"));
        let out = dir.path().join("out.jpg");
        assert!(w
            .preview(&common::fixture(&e.path), e.claimed, &out)
            .is_err());
        assert!(w
            .compatible(&common::fixture(&e.path), e.claimed, &out)
            .is_err());
        assert_dir_empty(dir.path());
    }
}

#[test]
fn large_50mb_jpeg_decodes_within_the_time_limit() {
    let path = common::fixture("generated/large_50mb.jpg");
    assert!(
        path.exists(),
        "fixtures/generated/large_50mb.jpg is missing: run `just fixtures-verify`"
    );
    let w = worker();
    let start = Instant::now();
    let facts = w.decode(&path, ImageFormat::Jpeg).expect("decode");
    assert!(start.elapsed() < w.timeout());
    assert_eq!((facts.width, facts.height), (1800, 1200));
}

#[cfg(debug_assertions)]
#[test]
fn child_runs_with_a_clean_environment() {
    std::env::set_var("IMAGE_WORKER_TEST_LEAK", "1");
    assert!(std::env::var_os("PATH").is_some());
    let names = worker().child_env_var_names().unwrap();
    // CoreFoundation sets __CF_USER_TEXT_ENCODING inside every process it
    // loads into; nothing is inherited from the parent.
    let inherited: Vec<_> = names
        .iter()
        .filter(|n| n.as_str() != "__CF_USER_TEXT_ENCODING")
        .collect();
    assert!(inherited.is_empty(), "inherited: {inherited:?}");
}

// ---- kill tests (test-only job kinds exist in debug builds) ----

#[cfg(debug_assertions)]
mod kill {
    use super::*;
    use image_worker::Task;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    fn observed() -> (Worker, Arc<Mutex<Vec<u32>>>) {
        let pids = Arc::new(Mutex::new(Vec::new()));
        let p = pids.clone();
        let w = worker().with_spawn_observer(move |pid| p.lock().unwrap().push(pid));
        (w, pids)
    }

    fn assert_gone(pids: &Mutex<Vec<u32>>) {
        let pids = pids.lock().unwrap();
        assert_eq!(pids.len(), 1);
        let status = std::process::Command::new("/bin/ps")
            .args(["-p", &pids[0].to_string()])
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(!status.success(), "child {} still running", pids[0]);
    }

    fn expected() -> WorkerError {
        WorkerError::InvalidImage(InvalidImage {
            format: ImageFormat::Jpeg,
            size_bucket: SizeBucket::Under5Mb,
        })
    }

    #[test]
    fn hanging_job_is_killed_at_the_timeout() {
        let (w, pids) = observed();
        let w = w.with_timeout(Duration::from_millis(500));
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.jpg");
        let start = Instant::now();
        let err = w
            .run(
                Task::TestHang,
                &common::fixture("images/plain.jpg"),
                ImageFormat::Jpeg,
                Some(&out),
            )
            .unwrap_err();
        let took = start.elapsed();
        assert_eq!(err, expected());
        assert!(took >= Duration::from_millis(500), "{took:?}");
        assert!(took < Duration::from_secs(5), "{took:?}");
        assert_gone(&pids);
        assert_dir_empty(dir.path());
    }

    #[test]
    fn crashing_job_is_reported_the_same_way() {
        let (w, pids) = observed();
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.jpg");
        let start = Instant::now();
        let err = w
            .run(
                Task::TestAbort,
                &common::fixture("images/plain.jpg"),
                ImageFormat::Jpeg,
                Some(&out),
            )
            .unwrap_err();
        assert_eq!(err, expected());
        assert!(start.elapsed() < w.timeout());
        assert_gone(&pids);
        assert_dir_empty(dir.path());
    }

    #[test]
    fn job_over_the_memory_limit_is_killed() {
        let (w, pids) = observed();
        let w = w.with_memory_limit(200 << 20);
        let start = Instant::now();
        let err = w
            .run(
                Task::TestAlloc(800 << 20),
                &common::fixture("images/plain.jpg"),
                ImageFormat::Jpeg,
                None,
            )
            .unwrap_err();
        assert_eq!(err, expected());
        // Killed for memory, well before the 10 s time limit.
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "{:?}",
            start.elapsed()
        );
        assert_gone(&pids);
    }

    #[test]
    fn job_under_the_memory_limit_is_not_killed_for_memory() {
        // Same allocation with the default 1 GiB limit: it then hangs until
        // the (short) timeout, which shows the memory check did not fire.
        let w = worker().with_timeout(Duration::from_millis(1500));
        let start = Instant::now();
        let err = w
            .run(
                Task::TestAlloc(300 << 20),
                &common::fixture("images/plain.jpg"),
                ImageFormat::Jpeg,
                None,
            )
            .unwrap_err();
        assert_eq!(err, expected());
        assert!(start.elapsed() >= Duration::from_millis(1500));
    }

    #[test]
    fn malformed_result_line_and_non_zero_exit_are_reported_the_same_way() {
        for task in [Task::TestGarbage, Task::TestExitNonZero] {
            let (w, pids) = observed();
            let err = w
                .run(
                    task,
                    &common::fixture("images/plain.jpg"),
                    ImageFormat::Jpeg,
                    None,
                )
                .unwrap_err();
            assert_eq!(err, expected(), "{task:?}");
            assert_gone(&pids);
        }
    }
}

// ---- helpers ----

fn assert_dir_empty(dir: &Path) {
    let left: Vec<_> = std::fs::read_dir(dir).unwrap().collect();
    assert!(left.is_empty(), "files left behind: {}", left.len());
}

/// The ICC profile embedded in a JPEG's APP2 `ICC_PROFILE` segments.
fn icc_profile(jpeg: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 2;
    let mut profile = Vec::new();
    while pos + 4 <= jpeg.len() && jpeg[pos] == 0xFF {
        let marker = jpeg[pos + 1];
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        let len = usize::from(u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]));
        let body = &jpeg[pos + 4..pos + 2 + len];
        if marker == 0xE2 && body.starts_with(b"ICC_PROFILE\0") {
            profile.extend_from_slice(&body[14..]);
        }
        pos += 2 + len;
    }
    (!profile.is_empty()).then_some(profile)
}

/// True if the profile's description names Display P3 (ASCII `desc` or
/// UTF-16BE `mluc`).
fn is_display_p3(icc: &[u8]) -> bool {
    let ascii = b"Display P3";
    let utf16: Vec<u8> = "Display P3"
        .encode_utf16()
        .flat_map(|u| u.to_be_bytes())
        .collect();
    icc.windows(ascii.len()).any(|w| w == ascii) || icc.windows(utf16.len()).any(|w| w == utf16)
}

/// A minimal valid RGB PNG (stored deflate blocks).
fn tiny_png(width: u32, height: u32) -> Vec<u8> {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in bytes {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    }
    let mut raw = Vec::new();
    for y in 0..height {
        raw.push(0);
        for x in 0..width {
            raw.extend_from_slice(&[(x * 6) as u8, (y * 12) as u8, 128]);
        }
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, block) in blocks.iter().enumerate() {
        z.push(u8::from(i + 1 == blocks.len()));
        let len = block.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    png
}
