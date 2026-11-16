#!/usr/bin/env python3
"""Check the fixture manifest against the files on disk.

Run as: python3 -I scripts/fixtures_verify.py [--add-device] [--root DIR]

Pure standard library; works on Windows. The metadata parsers here are written
independently of the generator (make_fixtures.py) so a location claim in the
manifest is checked against the bytes, not against the generator's intent.
"""

import argparse
import hashlib
import importlib.util
import json
import struct
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_ROOT = REPO / "fixtures"
MAX_COMMITTED_BYTES = 400_000
DEVICE_FILES = [
    "iphone_hdr.heic",
    "iphone_live_still.heic",
    "android_ultrahdr.jpg",
    "android.heic",
]
HEIC_BRANDS = {b"heic", b"heix", b"heim", b"heis", b"hevc", b"hevx", b"hevm", b"hevs", b"mif1", b"msf1"}
XMP_NS = b"http://ns.adobe.com/xap/1.0/\x00"


# ---------------------------------------------------------------- TIFF / EXIF


def parse_tiff(buf):
    """Return {'gps': bool, 'orientation': int|None, 'time': str|None} or None."""
    try:
        if buf[:2] == b"II":
            e = "<"
        elif buf[:2] == b"MM":
            e = ">"
        else:
            return None
        if struct.unpack(e + "H", buf[2:4])[0] != 42:
            return None

        def ifd(off):
            n = struct.unpack(e + "H", buf[off : off + 2])[0]
            out = {}
            for i in range(n):
                p = off + 2 + 12 * i
                tag, typ, cnt = struct.unpack(e + "HHI", buf[p : p + 8])
                out[tag] = (typ, cnt, buf[p + 8 : p + 12])
            return out

        def long_of(entry):
            return struct.unpack(e + "I", entry[2])[0]

        first = ifd(struct.unpack(e + "I", buf[4:8])[0])
        res = {"gps": False, "orientation": None, "time": None}
        if 0x0112 in first:
            res["orientation"] = struct.unpack(e + "H", first[0x0112][2][:2])[0]
        if 0x8769 in first:
            sub = ifd(long_of(first[0x8769]))
            if 0x9003 in sub:
                typ, cnt, raw = sub[0x9003]
                off = struct.unpack(e + "I", raw)[0]
                res["time"] = buf[off : off + cnt].split(b"\x00")[0].decode("ascii", "replace")
        if 0x8825 in first:
            g = ifd(long_of(first[0x8825]))
            res["gps"] = all(t in g for t in (1, 2, 3, 4))
        return res
    except (struct.error, IndexError):
        return None


# ----------------------------------------------------------------------- JPEG


def inspect_jpeg(data):
    info = {"exif": None, "xmp_gps": False, "iptc_location": False, "width": None, "height": None}
    pos = 2
    n = len(data)
    while pos + 4 <= n and data[pos] == 0xFF:
        marker = data[pos + 1]
        if marker == 0xFF:
            pos += 1
            continue
        if marker == 0xD8 or 0xD0 <= marker <= 0xD7 or marker == 0x01:
            pos += 2
            continue
        length = struct.unpack(">H", data[pos + 2 : pos + 4])[0]
        seg = data[pos + 4 : pos + 2 + length]
        if marker == 0xE1 and seg.startswith(b"Exif\x00\x00"):
            info["exif"] = parse_tiff(seg[6:])
        elif marker == 0xE1 and seg.startswith(XMP_NS):
            info["xmp_gps"] = xmp_has_gps(seg[len(XMP_NS) :])
        elif marker == 0xED and seg.startswith(b"Photoshop 3.0\x00"):
            info["iptc_location"] = iptc_has_location(seg[14:])
        elif marker in (0xC0, 0xC1, 0xC2) and len(seg) >= 5:
            info["height"], info["width"] = struct.unpack(">HH", seg[1:5])
        if marker == 0xDA:
            break
        pos += 2 + length
    return info


