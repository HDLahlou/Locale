"""Tests for the fixture set and its verifier. Every mutation works on a temporary copy; the large files are never created."""

import contextlib
import io
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from scripts import fixtures_verify as fv
from scripts import make_fixtures as mf

REAL = fv.DEFAULT_ROOT


def manifest_entries():
    return {e["path"]: e for e in json.loads((REAL / "manifest.json").read_text())["files"]}


class TempFixtures(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name) / "fixtures"
        shutil.copytree(REAL, self.root, ignore=shutil.ignore_patterns("generated"))

    def verify(self):
        return fv.verify(self.root, generate=False)

    def edit_manifest(self, path, **changes):
        m = json.loads((self.root / "manifest.json").read_text())
        for e in m["files"]:
            if e["path"] == path:
                e.update(changes)
        (self.root / "manifest.json").write_text(json.dumps(m))


class VerifierTests(TempFixtures):
    def test_committed_fixtures_verify_clean(self):
        errors, notes = self.verify()
        self.assertEqual(errors, [])
        self.assertTrue(any("waiting for a person" in n for n in notes))

    def test_fails_when_bytes_change(self):
        p = self.root / "images" / "plain.png"
        p.write_bytes(p.read_bytes() + b"x")
        errors, _ = self.verify()
        self.assertTrue(any("plain.png" in e and "sha256" in e for e in errors), errors)

    def test_fails_when_same_size_bytes_change(self):
        p = self.root / "images" / "plain.png"
        data = bytearray(p.read_bytes())
        data[-20] ^= 0xFF
        p.write_bytes(bytes(data))
        errors, _ = self.verify()
        self.assertTrue(any("plain.png" in e and "sha256" in e for e in errors), errors)

    def test_fails_when_file_missing(self):
        (self.root / "images" / "p3.jpg").unlink()
        errors, _ = self.verify()
        self.assertTrue(any("p3.jpg" in e and "missing" in e for e in errors), errors)

    def test_fails_when_unlisted_file_appears(self):
        (self.root / "images" / "surprise.jpg").write_bytes(b"x")
        errors, _ = self.verify()
        self.assertTrue(any("surprise.jpg" in e and "not in the manifest" in e for e in errors), errors)

    def test_fails_when_location_claim_is_false(self):
        # claim location where there is none
        self.edit_manifest("images/plain.jpg", has_location=True, location_kinds=["exif"])
        errors, _ = self.verify()
        self.assertTrue(any("plain.jpg" in e and "location claim" in e for e in errors), errors)

    def test_fails_when_location_claim_is_missing(self):
        # deny location where there is some
        self.edit_manifest("images/gps_all.jpg", has_location=True, location_kinds=["exif", "xmp"])
        errors, _ = self.verify()
        self.assertTrue(any("gps_all.jpg" in e and "location claim" in e for e in errors), errors)

    def test_fails_when_heic_location_claim_is_false(self):
        self.edit_manifest("images/gps_exif.heic", has_location=False, location_kinds=[])
        errors, _ = self.verify()
        self.assertTrue(any("gps_exif.heic" in e for e in errors), errors)

    def test_location_is_detected_from_bytes_not_file_name(self):
        # a file named gps_* with its metadata removed no longer passes
        p = self.root / "images" / "gps_exif.jpg"
        p.write_bytes(mf.jpeg_strip_meta(p.read_bytes()))
        errors, _ = self.verify()
        self.assertTrue(any("gps_exif.jpg" in e and "location claim" in e for e in errors), errors)

    def test_orientation_claim_is_checked(self):
        self.edit_manifest("images/orientation6.jpg", orientation=1)
        errors, _ = self.verify()
        self.assertTrue(any("orientation6.jpg" in e and "orientation" in e for e in errors), errors)

    def test_absent_device_files_do_not_fail(self):
        errors, notes = self.verify()
        self.assertEqual(errors, [])
        self.assertEqual(len([n for n in notes if "waiting for a person" in n]), 4)

    def test_device_file_present_but_unrecorded_fails_then_add_device_records_it(self):
        dev = self.root / "device" / "android.heic"
        dev.write_bytes((REAL / "images" / "plain.heic").read_bytes())
        errors, _ = self.verify()
        self.assertTrue(any("android.heic" in e for e in errors), errors)
        self.assertEqual(fv.add_device(self.root), ["device/android.heic"])
        errors, notes = self.verify()
        self.assertEqual(errors, [])
        self.assertEqual(len([n for n in notes if "waiting for a person" in n]), 3)
        entry = {e["path"]: e for e in json.loads((self.root / "manifest.json").read_text())["files"]}["device/android.heic"]
        self.assertTrue(entry["present"])
        self.assertEqual(entry["sha256"], fv.sha256(dev.read_bytes()))
        self.assertEqual(entry["size_bytes"], dev.stat().st_size)

    def test_unlisted_device_file_fails(self):
        (self.root / "device" / "other.jpg").write_bytes(b"x")
        errors, _ = self.verify()
        self.assertTrue(any("other.jpg" in e for e in errors), errors)

    def test_main_exits_nonzero_on_mismatch(self):
        (self.root / "images" / "p3.jpg").unlink()
        # Capture the verifier's report: printed into `just check` output, its
        # "FAIL: images/p3.jpg" line was read as a real failure by people and agents.
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = fv.main(["--root", str(self.root)])
        self.assertEqual(code, 1)
        self.assertIn("p3.jpg", out.getvalue() + err.getvalue())


