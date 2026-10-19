#!/usr/bin/env python3
"""Cut only an owned local proxy, then verify a queued native send on Palpo."""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import threading
import time
import urllib.error
import urllib.request
import uuid

from native_probe import NativeApp
from native_soak import open_emma
from seed import checked


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("lab/wechat-ux/evidence/live"))
    parser.add_argument("--port", type=int, default=8299)
    args = parser.parse_args()
    fixture = json.loads((args.root / "fixture.json").read_text())
    upstream = fixture["url"].rstrip("/")
    disconnected = threading.Event()
    requests = []

    class Proxy(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass  # Auth-bearing requests must never go to the terminal.

        def do_request(self):
            requests.append({"method": self.command, "path": self.path.split("?")[0], "blocked": disconnected.is_set(), "at": time.time()})
            if disconnected.is_set():
                status, body, content_type = 503, b'{"errcode":"M_UNKNOWN","error":"Isolated reconnect test"}', "application/json"
            else:
                body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
                headers = {key: value for key, value in self.headers.items() if key.lower() not in {"host", "connection", "content-length"}}
                req = urllib.request.Request(upstream + self.path, data=body if body else None, headers=headers, method=self.command)
                try:
                    with urllib.request.urlopen(req, timeout=65) as response:
                        status, body, content_type = response.status, response.read(), response.headers.get("Content-Type", "application/json")
                except urllib.error.HTTPError as error:
                    status, body, content_type = error.code, error.read(), error.headers.get("Content-Type", "application/json")
                    error.close()
                except OSError:
                    status, body, content_type = 502, b'{"errcode":"M_UNKNOWN"}', "application/json"
                # Matrix discovery and login may advertise the real upstream
                # base URL. Keep this isolated client's subsequent requests on
                # the proxy; otherwise the interruption would test nothing.
                if "json" in content_type and (".well-known/matrix/client" in self.path or self.path.rstrip("/").endswith("/login")):
                    payload = json.loads(body)
                    discovery = payload.get("well_known", payload)
                    if "m.homeserver" in discovery:
                        discovery["m.homeserver"]["base_url"] = f"http://127.0.0.1:{self.server.server_port}"
                        body = json.dumps(payload).encode()
            try:
                self.send_response(status)
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            except (BrokenPipeError, ConnectionResetError):
                pass

        do_GET = do_POST = do_PUT = do_DELETE = do_OPTIONS = do_request

    proxy = ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
    proxy.daemon_threads = True
    thread = threading.Thread(target=proxy.serve_forever, daemon=True)
    thread.start()
    run_root = args.root / ("reconnect-" + uuid.uuid4().hex)
    run_root.mkdir(mode=0o700)
    fixture["url"] = f"http://127.0.0.1:{proxy.server_port}"
    fixture_path = run_root / "fixture.json"
    fixture_path.write_text(json.dumps(fixture))
    os.chmod(fixture_path, 0o600)
    app = NativeApp(run_root, args.port)
    result = {"passed": False, "scope": "HTTP transport interruption of isolated fixture client"}
    try:
        app.start()
        open_emma(app)
        app.wait_text("Message (unencrypted)", pixels=True)
        app.capture("before-disconnect")
        disconnected.set()
        # New requests now fail; no remote service or existing tunnel is stopped.
        body = "Offline message " + uuid.uuid4().hex[:6]
        app.click_text("Message (unencrypted)")
        app.request("/t", t=body, wait=1)
        app.click_text("Send")
        app.wait_text("Offline message", pixels=True)
        room = fixture["rooms"]["emma"]

        def received():
            events = checked(upstream, "GET", f"rooms/{room}/messages?dir=b&limit=30", token=fixture["users"]["emma"]["access_token"])["chunk"]
            return [e for e in events if e.get("content", {}).get("body") == body]

        time.sleep(5)
        assert not received(), "Send unexpectedly bypassed the disconnected proxy"
        assert any(r["blocked"] and "/send/m.room.message/" in r["path"] for r in requests), "Native send never reached the interrupted transport"
        app.capture("disconnected-pending")
        disconnected.clear()
        restored = time.monotonic()
        deadline = restored + 120
        while time.monotonic() < deadline:
            if received():
                break
            time.sleep(1)
        else:
            raise AssertionError("Queued native send did not recover within 120 seconds")
        assert len(received()) == 1, "Reconnect duplicated a native send"
        app.capture("reconnected-delivered")
        result.update(passed=True, duplicate_events=0, recovery_seconds=time.monotonic()-restored)
        app.trace.append({"assert": "transport_reconnect_delivered_exact_native_message_once", "passed": True, "at": time.time()})
        print(json.dumps(result), flush=True)
    except Exception as error:
        result["error"] = str(error)
        try:
            app.capture("reconnect-failure")
        except Exception:
            pass
        raise
    finally:
        disconnected.clear()
        app.stop()
        (run_root / "result.json").write_text(json.dumps(result, indent=2))
        (run_root / "transport-trace.json").write_text(json.dumps(requests, indent=2))
        proxy.shutdown()
        proxy.server_close()


if __name__ == "__main__":
    main()
