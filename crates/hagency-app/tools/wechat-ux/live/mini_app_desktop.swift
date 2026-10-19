// Capture/hit-test only the owned native test window, including its WKWebView.
import AppKit
import CoreGraphics
import ApplicationServices
import Foundation

guard CommandLine.arguments.count >= 2, let pid = Int32(CommandLine.arguments[1]),
      let app = NSRunningApplication(processIdentifier: pid) else { exit(2) }
app.activate(options: [.activateAllWindows])
AXUIElementSetAttributeValue(AXUIElementCreateApplication(pid), kAXFrontmostAttribute as CFString, kCFBooleanTrue)
Thread.sleep(forTimeInterval: 0.3)
let windows = CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID) as? [[String: Any]] ?? []
guard let window = windows.first(where: {
    ($0[kCGWindowOwnerPID as String] as? Int32) == pid &&
    ($0[kCGWindowLayer as String] as? Int) == 0 &&
    ($0[kCGWindowName as String] as? String ?? "").hasPrefix("Rinx")
}), let bounds = window[kCGWindowBounds as String] as? [String: Double],
let number = window[kCGWindowNumber as String] as? Int else { exit(3) }
if CommandLine.arguments.count == 4 {
    guard NSWorkspace.shared.frontmostApplication?.processIdentifier == pid else { exit(4) }
    let point = CGPoint(x: bounds["X"]! + Double(CommandLine.arguments[2])!,
                        y: bounds["Y"]! + Double(CommandLine.arguments[3])!)
    for kind in [CGEventType.leftMouseDown, CGEventType.leftMouseUp] {
        let event = CGEvent(mouseEventSource: nil, mouseType: kind, mouseCursorPosition: point, mouseButton: .left)!
        event.setIntegerValueField(.mouseEventWindowUnderMousePointer, value: Int64(number))
        event.setIntegerValueField(.mouseEventWindowUnderMousePointerThatCanHandleThisEvent, value: Int64(number))
        event.post(tap: .cghidEventTap)
        Thread.sleep(forTimeInterval: 0.1)
    }
}
let data = try JSONSerialization.data(withJSONObject: ["window_id": number, "bounds": bounds])
FileHandle.standardOutput.write(data)
