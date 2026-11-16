#!/usr/bin/env python3
"""Make the image fixtures. Documentation of how each file was made.

  python3 -I scripts/make_fixtures.py          small committed files (macOS: needs sips and xcrun swift)
  python3 -I scripts/make_fixtures.py --large  the two large files into fixtures/generated/ (pure Python)

The small files are committed as binaries; a test run never regenerates them (CI on Windows has no sips).
Only run the first form to deliberately remake them, then commit the result.
All location values are synthetic and identical in every file (LOCATION below).
"""

import argparse
import hashlib
import importlib.util
import json
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent / "fixtures"
SIZE_LIMIT = 52_428_800
P3_PROFILE = "/System/Library/ColorSync/Profiles/Display P3.icc"

LOCATION = {
    "latitude": 12.3456,
    "longitude": 98.7654,
    "exif_latitude_rationals": [[12, 1], [20, 1], [4416, 100]],
    "exif_longitude_rationals": [[98, 1], [45, 1], [5544, 100]],
    "exif_latitude_ref": "N",
    "exif_longitude_ref": "E",
    "xmp_latitude": "12,20.736N",
    "xmp_longitude": "98,45.924E",
    "iptc_city": "Testville",
    "iptc_sub_location": "Test Quarter",
    "iptc_country": "Testland",
}
CAPTURE_TIME = "2026:01:02 03:04:05"
ORIENTATION = 6

W, H = 1800, 1200