def xmp_has_gps(xmp):
    return b"exif:GPSLatitude" in xmp and b"exif:GPSLongitude" in xmp


def iptc_has_location(res):
    """Walk Photoshop 8BIM resources; resource 0x0404 holds IPTC-IIM records."""
    pos = 0
    found = set()
    try:
        while pos + 12 <= len(res) and res[pos : pos + 4] == b"8BIM":
            rid = struct.unpack(">H", res[pos + 4 : pos + 6])[0]
            nlen = res[pos + 6]
            pos += 7 + nlen
            if (1 + nlen) % 2:
                pos += 1
            size = struct.unpack(">I", res[pos : pos + 4])[0]
            body = res[pos + 4 : pos + 4 + size]
            pos += 4 + size + (size % 2)
            if rid == 0x0404:
                i = 0
                while i + 5 <= len(body) and body[i] == 0x1C:
                    rec, ds, ln = body[i + 1], body[i + 2], struct.unpack(">H", body[i + 3 : i + 5])[0]
                    if rec == 2 and ln > 0:
                        found.add(ds)
                    i += 5 + ln
    except (struct.error, IndexError):
        pass
    # 2:90 city, 2:92 sub-location, 2:101 country name
    return {90, 92, 101} <= found


# ------------------------------------------------------------------------ PNG


def inspect_png(data):
    info = {"exif": None, "xmp_gps": False, "iptc_location": False, "width": None, "height": None}
    pos = 8
    while pos + 8 <= len(data):
        ln, typ = struct.unpack(">I4s", data[pos : pos + 8])
        body = data[pos + 8 : pos + 8 + ln]
        if typ == b"IHDR" and len(body) >= 8:
            info["width"], info["height"] = struct.unpack(">II", body[:8])
        elif typ == b"eXIf":
            info["exif"] = parse_tiff(body)
        elif typ == b"iTXt" and body.startswith(b"XML:com.adobe.xmp\x00"):
            info["xmp_gps"] = xmp_has_gps(body)
        if typ == b"IEND":
            break
        pos += 12 + ln
    return info


# ----------------------------------------------------------------------- HEIC


def boxes(data, start, end):
    pos = start
    while pos + 8 <= end:
        size, typ = struct.unpack(">I4s", data[pos : pos + 8])
        hdr = 8
        if size == 1:
            size = struct.unpack(">Q", data[pos + 8 : pos + 16])[0]
            hdr = 16
        elif size == 0:
            size = end - pos
        if size < hdr:
            return
        yield typ, pos + hdr, min(pos + size, end)
        pos += size


def find_box(data, start, end, name):
    for typ, s, e in boxes(data, start, end):
        if typ == name:
            return s, e
    return None


def heic_brand(data):
    if len(data) >= 12 and data[4:8] == b"ftyp":
        return data[8:12]
    return None


