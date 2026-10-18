#!/usr/bin/env python3
"""Repeated real-client navigation, draft preservation and two-way Matrix sends."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time
import uuid

from native_probe import NativeApp
from seed import checked


def open_emma(app):
    app.wait_text("Emma Wilson")
    row = next(w for w in app.snap() if w.get("t") == "Emma Wilson")
    app.click(row["r"][0] + 30, row["r"][1] + 6)


def wait_draft(app):
    # OCR confuses 0/O and 1/l in transaction suffixes. Check the composer
    # region for the stable prefix; exact full-text restoration is verified by
    # the recipient after Send, so an old message bubble cannot satisfy this.
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        if any("Draft" in row["text"] and row["box"][1] > .92 for row in app.ocr()):
            return
        time.sleep(.25)
    raise AssertionError("Draft is absent from the rendered composer")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("lab/wechat-ux/evidence/live"))
    parser.add_argument("--minutes", type=float, default=30)
    parser.add_argument("--port", type=int, default=8199)
    parser.add_argument("--chat-info", action="store_true", help="Also open/return from Chat Info with an unsent draft each cycle")
    args = parser.parse_args()
    fixture = json.loads((args.root / "fixture.json").read_text())
    run = uuid.uuid4().hex[:12]
    code_words = ["amber", "beach", "cedar", "delta", "eagle", "forest", "garden", "harbor", "island", "jasmine", "kite", "lemon", "maple", "north", "olive", "pearl"]
    spoken_run = " ".join(code_words[int(n, 16)] for n in run[:4])
    app = NativeApp(args.root, args.port)
    started = time.monotonic()
    count = 0
    passed = False
    output = args.root / f"native-soak-{run}.jsonl"
    with output.open("w") as log:
        def record(data):
            log.write(json.dumps(data, ensure_ascii=False) + "\n")
            log.flush()
        record({"kind": "start", "run": run, "minutes": args.minutes,
                "binary_sha256": hashlib.sha256(Path("target/debug/rinx").read_bytes()).hexdigest(), "at": time.time()})
        try:
            app.start()
            # A freshly restored session must render without a tab switch.
            app.wait_text("Emma Wilson")
            while time.monotonic() - started < args.minutes * 60:
                cycle_started = time.monotonic()
                for tab, expected in [("contacts_tab", "Emma Wilson"), ("me_tab", "Alex Chen"), ("discover_tab", "Explore Groups"), ("chats_tab", "Emma Wilson")]:
                    app.click_id(tab)
                    app.wait_text(expected)
                open_emma(app)
                # This dedicated fixture account may retain a draft from a
                # failed prior run. Reset it through native editing before use.
                # Capture waits for the pushed screen to draw before hit testing
                # its TextInput; geometry from the outgoing list is stale.
                rows = app.ocr()
                if any("Message (unencrypted)" in row["text"] for row in rows):
                    app.click_text("Message (unencrypted)")
                else:
                    app.click(125, 748)
                app.request("/k", c="A", cmd=1, wait=1)
                app.request("/k", c="Backspace", wait=1)
                body = f"Draft {run[:4]} {count}"
                app.request("/t", t=body, wait=1)
                app.trace.append({"input": "text", "text": body, "at": time.time()})
                wait_draft(app)
                if args.chat_info:
                    app.click(382, 55)
                    app.wait_text("Chat Info", pixels=True)
                    app.wait_text("2 members", pixels=True)
                    app.click(34, 54)
                    wait_draft(app)
                # Back out with an unsent draft; reopening must retain it.
                app.click(34, 54)
                open_emma(app)
                wait_draft(app)
                app.click_text("Send")
                room = fixture["rooms"]["emma"]
                deadline = time.monotonic() + 20
                while time.monotonic() < deadline:
                    events = checked(fixture["url"], "GET", f"rooms/{room}/messages?dir=b&limit=30", token=fixture["users"]["emma"]["access_token"])["chunk"]
                    matching = [event for event in events if event.get("content", {}).get("body") == body]
                    if matching:
                        assert len(matching) == 1, "Native send produced duplicate events"
                        break
                    time.sleep(.3)
                else:
                    raise AssertionError("Native send absent from recipient history")
                words = ["apple", "beach", "cedar", "delta", "eagle", "forest", "garden", "harbor", "island", "jasmine"]
                reply = "Reply " + spoken_run + " " + " ".join(words[int(n)] for n in str(count))
                assert not any(e.get("content", {}).get("body") == reply for e in events), "Reply marker is not unique"
                checked(fixture["url"], "PUT", f"rooms/{room}/send/m.room.message/native-{run}-{count}",
                        {"msgtype": "m.text", "body": reply}, fixture["users"]["emma"]["access_token"])
                app.wait_text(reply, pixels=True)
                if count % 5 == 0:
                    app.capture(f"native-soak-{run}-{count:03d}")
                rss_kib, cpu = subprocess.check_output(["ps", "-o", "rss=,%cpu=", "-p", str(app.process.pid)], text=True).split()
                record({"kind": "cycle", "index": count, "passed": True, "draft_retained": True,
                        "chat_info_draft_retained": True if args.chat_info else None,
                        "recipient_confirmed": True, "native_reply_visible": True,
                        "rss_kib": int(rss_kib), "cpu_percent": float(cpu), "seconds": time.monotonic() - cycle_started})
                count += 1
                if count % 5 == 0:
                    print(json.dumps({"cycles": count, "elapsed_seconds": round(time.monotonic()-started)}), flush=True)
                app.click(34, 54)
                app.wait_text("Emma Wilson")
            script_errors = [line for line in (app.output / "native.log").read_text(errors="replace").splitlines() if line.startswith("[E]")]
            assert not script_errors, "Native script errors: " + "\n".join(script_errors[:5])
            passed = count > 0
        except Exception as error:
            record({"kind": "failure", "error": str(error), "at": time.time()})
            try:
                app.capture(f"native-soak-{run}-failure")
            except Exception:
                pass
            raise
        finally:
            app.stop()
            (args.root / f"native-soak-{run}-trace.json").write_text(json.dumps(app.trace, indent=2))
            summary = {"kind": "summary", "passed": passed, "cycles": count, "elapsed_seconds": time.monotonic() - started}
            record(summary)
            print(json.dumps(summary), flush=True)


if __name__ == "__main__":
    main()
