//! macOS only: decodes each fixture before and after the strip with the
//! system's `sips` (to uncompressed BMP in a temp directory) and compares the
//! pixels and the reported orientation and colour profile.

#[cfg(target_os = "macos")]
mod strip {
    use std::path::Path;
    use std::process::Command;

    use image_worker::strip::strip_location;

    const FILES: [&str; 12] = [
        "plain.jpg",
        "plain.png",
        "plain.heic",
        "orientation6.jpg",
        "p3.jpg",
        "gps_exif.jpg",
        "gps_xmp.jpg",
        "gps_iptc.jpg",
        "gps_all.jpg",
        "gps_exif.png",
        "gps_xmp.png",
        "gps_exif.heic",
    ];

    const PROPERTIES: [&str; 8] = [
        "pixelWidth",
        "pixelHeight",
        "orientation",
        "profile",
        "space",
        "hasAlpha",
        "bitsPerSample",
        "format",
    ];

    fn sips(args: &[&str]) -> String {
        let out = Command::new("/usr/bin/sips")
            .args(args)
            .output()
            .expect("sips runs");
        assert!(
            out.status.success(),
            "sips {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// `sips -g` output without the first line (the file path).
    fn properties(path: &Path) -> String {
        let mut args = Vec::new();
        for p in PROPERTIES {
            args.push("-g");
            args.push(p);
        }
        let p = path.to_str().unwrap();
        args.push(p);
        sips(&args).lines().skip(1).collect::<Vec<_>>().join("\n")
    }

    fn pixels(path: &Path, dir: &Path, tag: &str) -> Vec<u8> {
        let bmp = dir.join(format!("{tag}.bmp"));
        sips(&[
            "-s",
            "format",
            "bmp",
            path.to_str().unwrap(),
            "--out",
            bmp.to_str().unwrap(),
        ]);
        std::fs::read(bmp).unwrap()
    }

    #[test]
    fn strip_output_decodes_identically_with_sips() {
        let dir = tempfile::tempdir().unwrap();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/images");
        for name in FILES {
            let input_path = fixtures.join(name);
            let input = std::fs::read(&input_path).unwrap();
            let out = strip_location(&input).unwrap();
            let ext = name.rsplit('.').next().unwrap();
            let out_path = dir.path().join(format!("stripped.{ext}"));
            std::fs::write(&out_path, &out.bytes).unwrap();

            assert_eq!(
                properties(&input_path),
                properties(&out_path),
                "{name}: orientation, profile, size"
            );
            let before = pixels(&input_path, dir.path(), "before");
            let after = pixels(&out_path, dir.path(), "after");
            assert!(before.len() > 1000, "{name}: decoded");
            assert!(before == after, "{name}: decoded pixels differ");
        }
    }
}
