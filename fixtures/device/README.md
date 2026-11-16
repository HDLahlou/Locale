# Real device photos

Four real photos are added here by a person. Tests that need them skip with "waiting for a person" until they exist. Never convert, edit, crop or share them through an app that recompresses: the point is the unconverted bytes.

| Name | What it is |
|---|---|
| `iphone_hdr.heic` | An iPhone photo taken with HDR on, in a bright scene, so it carries an HDR gain map |
| `iphone_live_still.heic` | The still of an iPhone Live Photo (a Live Photo taken with the camera app) |
| `android_ultrahdr.jpg` | An Android Ultra HDR JPEG (a phone camera with Ultra HDR on) |
| `android.heic` | An Android photo saved as HEIC (camera set to HEIC/HEIF) |

## Getting them off the phone unconverted

- iPhone: AirDrop to a Mac and choose Options, then All Photos Data, then Accept; or in Photos choose File, Export, Export Unmodified Original. If you use a cable and Image Capture, files are copied as they are. Do not email or message them: that can convert to JPEG or shrink them. In Settings, Photos, "Transfer to Mac or PC" must be "Keep Originals".
- Android: connect by USB in "File transfer" mode and copy the file out of `DCIM/Camera`, or use Files by Google, then Share, then Save to a computer folder over a cable. Do not use a chat app or cloud share that recompresses.

## Then

1. Copy the four files into this folder under exactly the names above.
2. Run `python3 -I scripts/fixtures_verify.py --add-device`. It records each file's size, hash and detected facts in `fixtures/manifest.json` and sets `"present": true`.
3. Run `just fixtures-verify`, then commit the four photos and the manifest.

The photos may contain real location data. That is fine for tests that strip it; do not publish the repository with photos you would not want public.
