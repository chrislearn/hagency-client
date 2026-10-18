#!/usr/bin/env python3
"""Verify push-rule updates from another session and after a native restart."""
import json
from pathlib import Path
import time
import urllib.parse

from native_probe import NativeApp
from seed import checked, api


def open_choices(app):
    app.wait_text("Emma Wilson")
    row = next(w for w in app.snap() if w.get("t") == "Emma Wilson")
    app.request("/click", x=row["r"][0]+30, y=row["r"][1]+6, b=1, wait=1)
    app.wait_text("Notifications")
    app.click_id("notifications_button")


def assert_mode(app, expected, escape=False):
    # Account-data changes may arrive at the end of an outstanding sync poll.
    app.wait_text("✓ " + expected, timeout=40)
    app.capture("notifications-current-mode")
    if escape:
        app.request("/k", c="Escape", wait=1)
        deadline = time.monotonic() + 5
        while any(w["i"] == "notify_mute" for w in app.snap()):
            assert time.monotonic() < deadline, "Escape did not dismiss the room menu"
            time.sleep(.1)
    else:
        app.click(10, 100)  # Tap the modal backdrop, as on a touch device.


def main():
    root = Path("lab/wechat-ux/evidence/live")
    fixture = json.loads((root / "fixture.json").read_text())
    room = fixture["rooms"]["emma"]
    token = fixture["users"]["alex"]["access_token"]
    path = "pushrules/global/override/" + urllib.parse.quote(room, safe="")
    results = []
    try:
        app = NativeApp(root)
        try:
            app.start()
            open_choices(app)
            # A previous interrupted run may have cached "Mute" even after
            # cleanup. Require the synced baseline before testing a transition.
            app.wait_text("✓ Use Account Defaults", timeout=40)
            checked(fixture["url"], "PUT", path, {"conditions": [{"kind": "event_match", "key": "room_id", "pattern": room}], "actions": []}, token)
            assert_mode(app, "Mute Notifications", escape=True)
            app.capture("muted-chat")
            results.append({"check": "other_session_mute_received", "passed": True, "evidence": str(app.output)})
        finally:
            app.stop()
        app = NativeApp(root)
        try:
            app.start()
            open_choices(app)
            assert_mode(app, "Mute Notifications")
            results.append({"check": "muted_mode_restored_after_restart", "passed": True, "evidence": str(app.output)})
            open_choices(app)
            checked(fixture["url"], "DELETE", path, token=token)
            assert_mode(app, "Use Account Defaults")
            app.capture("unmuted-chat")
            results.append({"check": "other_session_reset_received", "passed": True, "evidence": str(app.output)})
        finally:
            app.stop()
        print(json.dumps({"passed": True, "checks": len(results)}))
    finally:
        # Remove only the test-created room override, including on failure.
        api(fixture["url"], "DELETE", path, token=token)
        (root / "native-notifications.json").write_text(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
