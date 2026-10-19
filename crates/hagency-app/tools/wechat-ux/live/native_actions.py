#!/usr/bin/env python3
"""Exercise native replies, edits and reactions, verified by the recipient."""
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
    sender = fixture["users"]["alex"]["user_id"]
    app = NativeApp(root)
    result = {"passed": False, "evidence": str(app.output), "checks": []}
    words = ["amber", "beach", "cedar", "delta", "eagle", "forest", "garden", "harbor", "island", "jasmine", "kite", "lemon", "maple", "north", "olive", "pearl"]
    suffix = " ".join(words[int(c, 16)] for c in uuid.uuid4().hex[:5])

    def text(value):
        app.trace.append({"input": "text", "text": value, "at": time.time()})
        app.request("/t", t=value, wait=1)

    def key(value, **kwargs):
        app.trace.append({"input": "key", "key": value, **kwargs, "at": time.time()})
        app.request("/k", c=value, **kwargs, wait=1)

    def received(name, predicate):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            events = checked(fixture["url"], "GET", f"rooms/{room}/messages?dir=b&limit=30", token=fixture["users"]["emma"]["access_token"])["chunk"]
            matches = [e for e in events if e["sender"] == sender and predicate(e)]
            if matches:
                assert len(matches) == 1, f"Duplicate effect: {name}"
                result["checks"].append({"name": name, "passed": True, "recipient_event": matches[0], "duplicate_events": 0})
                return matches[0]
            time.sleep(.3)
        raise AssertionError(f"Recipient did not receive {name}")

    def context(prefix):
        app.wait_text(prefix, pixels=True)
        row = max((r for r in app.ocr() if prefix in r["text"]), key=lambda r: r["box"][1])
        x, y, w, h = row["box"]
        app.trace.append({"input": "secondary_click", "x": (x+w/2)*406, "y": (y+h/2)*776, "at": time.time()})
        app.request("/click", x=(x+w/2)*406, y=(y+h/2)*776, b=1, wait=1)
        app.wait_text("Reply")

    try:
        app.start()
        app.wait_text("Emma Wilson")
        app.click_text("Emma Wilson")
        if any("Editing:" in row["text"] for row in app.ocr()):
            app.click(150, 735)
            key("Escape")
        app.wait_text("Message (unencrypted)", pixels=True)
        app.click_text("Message (unencrypted)")
        anchor_body = "Action anchor " + suffix
        text(anchor_body)
        app.click_text("Send")
        anchor = received("native_anchor", lambda e: e.get("content", {}).get("body") == anchor_body)
        context("Action anchor")
        app.capture("actions-menu-native")
        app.click_id("reply_button")
        app.wait_text("Replying to:", pixels=True)
        app.capture("actions-reply-composer-native")
        app.click(100, 748)
        reply_body = "Action reply " + suffix
        text(reply_body)
        app.click_text("Send")
        reply = received("native_reply", lambda e: e.get("content", {}).get("body", "").endswith(reply_body) and e["content"].get("m.relates_to", {}).get("m.in_reply_to", {}).get("event_id") == anchor["event_id"])
        context("Action reply")
        app.click_id("edit_message_button")
        app.wait_text("Editing:", pixels=True)
        # Focus must be ready on the first draw, without an extra input tap.
        key("A", cmd=1)
        edited_body = "Action revised " + suffix
        text(edited_body)
        app.wait_text("Action revised", pixels=True)
        app.capture("actions-edit-native")
        key("Return")
        received("native_edit", lambda e: e.get("content", {}).get("m.new_content", {}).get("body") == edited_body and e["content"].get("m.relates_to", {}).get("event_id") == reply["event_id"] and e["content"]["m.relates_to"].get("rel_type") == "m.replace")
        context("Action revised")
        app.click_id("react_button")
        key("Return")
        app.wait_text("Enter reaction", pixels=True)
        result["checks"].append({"name": "empty_reaction_keeps_editor_open", "passed": True})
        text("👍")
        app.capture("actions-reaction-native")
        app.click_id("reaction_send_button")
        received("native_reaction", lambda e: e.get("type") == "m.reaction" and e["content"].get("m.relates_to", {}).get("event_id") == reply["event_id"] and e["content"]["m.relates_to"].get("key") == "👍")
        app.wait_text("edited", pixels=True)
        labels = [r for r in app.ocr() if r["text"].strip() == "edited"]
        assert labels and all(r["box"][0] >= 0 and r["box"][0]+r["box"][2] <= 1 for r in labels), "Edited indicator is clipped"
        result["checks"].append({"name": "mobile_edited_indicator_visible", "passed": True, "rendered_labels": labels})
        app.capture("actions-complete-native")
        errors = [line for line in (app.output / "native.log").read_text().splitlines() if line.startswith("[E]")]
        assert not errors, "Native script errors"
        result["passed"] = True
        print(json.dumps({"passed": True, "checks": len(result["checks"])}))
    except Exception as error:
        result["error"] = str(error)
        raise
    finally:
        app.stop()
        (app.output / "result.json").write_text(json.dumps(result, indent=2))
        (root / "native-actions.json").write_text(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