def load_verifier():
    spec = importlib.util.spec_from_file_location("fixtures_verify", HERE / "fixtures_verify.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


# ------------------------------------------------------------- base image


def base_rows():
    """A smooth gradient with a few sharp shapes; compresses well, no noise."""
    circles = [(450, 400, 260, (230, 70, 60)), (1200, 700, 330, (40, 150, 210)), (900, 300, 140, (250, 210, 60))]
    for y in range(H):
        row = bytearray()
        for x in range(W):
            r, g, b = (x * 255) // W, (y * 255) // H, 255 - ((x + y) * 255) // (W + H)
            for cx, cy, rad, col in circles:
                if (x - cx) ** 2 + (y - cy) ** 2 < rad * rad:
                    r, g, b = col
            if (x // 150 + y // 150) % 2 == 0 and y > 1000:
                r, g, b = 20, 20, 20
            row += bytes((r, g, b))
        yield bytes(row)


def png_chunk(typ, body):
    return struct.pack(">I", len(body)) + typ + body + struct.pack(">I", zlib.crc32(typ + body) & 0xFFFFFFFF)


def make_plain_png():
    raw = bytearray()
    for row in base_rows():
        # filter type 1 (Sub): byte minus the byte one pixel to the left
        raw.append(1)
        raw += row[:3] + bytes((row[i] - row[i - 3]) & 255 for i in range(3, len(row)))
    ihdr = struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", ihdr)
        + png_chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + png_chunk(b"IEND", b"")
    )


# --------------------------------------------------------- metadata builders


def tiff(gps, orientation=ORIENTATION, time=CAPTURE_TIME):
    """Big-endian TIFF: IFD0 (orientation, Exif IFD, optional GPS IFD), Exif IFD (DateTimeOriginal)."""
    e = ">"
    n0 = 3 if gps else 2
    ifd0_off = 8
    ifd0_len = 2 + 12 * n0 + 4
    exif_off = ifd0_off + ifd0_len
    exif_len = 2 + 12 + 4
    time_off = exif_off + exif_len
    time_b = time.encode("ascii") + b"\x00"
    gps_off = time_off + len(time_b)

    def entry(tag, typ, cnt, val):
        return struct.pack(e + "HHI", tag, typ, cnt) + val

    ifd0 = [
        entry(0x0112, 3, 1, struct.pack(e + "HH", orientation, 0)),
        entry(0x8769, 4, 1, struct.pack(e + "I", exif_off)),
    ]
    if gps:
        ifd0.append(entry(0x8825, 4, 1, struct.pack(e + "I", gps_off)))
    out = b"MM\x00\x2a" + struct.pack(e + "I", ifd0_off)
    out += struct.pack(e + "H", n0) + b"".join(ifd0) + struct.pack(e + "I", 0)
    out += struct.pack(e + "H", 1) + entry(0x9003, 2, len(time_b), struct.pack(e + "I", time_off))
    out += struct.pack(e + "I", 0)
    out += time_b
    if gps:
        lat, lon = LOCATION["exif_latitude_rationals"], LOCATION["exif_longitude_rationals"]
        data_off = gps_off + 2 + 12 * 5 + 4
        rat = lambda rs: b"".join(struct.pack(e + "II", a, b) for a, b in rs)
        out += struct.pack(e + "H", 5)
        out += entry(0, 1, 4, b"\x02\x03\x00\x00")
        out += entry(1, 2, 2, b"N\x00\x00\x00")
        out += entry(2, 5, 3, struct.pack(e + "I", data_off))
        out += entry(3, 2, 2, b"E\x00\x00\x00")
        out += entry(4, 5, 3, struct.pack(e + "I", data_off + 24))
        out += struct.pack(e + "I", 0)
        out += rat(lat) + rat(lon)
    return out


def xmp_packet():
    return (
        '<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>\n'
        '<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">\n'
        '<rdf:Description rdf:about="" xmlns:exif="http://ns.adobe.com/exif/1.0/" '
        'xmlns:tiff="http://ns.adobe.com/tiff/1.0/"\n'
        f' exif:GPSLatitude="{LOCATION["xmp_latitude"]}" exif:GPSLongitude="{LOCATION["xmp_longitude"]}"\n'
        f' exif:DateTimeOriginal="2026-01-02T03:04:05" tiff:Orientation="{ORIENTATION}"/>\n'
        "</rdf:RDF></x:xmpmeta>\n"
        '<?xpacket end="w"?>'
    ).encode("utf-8")


def iptc_block():
    def ds(num, text):
        b = text.encode("utf-8")
        return b"\x1c\x02" + bytes((num,)) + struct.pack(">H", len(b)) + b

    body = ds(90, LOCATION["iptc_city"]) + ds(92, LOCATION["iptc_sub_location"]) + ds(101, LOCATION["iptc_country"])
    if len(body) % 2:
        body += b"\x00"
    return b"Photoshop 3.0\x00" + b"8BIM" + struct.pack(">H", 0x0404) + b"\x00\x00" + struct.pack(">I", len(body)) + body


def jpeg_segment(marker, body):
    return b"\xff" + bytes((marker,)) + struct.pack(">H", len(body) + 2) + body


def jpeg_insert(data, segments):
    """Insert segments after SOI and a leading JFIF APP0, if any."""
    pos = 2
    if data[2:4] == b"\xff\xe0":
        pos = 2 + 2 + struct.unpack(">H", data[4:6])[0]
    return data[:pos] + b"".join(segments) + data[pos:]


def jpeg_strip_meta(data):
    """Drop APP1 (Exif, XMP) and APP13 (Photoshop/IPTC) segments, keeping everything else (JFIF, ICC)."""
    out, pos = data[:2], 2
    while data[pos] == 0xFF and data[pos + 1] != 0xDA:
        end = pos + 2 + struct.unpack(">H", data[pos + 2 : pos + 4])[0]
        if data[pos + 1] not in (0xE1, 0xED):
            out += data[pos:end]
        pos = end
    return out + data[pos:]


def jpeg_with(data, exif_gps=None, xmp=False, iptc=False):
    """exif_gps None: no Exif segment; False: orientation and time only; True: with GPS."""
    data = jpeg_strip_meta(data)
    segs = []
    if exif_gps is not None:
        segs.append(jpeg_segment(0xE1, b"Exif\x00\x00" + tiff(exif_gps)))
    if xmp:
        segs.append(jpeg_segment(0xE1, b"http://ns.adobe.com/xap/1.0/\x00" + xmp_packet()))
    if iptc:
        segs.append(jpeg_segment(0xED, iptc_block()))
    return jpeg_insert(data, segs)


def png_with(data, exif=False, xmp=False):
    chunks = b""
    if exif:
        chunks += png_chunk(b"eXIf", tiff(True))
    if xmp:
        chunks += png_chunk(b"iTXt", b"XML:com.adobe.xmp\x00\x00\x00\x00\x00" + xmp_packet())
    pos = 8 + 12 + 13  # signature + IHDR chunk
    return data[:pos] + chunks + data[pos:]


# ----------------------------------------------------------------- large files


def _pad_stream(seed, n):
    out = bytearray()
    counter = 0
    while len(out) < n:
        out += hashlib.sha256(f"{seed}:{counter}".encode()).digest()
        counter += 1
    return bytes(out[:n])


def pad_jpeg(data, total, seed):
    """Insert COM segments of seeded pseudo-random bytes so the file is exactly `total` bytes."""
    remaining = total - len(data)
    if remaining < 4:
        raise ValueError("target smaller than the source plus one segment")
    segs = []
    index = 0
    while remaining > 0:
        chunk = min(remaining, 65537)  # marker 2 + length 2 + payload up to 65533
        if 0 < remaining - chunk < 4:
            chunk -= 4
        segs.append(jpeg_segment(0xFE, _pad_stream(f"{seed}:{index}", chunk - 4)))
        remaining -= chunk
        index += 1
    return jpeg_insert(data, segs)


def make_large(outdir):
    outdir = Path(outdir)
    outdir.mkdir(parents=True, exist_ok=True)
    plain = (ROOT / "images" / "plain.jpg").read_bytes()
    for name, total in (("large_50mb.jpg", SIZE_LIMIT), ("over_limit.jpg", SIZE_LIMIT + 1)):
        (outdir / name).write_bytes(pad_jpeg(plain, total, "locale-fixture-" + name))


# -------------------------------------------------------------- small files


def run(*cmd):
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL)


