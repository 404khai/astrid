// Screenshot point samples only; not a full accessibility certification.
import AppKit
let path = CommandLine.arguments[1]
let rep = NSBitmapImageRep(data: try Data(contentsOf: URL(fileURLWithPath: path)))!
func luminance(_ c: NSColor) -> Double {
    let rgb = c.usingColorSpace(.sRGB)!
    func linear(_ v: Double) -> Double { v <= 0.04045 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4) }
    return 0.2126 * linear(rgb.redComponent) + 0.7152 * linear(rgb.greenComponent) + 0.0722 * linear(rgb.blueComponent)
}
let points = [140,400,580,720,1000,1180,1360,1550,1750].map { ($0,280) } + [(1000,780)]
let rows: [[String: Any]] = points.map { x, y in
    let color = rep.colorAt(x: x, y: y)!
    return ["xy": [x,y], "color": color.description, "white_contrast": 1.05 / (luminance(color) + 0.05)]
}
let data = try JSONSerialization.data(withJSONObject: ["method": "sRGB screenshot point samples; hypothetical white foreground; not full audit", "rows": rows], options: [.prettyPrinted, .sortedKeys])
print(String(data: data, encoding: .utf8)!)
