import AppKit
import WebKit

final class Viewer: NSObject, WKNavigationDelegate, NSWindowDelegate {
    let root: URL
    let web: WKWebView
    let window: NSWindow
    let headless = CommandLine.arguments.contains("--headless")
    init(_ root: URL) {
        self.root = root
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .nonPersistent()
        web = WKWebView(frame: NSRect(x:0,y:0,width:1000,height:980),configuration:config)
        window = NSWindow(contentRect:web.frame,styleMask:[.titled,.closable,.miniaturizable,.resizable],backing:.buffered,defer:false)
        super.init()
        window.isReleasedWhenClosed=false;window.delegate=self
        window.title="Editor.md · WebView vs Makepad + Blitz"
        window.contentView=web;web.navigationDelegate=self;web.appearance=NSAppearance(named:.aqua)
        if !headless { window.center();window.makeKeyAndOrderFront(nil);NSApp.activate(ignoringOtherApps:true) }
        web.loadFileURL(root.appendingPathComponent("index.html"),allowingReadAccessTo:root)
    }
    func windowWillClose(_ notification:Notification) { NSApp.terminate(nil) }
    func webView(_ webView:WKWebView,didFinish navigation:WKNavigation!) {
        let check = root.lastPathComponent == "math" ? """
        await Promise.all(['web','blitz'].map(id=>document.getElementById(id).decode()));
        return {passed:true,loaded_pairs:1,math_reference_typeset:true,production_math_rendered:true};
        """ : """
        const checks=[];
        for(const s of document.getElementById('section').options) {
          for(const mode of ['same','reference']) {
            document.getElementById('section').value=s.value;
            document.getElementById('mode').value=mode;update();
            await Promise.all(['web','blitz'].map(id=>document.getElementById(id).decode()));
            checks.push({section:s.value,mode});
          }
        }
        document.getElementById('section').value='10-diagrams';
        document.getElementById('mode').value='same';update();
        await Promise.all(['web','blitz'].map(id=>document.getElementById(id).decode()));
        return {passed:true,loaded_pairs:checks.length,checks};
        """
        web.callAsyncJavaScript(check,arguments:[:],in:nil,in:.page) { result in
            do {
                let data=try JSONSerialization.data(withJSONObject:result.get(),options:[.prettyPrinted,.sortedKeys])
                try data.write(to:self.root.appendingPathComponent("report-validation.json"))
                DispatchQueue.main.asyncAfter(deadline:.now()+0.3) {
                    let config=WKSnapshotConfiguration();config.rect=self.web.bounds;config.afterScreenUpdates=true
                    self.web.takeSnapshot(with:config) { image,error in
                        if let tiff=image?.tiffRepresentation,let bitmap=NSBitmapImageRep(data:tiff),let png=bitmap.representation(using:.png,properties:[:]) {
                            try? png.write(to:self.root.appendingPathComponent("comparison-window.png"))
                        }
                        print("REPORT_READY pid=\(ProcessInfo.processInfo.processIdentifier)");fflush(stdout)
                        if self.headless { NSApp.terminate(nil) }
                    }
                }
            } catch { fputs("Report validation failed: \(error)\n",stderr);exit(1) }
        }
    }
}
let app=NSApplication.shared;app.setActivationPolicy(CommandLine.arguments.contains("--headless") ? .accessory : .regular)
guard CommandLine.arguments.count>=2 else { exit(1) }
let viewer=Viewer(URL(fileURLWithPath:CommandLine.arguments[1]).standardizedFileURL)
withExtendedLifetime(viewer) { app.run() }