def make_small():
    images = ROOT / "images"
    images.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        png = make_plain_png()
        (images / "plain.png").write_bytes(png)
        run("sips", "-s", "format", "jpeg", "-s", "formatOptions", "70", str(images / "plain.png"), "--out", str(tmp / "plain.jpg"))
        run("sips", "-s", "format", "heic", "-s", "formatOptions", "60", str(images / "plain.png"), "--out", str(tmp / "plain.heic"))
        run("sips", "-s", "format", "jpeg", "-s", "formatOptions", "70", "--matchTo", P3_PROFILE, str(images / "plain.png"), "--out", str(tmp / "p3.jpg"))
        run("xcrun", "swift", str(HERE / "fixtures" / "add_gps_heic.swift"), str(tmp / "plain.heic"), str(tmp / "gps_exif.heic"))
        jpg = (tmp / "plain.jpg").read_bytes()
        out = {
            "plain.jpg": jpg,
            "plain.heic": (tmp / "plain.heic").read_bytes(),
            "p3.jpg": (tmp / "p3.jpg").read_bytes(),
            "orientation6.jpg": jpeg_with(jpg, exif_gps=False),
            "gps_exif.jpg": jpeg_with(jpg, exif_gps=True),
            "gps_xmp.jpg": jpeg_with(jpg, exif_gps=False, xmp=True),
            "gps_iptc.jpg": jpeg_with(jpg, exif_gps=False, iptc=True),
            "gps_all.jpg": jpeg_with(jpg, exif_gps=True, xmp=True, iptc=True),
            "gps_exif.png": png_with(png, exif=True),
            "gps_xmp.png": png_with(png, xmp=True),
            "gps_exif.heic": (tmp / "gps_exif.heic").read_bytes(),
        }
        for name, data in out.items():
            (images / name).write_bytes(data)
    cut = lambda b: b[: len(b) * 60 // 100]
    broken = {
        "truncated.jpg": cut((images / "plain.jpg").read_bytes()),
        "truncated.png": cut((images / "plain.png").read_bytes()),
        "truncated.heic": cut((images / "plain.heic").read_bytes()),
        "wrong_magic.jpg": (images / "plain.png").read_bytes(),
        "wrong_magic.heic": (images / "plain.jpg").read_bytes(),
        "not_an_image.jpg": b"This is a text file with an image name.\n",
        "empty.jpg": b"",
        "heic_bad_brand.heic": b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00mp42isom" + b"\x00\x00\x00\x08free",
    }
    for name, data in broken.items():
        (images / name).write_bytes(data)


# ----------------------------------------------------------------- manifest

# name: (valid, location kinds, orientation or None to read from the bytes, purpose)
SMALL = {
    "plain.jpg": (True, [], None, "valid JPEG, no location data, long edge 1800 px (1024 px preview is a real downscale)"),
    "plain.png": (True, [], None, "valid PNG, no location data, long edge 1800 px"),
    "plain.heic": (True, [], None, "valid HEIC made with sips, no location data, long edge 1800 px"),
    "orientation6.jpg": (True, [], ORIENTATION, "EXIF orientation 6 (rotate 90 CW) to tell whether orientation was applied"),
    "p3.jpg": (True, [], None, "carries a Display P3 colour profile to tell whether the profile is kept"),
    "gps_exif.jpg": (True, ["exif"], ORIENTATION, "location in the EXIF GPS IFD; orientation and capture time must survive a strip"),
    "gps_xmp.jpg": (True, ["xmp"], ORIENTATION, "location in an XMP packet; orientation and capture time must survive a strip"),
    "gps_iptc.jpg": (True, ["iptc"], ORIENTATION, "location in IPTC city, sub-location and country; orientation and capture time must survive a strip"),
    "gps_all.jpg": (True, ["exif", "xmp", "iptc"], ORIENTATION, "location in EXIF, XMP and IPTC together; orientation and capture time must survive a strip"),
    "gps_exif.png": (True, ["exif"], ORIENTATION, "PNG with an eXIf chunk holding a GPS IFD"),
    "gps_xmp.png": (True, ["xmp"], None, "PNG with an XMP iTXt chunk holding GPS properties"),
    "gps_exif.heic": (True, ["exif"], None, "HEIC whose Exif item has a GPS IFD (written through ImageIO)"),
    "truncated.jpg": (False, [], None, "plain.jpg cut at 60% of its length"),
    "truncated.png": (False, [], None, "plain.png cut at 60% of its length"),
    "truncated.heic": (False, [], None, "plain.heic cut at 60% of its length"),
    "wrong_magic.jpg": (False, [], None, "PNG bytes with a .jpg name; claimed type and magic bytes disagree"),
    "wrong_magic.heic": (False, [], None, "JPEG bytes with a .heic name; claimed type and magic bytes disagree"),
    "not_an_image.jpg": (False, [], None, "plain text with a .jpg name"),
    "empty.jpg": (False, [], None, "zero bytes with a .jpg name"),
    "heic_bad_brand.heic": (False, [], None, "ISO-BMFF file whose ftyp brand is mp42, not an image brand"),
}

DEVICE = {
    "iphone_hdr.heic": "real iPhone HDR photo with a gain map, unconverted",
    "iphone_live_still.heic": "real iPhone Live Photo still, unconverted",
    "android_ultrahdr.jpg": "real Android Ultra HDR JPEG, unconverted",
    "android.heic": "real Android HEIC, unconverted",
}


def make_manifest():
    ver = load_verifier()
    old = {}
    mpath = ROOT / "manifest.json"
    if mpath.exists():
        old = {e["path"]: e for e in json.loads(mpath.read_text(encoding="utf-8"))["files"] if e.get("source") == "device"}
    files = []
    for name, (valid, kinds, orient, purpose) in SMALL.items():
        data = (ROOT / "images" / name).read_bytes()
        facts = ver.inspect(data)
        # format, size and hash come from the bytes; location and validity are the intent above
        files.append(
            {
                "path": f"images/{name}",
                "format": facts["format"],
                "claimed_format": name.rsplit(".", 1)[1].replace("jpg", "jpeg"),
                "size_bytes": len(data),
                "sha256": ver.sha256(data),
                "valid": valid,
                "has_location": bool(kinds),
                "location_kinds": kinds,
                "orientation": orient,
                "capture_time": CAPTURE_TIME if kinds or name == "orientation6.jpg" else None,
                "width": facts["width"],
                "height": facts["height"],
                "color_profile": "Display P3" if name == "p3.jpg" else None,
                "committed": True,
                "purpose": purpose,
            }
        )
    with tempfile.TemporaryDirectory() as tmp:
        make_large(tmp)
        for name, purpose in (
            ("large_50mb.jpg", "valid JPEG of exactly 52,428,800 bytes (the size limit); generated by --large"),
            ("over_limit.jpg", "valid JPEG of exactly 52,428,801 bytes (one over the limit); generated by --large"),
        ):
            data = (Path(tmp) / name).read_bytes()
            facts = ver.inspect(data)
            files.append(
                {
                    "path": f"generated/{name}",
                    "format": "jpeg",
                    "claimed_format": "jpeg",
                    "size_bytes": len(data),
                    "sha256": ver.sha256(data),
                    "valid": True,
                    "has_location": False,
                    "location_kinds": [],
                    "orientation": None,
                    "capture_time": None,
                    "width": facts["width"],
                    "height": facts["height"],
                    "committed": False,
                    "purpose": purpose,
                }
            )
    for name, purpose in DEVICE.items():
        path = f"device/{name}"
        files.append(
            old.get(path)
            or {
                "path": path,
                "source": "device",
                "present": False,
                "format": name.rsplit(".", 1)[1].replace("jpg", "jpeg"),
                "claimed_format": name.rsplit(".", 1)[1].replace("jpg", "jpeg"),
                "size_bytes": None,
                "sha256": None,
                "valid": True,
                "has_location": False,
                "location_kinds": [],
                "orientation": None,
                "capture_time": None,
                "width": None,
                "height": None,
                "committed": True,
                "purpose": purpose,
            }
        )
    manifest = {
        "version": 1,
        "size_limit_bytes": SIZE_LIMIT,
        "location": LOCATION,
        "location_note": "Synthetic. Search output bytes for these strings to prove a strip worked.",
        "files": files,
    }
    mpath.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--large", action="store_true", help="only generate fixtures/generated/*")
    args = parser.parse_args()
    if args.large:
        make_large(ROOT / "generated")
        print("wrote", ROOT / "generated")
        return 0
    make_small()
    make_manifest()
    print("wrote small fixtures and manifest; review with git status, then commit")
    return 0


if __name__ == "__main__":
    sys.exit(main())
