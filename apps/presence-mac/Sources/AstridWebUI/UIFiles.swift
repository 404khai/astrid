import Foundation
import WebKit

/// Read-only file server for the built web UI folder (`web/dist-presence`),
/// served over the private `astrid-ui://` scheme; never leaves that folder.
/// Shared by the presence host (ADRs 0033/0036) and the desktop app (ADR 0037).
public final class UIFiles: NSObject, WKURLSchemeHandler {
    private let root: URL
    public init(root: URL) { self.root = root.standardizedFileURL }

    public func webView(_ webView: WKWebView, start task: WKURLSchemeTask) {
        guard let url = task.request.url else { return }
        let file = root.appendingPathComponent(url.path).standardizedFileURL
        guard file.path.hasPrefix(root.path + "/"), let data = try? Data(contentsOf: file) else {
            task.didFailWithError(URLError(.fileDoesNotExist))
            return
        }
        let types = ["html": "text/html", "js": "text/javascript", "css": "text/css", "woff2": "font/woff2", "woff": "font/woff", "svg": "image/svg+xml", "png": "image/png", "json": "application/json"]
        let mime = types[file.pathExtension.lowercased()] ?? "application/octet-stream"
        task.didReceive(HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1", headerFields: ["Content-Type": mime, "Content-Length": String(data.count)])!)
        task.didReceive(data)
        task.didFinish()
    }

    public func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {}
}