def inspect_heic(data):
    info = {"exif": None, "xmp_gps": False, "iptc_location": False, "width": None, "height": None}
    try:
        meta = find_box(data, 0, len(data), b"meta")
        if not meta:
            return info
        ms, me = meta[0] + 4, meta[1]  # skip version/flags
        iprp = find_box(data, ms, me, b"iprp")
        if iprp:
            ipco = find_box(data, iprp[0], iprp[1], b"ipco")
            if ipco:
                # A grid image has one ispe per tile plus the overall one; the largest is the image.
                best = 0
                for typ, s, e in boxes(data, ipco[0], ipco[1]):
                    if typ == b"ispe":
                        w, h = struct.unpack(">II", data[s + 4 : s + 12])
                        if w * h > best:
                            best = w * h
                            info["width"], info["height"] = w, h
        exif_id = None
        iinf = find_box(data, ms, me, b"iinf")
        if iinf:
            ver = data[iinf[0]]
            p = iinf[0] + 4 + (2 if ver == 0 else 4)
            for typ, s, e in boxes(data, p, iinf[1]):
                if typ != b"infe":
                    continue
                v = data[s]
                if v == 2:
                    item_id, itype = struct.unpack(">H", data[s + 4 : s + 6])[0], data[s + 8 : s + 12]
                elif v == 3:
                    item_id, itype = struct.unpack(">I", data[s + 4 : s + 8])[0], data[s + 10 : s + 14]
                else:
                    continue
                if itype == b"Exif":
                    exif_id = item_id
        iloc = find_box(data, ms, me, b"iloc")
        if exif_id is not None and iloc:
            s = iloc[0]
            ver = data[s]
            sizes = struct.unpack(">H", data[s + 4 : s + 6])[0]
            off_sz, len_sz, base_sz = sizes >> 12, (sizes >> 8) & 15, (sizes >> 4) & 15
            idx_sz = (sizes & 15) if ver in (1, 2) else 0
            p = s + 6
            if ver < 2:
                count = struct.unpack(">H", data[p : p + 2])[0]
                p += 2
            else:
                count = struct.unpack(">I", data[p : p + 4])[0]
                p += 4

            def rd(n):
                nonlocal p
                v = int.from_bytes(data[p : p + n], "big")
                p += n
                return v

            for _ in range(count):
                iid = rd(2 if ver < 2 else 4)
                if ver in (1, 2):
                    rd(2)  # construction method
                rd(2)  # data reference index
                base = rd(base_sz)
                ext_count = rd(2)
                extents = []
                for _ in range(ext_count):
                    if idx_sz:
                        rd(idx_sz)
                    extents.append((rd(off_sz), rd(len_sz)))
                if iid == exif_id and extents:
                    off, ln = extents[0]
                    blob = data[base + off : base + off + ln]
                    skip = struct.unpack(">I", blob[:4])[0]
                    tiff = blob[4 + skip :]
                    if tiff.startswith(b"Exif\x00\x00"):
                        tiff = tiff[6:]
                    info["exif"] = parse_tiff(tiff)
    except (struct.error, IndexError):
        pass
    return info


# ------------------------------------------------------------------ top level


def sniff_format(data):
    if data[:3] == b"\xff\xd8\xff":
        return "jpeg"
    if data[:8] == b"\x89PNG\r\n\x1a\n":
        return "png"
    if heic_brand(data) in HEIC_BRANDS:
        return "heic"
    return "none"


def inspect(data):
    """Facts read from the bytes alone."""
    fmt = sniff_format(data)
    info = {"exif": None, "xmp_gps": False, "iptc_location": False, "width": None, "height": None}
    if fmt == "jpeg":
        info = inspect_jpeg(data)
    elif fmt == "png":
        info = inspect_png(data)
    elif fmt == "heic":
        info = inspect_heic(data)
    kinds = []
    if info["exif"] and info["exif"]["gps"]:
        kinds.append("exif")
    if info["xmp_gps"]:
        kinds.append("xmp")
    if info["iptc_location"]:
        kinds.append("iptc")
    return {
        "format": fmt,
        "location_kinds": kinds,
        "orientation": info["exif"]["orientation"] if info["exif"] else None,
        "capture_time": info["exif"]["time"] if info["exif"] else None,
        "width": info["width"],
        "height": info["height"],
    }


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def load_manifest(root):
    return json.loads((root / "manifest.json").read_text(encoding="utf-8"))


