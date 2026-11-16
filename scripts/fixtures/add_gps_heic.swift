// Re-writes a HEIC through ImageIO with a GPS dictionary, so the file's Exif item has a GPS IFD.
// Usage: xcrun swift scripts/fixtures/add_gps_heic.swift <in.heic> <out.heic>
// The location is synthetic and identical to every other fixture (see make_fixtures.py).
import Foundation
import ImageIO
import UniformTypeIdentifiers

let args = CommandLine.arguments
guard args.count == 3 else {
    FileHandle.standardError.write("usage: add_gps_heic <in> <out>\n".data(using: .utf8)!)
    exit(2)
}
let inURL = URL(fileURLWithPath: args[1])
let outURL = URL(fileURLWithPath: args[2])
guard let src = CGImageSourceCreateWithURL(inURL as CFURL, nil) else { exit(1) }
guard let dst = CGImageDestinationCreateWithURL(outURL as CFURL, UTType.heic.identifier as CFString, 1, nil) else { exit(1) }

let gps: [CFString: Any] = [
    kCGImagePropertyGPSLatitude: 12.3456,
    kCGImagePropertyGPSLatitudeRef: "N",
    kCGImagePropertyGPSLongitude: 98.7654,
    kCGImagePropertyGPSLongitudeRef: "E",
]
let exif: [CFString: Any] = [
    kCGImagePropertyExifDateTimeOriginal: "2026:01:02 03:04:05",
]
let props: [CFString: Any] = [
    kCGImagePropertyGPSDictionary: gps,
    kCGImagePropertyExifDictionary: exif,
    kCGImagePropertyOrientation: 1,
    kCGImageDestinationLossyCompressionQuality: 0.7,
]
CGImageDestinationAddImageFromSource(dst, src, 0, props as CFDictionary)
if !CGImageDestinationFinalize(dst) { exit(1) }
