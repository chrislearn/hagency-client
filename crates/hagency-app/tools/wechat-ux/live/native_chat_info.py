#!/usr/bin/env python3
"""Exercise mobile Chat Info over Matrix while preserving a native draft."""
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
    token = fixture["users"]["alex"]["access_token"]
    app = NativeApp(root)
    result = {"passed": False, "evidence": str(app.output), "checks": []}

    def open_chat(name="Emma Wilson"):
        app.wait_text(name)
        row = next(w for w in app.snap() if w.get("t") == name)
        x, y, w, h = row["r"]
        app.click(x + w / 2, y + h / 2)

    def muted():
        rules = checked(fixture["url"], "GET", "pushrules/", token=token)["global"]
        return any(r["rule_id"] == room and "notify" not in r["actions"] for r in rules.get("override", []))

    def wait_mode(value):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if muted() == value:
                return
            time.sleep(.25)
        raise AssertionError("Native notification choice did not reach Matrix")

    try:
        assert not muted(), "Fixture must start with account-default notifications"
        result["scroll_fixture_events"] = []
        for word in ("amber", "beach", "cedar", "delta", "eagle", "forest", "garden", "harbor", "island", "jasmine", "kite", "lemon"):
            event = checked(fixture["url"], "PUT", f"rooms/{room}/send/m.room.message/{uuid.uuid4().hex}",
                            {"msgtype": "m.text", "body": "Info scroll anchor " + word},
                            token=fixture["users"]["emma"]["access_token"])
            result["scroll_fixture_events"].append(event["event_id"])
        app.start()
        open_chat()
        app.ocr()  # Wait for the pushed room to draw before editing the fixture draft.
        app.click(125, 748)
        app.request("/k", c="A", cmd=1, wait=1)
        app.request("/k", c="Backspace", wait=1)
        app.wait_text("Message (unencrypted)", pixels=True)
        app.click_text("Message (unencrypted)")
        app.request("/t", t="Draft survives chat details", wait=1)
        app.request("/m", k="scroll", x=260, y=460, dy=-180, wait=1)
        time.sleep(.5)
        anchors = [r for r in app.ocr() if r["text"].startswith("Info scroll anchor ") and .2 < r["box"][1] < .7]
        assert anchors, "Seeded timeline anchors are absent"
        anchor = min(anchors, key=lambda r: abs(r["box"][1] - .45))
        app.capture("chat-info-before-native")
        app.click(382, 55)
        app.wait_text("Chat Info", pixels=True)
        app.wait_text("2 members", pixels=True)
        app.wait_text("Alex Chen", pixels=True)
        app.wait_text("Emma Wilson", pixels=True)
        app.capture("chat-info-native")
        result["checks"].append({"name": "matrix_members_rendered", "passed": True})
        app.click_text("Invite to Chat")
        app.wait_text("Cancel", pixels=True)
        app.capture("chat-info-invite-native")
        app.click_text("Cancel")
        app.wait_text("Chat Info", pixels=True)
        result["checks"].append({"name": "existing_invite_flow_and_cancel", "passed": True})
        app.click_text("Notifications")
        app.wait_text("Mute Notifications", pixels=True)
        app.click_text("Mute Notifications")
        wait_mode(True)
        app.click_text("Back to Chat Info")
        app.wait_text("Muted", pixels=True)
        app.capture("chat-info-muted-native")
        result["checks"].append({"name": "native_mute_confirmed_by_matrix", "passed": True})
        app.click_text("Notifications")
        app.click_text("Use Account Defaults")
        wait_mode(False)
        app.click_text("Back to Chat Info")
        app.wait_text("Default", pixels=True)
        result["checks"].append({"name": "native_default_confirmed_by_matrix", "passed": True})
        app.click(34, 55)
        app.wait_text("Draft survives chat details", pixels=True)
        app.capture("chat-info-return-native")
        result["checks"].append({"name": "back_preserves_native_draft", "passed": True})
        restored = next((r for r in app.ocr() if r["text"] == anchor["text"]), None)
        assert restored and abs(restored["box"][1] - anchor["box"][1]) * 776 <= 2, "Chat Info return changed the scrolled timeline anchor"
        result["checks"].append({"name": "back_preserves_scrolled_timeline_anchor", "passed": True,
                                 "before": anchor, "after": restored})
        app.click_text("Draft survives chat details")
        app.request("/k", c="A", cmd=1, wait=1)
        app.request("/k", c="Backspace", wait=1)
        app.click(34, 55)
        app.wait_text("Contacts", pixels=True)
        open_chat()
        app.wait_text("Message (unencrypted)", pixels=True)
        app.click(382, 55)
        app.wait_text("2 members", pixels=True)
        result["checks"].append({"name": "chat_info_reopens_after_back_to_roots", "passed": True})
        app.click(34, 55)
        app.wait_text("Message (unencrypted)", pixels=True)
        app.click(34, 55)
        app.wait_text("Contacts", pixels=True)
        open_chat("Weekend Plans")
        app.wait_text("Message (unencrypted)", pixels=True)
        app.click(382, 55)
        app.wait_text("4 members", pixels=True)
        app.wait_text("Leo Zhang", pixels=True)
        app.wait_text("Nora Patel", pixels=True)
        app.capture("chat-info-group-native")
        result["checks"].append({"name": "group_members_replace_previous_direct_room", "passed": True})
        log = (app.output / "native.log").read_text()
        errors = [line for line in log.splitlines() if line.startswith("[E]") or "Unable to delete override push rule" in line]
        assert not errors, "Native script errors or duplicate notification writes"
        result["passed"] = True
        print(json.dumps({"passed": True, "checks": len(result["checks"])}))
    except Exception as error:
        result["error"] = str(error)
        raise
    finally:
        app.stop()
        app.output.mkdir(parents=True, exist_ok=True)
        (app.output / "result.json").write_text(json.dumps(result, indent=2))
        (root / "native-chat-info.json").write_text(json.dumps(result, indent=2))
        if muted():
            checked(fixture["url"], "DELETE", f"pushrules/global/override/{room}", token=token)


if __name__ == "__main__":
    main()
