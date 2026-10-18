#!/usr/bin/env python3
"""Native Matrix mini-app card -> WKWebView -> share to a second Palpo chat.

WindowServer screenshots are required: Makepad's GPU-only captures omit WebKit.
The local HTTP fixture records real web navigation and JavaScript button input.
"""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import threading
import time

from native_probe import NativeApp
from seed import checked


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--headless", action="store_true", help="Hidden Makepad instrumentation and WebKit load checks; no desktop clicks/screenshots")
    args = parser.parse_args()
    live = Path("lab/wechat-ux/evidence/live")
    root = live / ("mini-app-headless-check" if args.headless else "mini-app-check")
    root.mkdir(mode=0o700, exist_ok=True)
    shutil.copyfile(live / "fixture.json", root / "fixture.json")
    os.chmod(root / "fixture.json", 0o600)
    fixture = json.loads((root / "fixture.json").read_text())
    requests = []
    class Page(BaseHTTPRequestHandler):
        def do_GET(self):
            requests.append({"path": self.path, "at": time.time()})
            page = '''<!doctype html><html><meta name="viewport" content="width=device-width, initial-scale=1">
<style>body{font:18px "PingFang SC",sans-serif;padding:20px;color:#191919;background:#fff}button,a{display:block;padding:20px;margin:20px 0;background:#07c160;color:white;border:0;border-radius:8px;font:inherit}</style>
<h1>Rinx Web Mini App</h1><p>网页小应用 · English and 中文</p>
<button onclick="fetch('/clicked').then(()=>this.textContent='Interaction passed')">Test interaction</button>
<a href="/second">Open second page</a><p>This content is rendered by WebKit.</p>
<script>window.addEventListener('load',()=>fetch('/ready'))</script></html>'''
            if self.path.startswith("/second"):
                page = "<html><meta name='viewport' content='width=device-width, initial-scale=1'><h1>Second page</h1><p>Web navigation works.</p></html>"
            data = page.encode()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        def log_message(self, *args):
            pass
    server = ThreadingHTTPServer(("127.0.0.1", 0), Page)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    address = f"http://127.0.0.1:{server.server_port}/app"
    app = NativeApp(root)
    title = "Daily Mini App " + app.output.name[:6]
    result = {"passed": False, "headless": args.headless, "evidence": str(app.output), "checks": []}
    helper = Path("target/robrix-mini-app-desktop")
    source = Path(__file__).with_name("mini_app_desktop.swift")
    if not args.headless and (not helper.exists() or helper.stat().st_mtime < source.stat().st_mtime):
        subprocess.run(["swiftc", str(source), "-o", str(helper)], check=True, capture_output=True)

    def fill(widget, text):
        app.click_id(widget)
        app.request("/k", c="A", cmd=1, wait=1)
        app.request("/t", t=text, wait=1)

    def desktop(name):
        if args.headless:
            app.capture(name + "-makepad")
            return {}, app.ocr()
        info = json.loads(subprocess.check_output([str(helper), str(app.process.pid)]))
        path = app.output / (name + ".png")
        subprocess.run(["screencapture", "-x", "-o", "-l", str(info["window_id"]), str(path)], check=True, capture_output=True)
        rows = json.loads(subprocess.check_output(["target/robrix-ux-ocr", str(path)]))
        app.trace.append({"desktop_capture": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "rows": rows, "at": time.time()})
        return info, rows

    loaded_pages = 0
    def desktop_text(text, name):
        nonlocal loaded_pages
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if args.headless:
                ready = sum(request["path"] == "/ready" for request in requests)
                if ready > loaded_pages:
                    loaded_pages = ready
                    app.wait_text("Shared link:")
                    return desktop(name)
                time.sleep(.1)
                continue
            info, rows = desktop(name)
            if text in " ".join(row["text"] for row in rows):
                return info, rows
            time.sleep(.5)
        raise AssertionError(f"Text absent from native WebKit capture: {text}")

    def web_click(text):
        info, rows = desktop_text(text, "web-before-click")
        row = next(row for row in rows if text in row["text"])
        x, y, w, h = row["box"]
        point = [(x+w/2)*info["bounds"]["Width"], (y+h/2)*info["bounds"]["Height"]]
        subprocess.run([str(helper), str(app.process.pid), *map(str, point)], check=True, capture_output=True)
        app.trace.append({"web_native_click": text, "point": point, "at": time.time()})

    def recipient(chat, expected_title):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            events = checked(fixture["url"], "GET", f"rooms/{fixture['rooms'][chat]}/messages?dir=b&limit=25",
                             token=fixture["users"][chat]["access_token"])["chunk"]
            matches = [event for event in events if event["sender"] == fixture["users"]["alex"]["user_id"]
                       and event.get("content", {}).get("mini_app", {}).get("title") == expected_title]
            if matches:
                assert len(matches) == 1, "Duplicate card"
                content = matches[0]["content"]
                assert content["msgtype"] == "rs.robius.robrix.mini_app"
                assert content["mini_app"]["url"] == address and address in content["body"]
                return matches[0]["event_id"]
            time.sleep(.3)
        raise AssertionError("Recipient did not receive mini-app card")

    if args.headless:
        os.environ.pop("MAKEPAD_FOCUS", None)
        os.environ["MAKEPAD_HIDE_WINDOWS"] = "1"
        os.environ["MAKEPAD_NO_FOCUS"] = "1"
    else:
        os.environ["MAKEPAD_FOCUS"] = "1"
    try:
        app.start()
        if args.headless:
            script = '''import CoreGraphics
import Foundation
let pid = Int32(CommandLine.arguments[1])!
let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]] ?? []
let visible = windows.filter { ($0[kCGWindowOwnerPID as String] as? Int32) == pid }
print(visible.count)
'''
            count = int(subprocess.check_output(["swift", "-e", script, str(app.process.pid)]))
            assert count == 0, "Headless test unexpectedly exposed a window"
            result["checks"].append("owned_makepad_window_is_hidden")
        app.wait_text("Emma Wilson", timeout=60)
        row = next(w for w in app.snap() if w.get("t") == "Emma Wilson")
        x, y, w, h = row["r"]
        app.click(x+w/2, y+h/2)
        app.wait_text("Message (unencrypted)", pixels=True)
        app.click(385, 739)
        app.wait_text("Share mini app", pixels=True)
        app.click_text("Share mini app")
        app.wait_text("Web address")
        fill("mini_url", "javascript:alert(1)")
        app.click_id("mini_preview")
        app.wait_text("HTTP or HTTPS")
        assert not requests, "Invalid address opened a web view"
        result["checks"].append("unsafe_scheme_rejected_without_navigation")
        fill("mini_url", address)
        fill("mini_title", title)
        app.capture("mini-app-form")
        app.click_id("mini_preview")
        desktop_text("Rinx Web Mini App", "mini-app-webview")
        assert any(request["path"] == "/app" for request in requests)
        if args.headless:
            app.click_id("mini_reload")
            desktop_text("Rinx Web Mini App", "mini-app-reloaded")
            result["checks"].append("hidden_webkit_loads_http_and_executes_javascript_after_reload")
        else:
            result["checks"].append("http_page_visible_in_native_wkwebview")
            web_click("Test interaction")
            desktop_text("Interaction passed", "mini-app-interacted")
            assert any(request["path"] == "/clicked" for request in requests)
            web_click("Open second page")
            desktop_text("Second page", "mini-app-second-page")
            app.click_id("mini_web_back")
            desktop_text("Rinx Web Mini App", "mini-app-history-back")
            result["checks"].append("javascript_input_and_web_history_work")
        app.click_id("mini_share")
        app.wait_text("Web address")
        _, rows = desktop("mini-app-share-form")
        assert not any("rendered by WebKit" in row["text"] for row in rows), "Web view was not removed"
        app.click_id("mini_send")
        result["emma_event"] = recipient("emma", title)
        app.wait_text(title, pixels=True)
        app.capture("mini-app-sent-card")
        result["checks"].append("native_card_sent_once_and_rendered")
        app.click_text(title)
        desktop_text("Rinx Web Mini App", "mini-app-reopened")
        app.click_id("mini_share")
        app.click_id("mini_choose_chat")
        app.wait_text("Leo Zhang")
        app.click_text("Leo Zhang")
        app.wait_text("To: Leo Zhang")
        app.capture("mini-app-share-recipient")
        app.click_id("mini_send")
        result["leo_event"] = recipient("leo", title)
        result["checks"].append("existing_card_shared_to_second_chat_once")
        # Receive the same schema from a peer and open the incoming card.
        incoming = title + " incoming"
        checked(fixture["url"], "PUT", f"rooms/{fixture['rooms']['emma']}/send/m.room.message/mini-{app.output.name}",
                {"msgtype": "rs.robius.robrix.mini_app", "body": f"[Mini app] {incoming}\n{address}",
                 "mini_app": {"version": 1, "title": incoming, "url": address}}, fixture["users"]["emma"]["access_token"])
        app.wait_text(incoming, pixels=True)
        app.capture("mini-app-incoming-card")
        # Several older fixture cards can be visible; tap this run's newest
        # incoming title, whose wrapped second line is lowest in the viewport.
        row = max((row for row in app.ocr() if "incoming" in row["text"]), key=lambda row: row["box"][1])
        x, y, w, h = row["box"]
        width, height = app.request("/s")["w"][0]["sz"]
        app.click((x+w/2)*width, (y+h/2)*height)
        desktop_text("Rinx Web Mini App", "mini-app-incoming-opened")
        app.click_id("mini_close")
        app.wait_text("Message (unencrypted)", pixels=True)
        _, rows = desktop("mini-app-closed")
        assert not any("rendered by WebKit" in row["text"] for row in rows)
        result["checks"].append("incoming_card_opens_and_close_restores_chat")
        errors = [line for line in (app.output / "native.log").read_text().splitlines() if line.startswith("[E]")]
        assert not errors, "Native runtime errors"
        result["passed"] = True
        print(json.dumps({"passed": True, "checks": result["checks"], "evidence": str(app.output)}))
    except Exception as error:
        result["error"] = str(error)
        if app.process and app.process.poll() is None:
            app.capture("mini-app-failure")
        raise
    finally:
        app.stop()
        server.shutdown()
        result["http_requests"] = requests
        (app.output / "result.json").write_text(json.dumps(result, indent=2))
        receipt = "native-mini-apps-headless.json" if args.headless else "native-mini-apps.json"
        (live / receipt).write_text(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
