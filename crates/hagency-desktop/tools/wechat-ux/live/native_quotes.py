#!/usr/bin/env python3
"""Check native long-quote layout, expansion and navigation to its Matrix event."""
import json
from pathlib import Path
import time
import uuid

from native_probe import NativeApp
from seed import checked


def main():
    root = Path("lab/wechat-ux/evidence/live")
    fixture = json.loads((root / "fixture.json").read_text())
    room = fixture["rooms"]["emma"]
    app = NativeApp(root)
    result = {"passed": False, "evidence": str(app.output), "checks": []}

    def send(user, content):
        return checked(fixture["url"], "PUT", f"rooms/{room}/send/m.room.message/{uuid.uuid4().hex}", content,
                       token=fixture["users"][user]["access_token"])["event_id"]

    def reveal(text):
        # Expanding a quote preserves the current scroll anchor. Its end may
        # therefore extend below the composer until the user scrolls down.
        for _ in range(8):
            if any(text in r["text"] for r in app.ocr()):
                return
            app.request("/m", k="scroll", x=260, y=530, dy=120, wait=1)
            time.sleep(.3)
        app.wait_text(text, pixels=True)

    try:
        original = send("emma", {"msgtype": "m.text", "body": "Quote origin begins. " +
            "The meeting starts tomorrow morning. Please bring the revised design, review the notes and confirm the new schedule. " * 5 + "Quote origin ends."})
        for index in range(12):
            send("emma", {"msgtype": "m.text", "body": f"Timeline separation message {index + 1}."})
        reply = send("alex", {"msgtype": "m.text", "body": "Short quoted reply", "m.relates_to": {"m.in_reply_to": {"event_id": original}}})
        result["fixture_events"] = {"original": original, "reply": reply}
        app.start()
        app.wait_text("Emma Wilson")
        app.click_text("Emma Wilson")
        app.wait_text("Short quoted reply", pixels=True)
        app.wait_text("Show more", pixels=True)
        rows = app.ocr()
        bubble = next(r for r in rows if "Short quoted reply" in r["text"])
        quote = next(r for r in rows if "Quote origin begins" in r["text"])
        assert bubble["box"][1] < quote["box"][1], "Mobile quote should follow its reply bubble"
        assert not any("Quote origin ends" in r["text"] for r in rows), "Long quote was not collapsed"
        app.capture("quote-collapsed-native")
        result["checks"].append({"name": "quote_below_bubble_and_collapsed", "passed": True})
        app.click_text("Show more")
        reveal("Show less")
        app.wait_text("Quote origin ends", pixels=True)
        app.capture("quote-expanded-native")
        result["checks"].append({"name": "long_quote_expands", "passed": True})
        app.click_text("Show less")
        app.wait_text("Show more", pixels=True)
        app.click_text("Quote origin begins")
        reveal("Show less")
        result["checks"].append({"name": "collapsed_quote_tap_expands", "passed": True})
        app.click_text("Quote origin begins")
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            rows = app.ocr()
            if any("Quote origin begins" in r["text"] for r in rows) and not any("Short quoted reply" in r["text"] for r in rows):
                break
            time.sleep(.3)
        else:
            raise AssertionError("Expanded quote did not navigate to its original event")
        app.capture("quote-original-native")
        result["checks"].append({"name": "expanded_quote_tap_jumps_to_original", "passed": True})
        errors = [line for line in (app.output / "native.log").read_text().splitlines() if line.startswith("[E]")]
        assert not errors, "Native script errors"
        result["passed"] = True
        print(json.dumps({"passed": True, "checks": len(result["checks"])}))
    except Exception as error:
        result["error"] = str(error)
        raise
    finally:
        app.stop()
        app.output.mkdir(parents=True, exist_ok=True)
        (app.output / "result.json").write_text(json.dumps(result, indent=2))
        (root / "native-quotes.json").write_text(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
