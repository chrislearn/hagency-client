import AppKit
import WebKit
import CryptoKit

// Capture actual WKWebView pixels, with no page scripts or persistent account data.
final class Capture: NSObject, WKNavigationDelegate {
    let root: URL
    let jobs: [[String: String]]
    let web: WKWebView
    let window: NSWindow
    var index = 0
    var positions: [Int] = []
    var tiles: [[String: Any]] = []
    var result: [String: Any] = [:]
    var serial = 0

    init(_ path: String) throws {
        root = URL(fileURLWithPath: path).standardizedFileURL
        jobs = try JSONSerialization.jsonObject(with: Data(contentsOf: root.appendingPathComponent("webview-jobs.json"))) as! [[String: String]]
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .nonPersistent()
        config.defaultWebpagePreferences.allowsContentJavaScript = false
        web = WKWebView(frame: NSRect(x: 0, y: 0, width: 440, height: 700), configuration: config)
        web.appearance = NSAppearance(named: .aqua)
        window = NSWindow(contentRect: web.frame, styleMask: .borderless, backing: .buffered, defer: false)
        super.init()
        window.isReleasedWhenClosed = false
        window.contentView = web
        window.alphaValue = 0
        window.orderFront(nil)
        web.navigationDelegate = self
        next()
    }

    func fail(_ error: Error) { fputs("WKWebView: \(error)\n", stderr); exit(1) }
    func next() {
        if index == jobs.count { print("All \(jobs.count) WKWebView pages captured"); NSApp.terminate(nil); return }
        serial += 1
        let ticket = serial
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) {
            if ticket == self.serial { self.fail(NSError(domain: "Page timed out", code: 1)) }
        }
        web.loadFileURL(root.appendingPathComponent(jobs[index]["input"]!), allowingReadAccessTo: root)
    }
    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) { fail(error) }
    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) { fail(error) }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        web.callAsyncJavaScript("""
        await document.fonts.ready;
        await Promise.all([...document.images].map(i=>i.decode().catch(()=>{})));
        const rect=e=>{const r=e.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height}};
        return {height:document.documentElement.scrollHeight,width:innerWidth,dpr:devicePixelRatio,
          font:getComputedStyle(document.body).font,
          math:{formulas:document.querySelectorAll('.katex').length,errors:document.querySelectorAll('.katex-error').length},
          fonts:[...document.fonts].map(f=>({family:f.family,status:f.status})),
          elements:Object.fromEntries(['h1','h2','h3','p','pre','table','li','blockquote'].map(s=>[s,[...document.querySelectorAll(s)].map(rect)])),
          images:[...document.images].map(i=>({src:i.getAttribute('src'),complete:i.complete,width:i.naturalWidth,height:i.naturalHeight}))};
        """, arguments: [:], in: nil, in: .page) { value in
            do {
                self.result = try value.get() as! [String: Any]
                let height = (self.result["height"] as! NSNumber).intValue
                guard height <= 8192 else { throw NSError(domain: "Section taller than capture budget", code: 1) }
                let input = self.root.appendingPathComponent(self.jobs[self.index]["input"]!)
                self.result["source_sha256"] = SHA256.hash(data: try Data(contentsOf: input)).map { String(format: "%02x", $0) }.joined()
                self.result["engine"] = "macOS WKWebView / WebKit"
                self.result["macos"] = ProcessInfo.processInfo.operatingSystemVersionString
                self.result["page_javascript"] = false
                self.tiles = []
                self.positions = Array(stride(from: 0, through: max(0, height-700), by: 700))
                if self.positions.last != max(0, height-700) { self.positions.append(max(0, height-700)) }
                self.capture()
            } catch { self.fail(error) }
        }
    }
    func capture() {
        let prefix = jobs[index]["output"]!
        if positions.isEmpty {
            do {
                result["tiles"] = tiles
                try JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted,.sortedKeys]).write(to: root.appendingPathComponent(prefix + ".json"))
                print("Captured \(prefix)"); fflush(stdout)
                index += 1; next()
            } catch { fail(error) }
            return
        }
        let y = positions.removeFirst()
        web.evaluateJavaScript("scrollTo(0,\(y)); scrollY") { value, error in
            if let error = error { self.fail(error); return }
            guard (value as? NSNumber)?.intValue == y else { self.fail(NSError(domain: "Scroll mismatch", code: 1)); return }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) {
                let config = WKSnapshotConfiguration()
                config.rect = self.web.bounds; config.snapshotWidth = 440; config.afterScreenUpdates = true
                self.web.takeSnapshot(with: config) { image, error in
                    if let error = error { self.fail(error); return }
                    do {
                        guard let tiff = image?.tiffRepresentation, let bitmap = NSBitmapImageRep(data: tiff),
                              bitmap.pixelsWide == 880, bitmap.pixelsHigh == 1400,
                              let png = bitmap.representation(using: .png, properties: [:]) else {
                            throw NSError(domain: "Unexpected native pixel dimensions", code: 1)
                        }
                        let file = prefix + "-tile-\(y).png"
                        try png.write(to: self.root.appendingPathComponent(file))
                        self.tiles.append(["y_css": y, "file": file])
                        self.capture()
                    } catch { self.fail(error) }
                }
            }
        }
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
guard CommandLine.arguments.count == 2 else { exit(1) }
do {
    let capture = try Capture(CommandLine.arguments[1])
    withExtendedLifetime(capture) { app.run() }
} catch { fputs("\(error)\n", stderr); exit(1) }