class FixtureSetTests(unittest.TestCase):
    """The committed files themselves, read back with the verifier's own parsers."""

    @classmethod
    def setUpClass(cls):
        cls.entries = manifest_entries()
        cls.dir = REAL / "images"

    def facts(self, name):
        return fv.inspect((self.dir / name).read_bytes())

    def test_every_image_is_listed_and_small(self):
        on_disk = {f"images/{p.name}" for p in self.dir.iterdir() if p.is_file()}
        self.assertEqual(on_disk, {p for p in self.entries if p.startswith("images/")})
        for p in self.dir.iterdir():
            self.assertLess(p.stat().st_size, 400 * 1024, p.name)

    def test_expected_file_names(self):
        names = {p.split("/", 1)[1] for p in self.entries if p.startswith("images/")}
        expected = """plain.jpg plain.png plain.heic orientation6.jpg p3.jpg gps_exif.jpg gps_xmp.jpg gps_iptc.jpg
            gps_all.jpg gps_exif.png gps_xmp.png gps_exif.heic truncated.jpg truncated.png truncated.heic
            wrong_magic.jpg wrong_magic.heic not_an_image.jpg empty.jpg heic_bad_brand.heic""".split()
        self.assertEqual(names, set(expected))

    def test_plain_files_are_large_enough_and_have_no_location(self):
        for name, fmt in (("plain.jpg", "jpeg"), ("plain.png", "png"), ("plain.heic", "heic")):
            f = self.facts(name)
            self.assertEqual(f["format"], fmt)
            self.assertGreaterEqual(max(f["width"], f["height"]), 1600, name)
            self.assertEqual(f["location_kinds"], [], name)

    def test_plain_heic_is_a_heic_brand(self):
        self.assertIn(fv.heic_brand((self.dir / "plain.heic").read_bytes()), fv.HEIC_BRANDS)

    def test_orientation6(self):
        self.assertEqual(self.facts("orientation6.jpg")["orientation"], 6)
        self.assertEqual(self.facts("orientation6.jpg")["location_kinds"], [])

    def test_p3_profile(self):
        self.assertIn("Display P3".encode("utf-16-be"), (self.dir / "p3.jpg").read_bytes())
        self.assertNotIn("Display P3".encode("utf-16-be"), (self.dir / "plain.jpg").read_bytes())

    def test_location_kinds_per_file(self):
        want = {
            "gps_exif.jpg": ["exif"],
            "gps_xmp.jpg": ["xmp"],
            "gps_iptc.jpg": ["iptc"],
            "gps_all.jpg": ["exif", "iptc", "xmp"],
            "gps_exif.png": ["exif"],
            "gps_xmp.png": ["xmp"],
            "gps_exif.heic": ["exif"],
        }
        for name, kinds in want.items():
            self.assertEqual(sorted(self.facts(name)["location_kinds"]), kinds, name)

    def test_gps_jpegs_keep_orientation_and_capture_time(self):
        for name in ("gps_exif.jpg", "gps_xmp.jpg", "gps_iptc.jpg", "gps_all.jpg"):
            f = self.facts(name)
            self.assertEqual(f["orientation"], 6, name)
            self.assertEqual(f["capture_time"], mf.CAPTURE_TIME, name)

    def test_synthetic_location_strings_are_in_the_bytes_and_manifest(self):
        loc = json.loads((REAL / "manifest.json").read_text())["location"]
        self.assertEqual(loc, mf.LOCATION)
        for name in ("gps_iptc.jpg", "gps_all.jpg"):
            data = (self.dir / name).read_bytes()
            for key in ("iptc_city", "iptc_sub_location", "iptc_country"):
                self.assertIn(loc[key].encode(), data, (name, key))
        for name in ("gps_xmp.jpg", "gps_all.jpg", "gps_xmp.png"):
            data = (self.dir / name).read_bytes()
            self.assertIn(loc["xmp_latitude"].encode(), data)
            self.assertIn(loc["xmp_longitude"].encode(), data)
        for name in ("plain.jpg", "plain.png", "plain.heic", "orientation6.jpg", "p3.jpg"):
            data = (self.dir / name).read_bytes()
            self.assertNotIn(b"Testville", data)
            self.assertNotIn(b"exif:GPS", data)

    def test_truncated_files_are_sixty_percent(self):
        for ext in ("jpg", "png", "heic"):
            whole = (self.dir / f"plain.{ext}").read_bytes()
            cut = (self.dir / f"truncated.{ext}").read_bytes()
            self.assertEqual(cut, whole[: len(whole) * 60 // 100])
            self.assertFalse(self.entries[f"images/truncated.{ext}"]["valid"])

    def test_broken_files(self):
        self.assertEqual(self.facts("wrong_magic.jpg")["format"], "png")
        self.assertEqual(self.facts("wrong_magic.heic")["format"], "jpeg")
        self.assertEqual(self.facts("not_an_image.jpg")["format"], "none")
        self.assertEqual((self.dir / "empty.jpg").stat().st_size, 0)
        data = (self.dir / "heic_bad_brand.heic").read_bytes()
        self.assertEqual(data[4:8], b"ftyp")
        self.assertEqual(fv.heic_brand(data), b"mp42")
        self.assertEqual(self.facts("heic_bad_brand.heic")["format"], "none")
        for name in ("wrong_magic.jpg", "wrong_magic.heic", "not_an_image.jpg", "empty.jpg", "heic_bad_brand.heic"):
            self.assertFalse(self.entries[f"images/{name}"]["valid"], name)

    def test_manifest_fields(self):
        needed = {
            "path", "format", "size_bytes", "sha256", "valid", "has_location", "location_kinds",
            "orientation", "width", "height", "committed", "purpose",
        }  # fmt: skip
        for path, e in self.entries.items():
            self.assertLessEqual(needed, set(e), path)
            self.assertIn(e["format"], ("jpeg", "png", "heic", "none"))
        self.assertFalse(self.entries["generated/large_50mb.jpg"]["committed"])
        self.assertFalse(self.entries["generated/over_limit.jpg"]["committed"])
        self.assertEqual(self.entries["generated/large_50mb.jpg"]["size_bytes"], 52_428_800)
        self.assertEqual(self.entries["generated/over_limit.jpg"]["size_bytes"], 52_428_801)
        for name in fv.DEVICE_FILES:
            e = self.entries[f"device/{name}"]
            self.assertEqual(e["source"], "device")
            self.assertIn(e["present"], (True, False))

    def test_device_readme_names_the_files_and_the_command(self):
        text = (REAL / "device" / "README.md").read_text()
        for name in fv.DEVICE_FILES:
            self.assertIn(name, text)
        self.assertIn("python3 -I scripts/fixtures_verify.py --add-device", text)


class PaddingTests(unittest.TestCase):
    """The padding routine behind --large, exercised on small totals so no large file is made."""

    def setUp(self):
        self.plain = (REAL / "images" / "plain.jpg").read_bytes()

    def test_exact_sizes_and_still_a_jpeg(self):
        base = len(self.plain)
        # includes totals that need the last-two-segments split (remainder of 1..3 bytes) and a multi-segment total
        for extra in (4, 5, 65537, 65538, 65540, 65541, 200_001):
            out = mf.pad_jpeg(self.plain, base + extra, "t")
            self.assertEqual(len(out), base + extra, extra)
            f = fv.inspect(out)
            self.assertEqual((f["format"], f["width"], f["height"]), ("jpeg", 1800, 1200), extra)
            self.assertTrue(out.endswith(self.plain[-1000:]))

    def test_deterministic_and_seeded(self):
        a = mf.pad_jpeg(self.plain, len(self.plain) + 100_000, "x")
        self.assertEqual(a, mf.pad_jpeg(self.plain, len(self.plain) + 100_000, "x"))
        self.assertNotEqual(a, mf.pad_jpeg(self.plain, len(self.plain) + 100_000, "y"))

    def test_limit_constants(self):
        self.assertEqual(mf.SIZE_LIMIT, 52_428_800)
        e = manifest_entries()
        self.assertEqual(e["generated/over_limit.jpg"]["size_bytes"], mf.SIZE_LIMIT + 1)

    def test_generated_folder_is_ignored(self):
        ignore = (fv.REPO / ".gitignore").read_text().splitlines()
        self.assertIn("fixtures/generated/", ignore)


if __name__ == "__main__":
    unittest.main()