def save_manifest(root, manifest):
    (root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def load_generator():
    spec = importlib.util.spec_from_file_location("make_fixtures", Path(__file__).with_name("make_fixtures.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def verify(root, generate=True):
    """Return (errors, notes). Never raises on a bad fixture."""
    root = Path(root)
    errors, notes = [], []
    try:
        manifest = load_manifest(root)
    except (OSError, ValueError) as exc:
        return [f"manifest.json unreadable: {exc}"], notes
    entries = manifest.get("files", [])
    listed = {entry["path"] for entry in entries}

    if generate and any(not entry.get("committed", True) for entry in entries):
        if any(not (root / entry["path"]).exists() for entry in entries if not entry.get("committed", True)):
            load_generator().make_large(root / "generated")

    for entry in entries:
        rel = entry["path"]
        path = root / rel
        if not path.is_file():
            if entry.get("source") == "device" and not entry.get("present", False):
                notes.append(f"{rel}: waiting for a person (see fixtures/device/README.md)")
            elif not entry.get("committed", True) and not generate:
                notes.append(f"{rel}: generated on demand, not present")
            else:
                errors.append(f"{rel}: file is missing")
            continue
        if entry.get("source") == "device" and not entry.get("present", False):
            errors.append(f"{rel}: present on disk but the manifest says it is not; run --add-device")
            continue
        data = path.read_bytes()
        if len(data) != entry["size_bytes"]:
            errors.append(f"{rel}: size {len(data)} != manifest {entry['size_bytes']}")
        if sha256(data) != entry["sha256"]:
            errors.append(f"{rel}: sha256 does not match the manifest")
        if rel.startswith("images/") and len(data) >= MAX_COMMITTED_BYTES:
            errors.append(f"{rel}: {len(data)} bytes, committed images must be under {MAX_COMMITTED_BYTES}")
        facts = inspect(data)
        if facts["format"] != entry["format"]:
            errors.append(f"{rel}: format by magic is {facts['format']}, manifest says {entry['format']}")
        claimed = sorted(entry.get("location_kinds", []))
        if bool(entry["has_location"]) != bool(claimed):
            errors.append(f"{rel}: has_location and location_kinds disagree")
        if claimed != sorted(facts["location_kinds"]):
            errors.append(f"{rel}: location claim {claimed} but the bytes hold {sorted(facts['location_kinds'])}")
        if entry.get("orientation") is not None and entry["orientation"] != facts["orientation"]:
            errors.append(f"{rel}: orientation {facts['orientation']} != manifest {entry['orientation']}")
        if entry.get("color_profile") and entry["color_profile"].encode("utf-16-be") not in data:
            errors.append(f"{rel}: colour profile {entry['color_profile']} not found in the bytes")
        for key in ("width", "height"):
            if facts[key] is not None and entry.get(key) is not None and facts[key] != entry[key]:
                errors.append(f"{rel}: {key} {facts[key]} != manifest {entry[key]}")

    images = root / "images"
    if images.is_dir():
        for path in sorted(images.iterdir()):
            if path.is_file() and f"images/{path.name}" not in listed:
                errors.append(f"images/{path.name}: file is not in the manifest")
    device = root / "device"
    if device.is_dir():
        for path in sorted(device.iterdir()):
            if path.is_file() and path.name != "README.md" and f"device/{path.name}" not in listed:
                errors.append(f"device/{path.name}: file is not in the manifest")
    return errors, notes


def add_device(root):
    root = Path(root)
    manifest = load_manifest(root)
    added = []
    for entry in manifest["files"]:
        if entry.get("source") != "device":
            continue
        path = root / entry["path"]
        if not path.is_file():
            continue
        data = path.read_bytes()
        facts = inspect(data)
        entry.update(
            present=True,
            format=facts["format"],
            size_bytes=len(data),
            sha256=sha256(data),
            valid=facts["format"] != "none",
            has_location=bool(facts["location_kinds"]),
            location_kinds=facts["location_kinds"],
            orientation=facts["orientation"],
            width=facts["width"],
            height=facts["height"],
        )
        added.append(entry["path"])
    save_manifest(root, manifest)
    return added


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--add-device", action="store_true", help="record size and hash of the real device photos")
    parser.add_argument("--root", default=str(DEFAULT_ROOT), help="fixtures folder (default: the repository's)")
    args = parser.parse_args(argv)
    root = Path(args.root)
    if args.add_device:
        added = add_device(root)
        print("recorded:", ", ".join(added) if added else "nothing (no device files found)")
    errors, notes = verify(root)
    for note in notes:
        print("note:", note)
    for err in errors:
        print("FAIL:", err)
    if errors:
        print(f"fixtures-verify: {len(errors)} problem(s)")
        return 1
    print("fixtures-verify: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
