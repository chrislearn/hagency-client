#!/usr/bin/env python3
"""Native Explore regression checks with isolated Palpo accounts only.

The public test room is created by fixture Emma; fixture Alex previews it without
joining. The personal Matrix account and its UI are never automated.
"""
import json
import os
from pathlib import Path
import shutil
import time
from native_probe import NativeApp
from seed import checked


def main():
    os.environ.update(MAKEPAD_HIDE_WINDOWS="1", MAKEPAD_NO_FOCUS="1")
    os.environ.pop("MAKEPAD_FOCUS", None)
    source = Path("lab/wechat-ux/evidence/live")
    root = source / "explore-navigation"
    root.mkdir(mode=0o700, exist_ok=True)
    shutil.copyfile(source / "fixture.json", root / "fixture.json")
    os.chmod(root / "fixture.json", 0o600)
    fixture = json.loads((root / "fixture.json").read_text())
    alex = fixture["users"]["alex"]
    emma = fixture["users"]["emma"]
    state = root / "directory-room.json"
    alias = "robrix-ux-explore-" + emma["user_id"].split("_")[-1].split(":")[0]
    if not state.exists():
        room = checked(fixture["url"], "POST", "createRoom", {
            "name": "Rinx Garden 设计", "topic": "Isolated room-name search fixture.",
            "preset": "public_chat", "visibility": "public", "room_alias_name": alias,
        }, token=emma["access_token"])
        state.write_text(json.dumps(room))
        os.chmod(state, 0o600)
    room_id = json.loads(state.read_text())["room_id"]
    membership_before = checked(fixture["url"], "GET", "joined_rooms", token=alex["access_token"])["joined_rooms"]
    assert room_id not in membership_before, "Fixture must start with the search room unjoined"
    app = NativeApp(root, port=8299, size=(375, 812))
    result = {"passed": False, "checks": [], "evidence": str(app.output)}

    def passed(name):
        result["checks"].append(name)
        print("PASS", name, flush=True)

    def search(value):
        app.click_id("room_alias_id_input")
        app.request("/k", c="A", cmd=1, wait=1)
        app.request("/t", t=value, wait=1)
        app.click_id("search_for_room_button")

    def visible(text):
        app.wait_text(text, pixels=True, timeout=60)

    try:
        app.start()
        app.wait_text("Emma Wilson", timeout=90)
        app.click_text("Discover")
        app.click_id("explore")
        visible("Explore Rooms")
        app.capture("explore")
        app.click_id("back")
        visible("Explore Groups")
        passed("back_returns_to_discover")
        app.click_id("explore")
        app.click_id("room_alias_id_input")
        app.request("/k", c="Escape", wait=1)
        visible("Explore Groups")
        passed("escape_returns_even_with_search_focused")
        app.click_id("explore")
        search("Garden")
        visible("Rinx Garden")
        app.capture("name-results")
        passed("partial_name_search_on_custom_homeserver")
        app.click_text("Rinx Garden")
        visible("Join this room")
        app.capture("room-preview")
        app.click_id("back")
        visible("result")
        visible("Rinx Garden")
        passed("preview_back_preserves_results")
        search("设计")
        visible("Rinx Garden")
        passed("chinese_name_search")
        search("design")
        visible("Design Studio")
        visible("Joined")
        app.capture("joined-results")
        passed("private_joined_room_name_search")
        search("zxq-no-such-room-70852")
        visible("No rooms found")
        app.capture("no-results")
        passed("empty_result_explains_unlisted_rooms")
        search("https://example.com/not-a-matrix-room")
        visible("This room address is invalid")
        passed("invalid_link_has_address_error")
        search("#" + alias + ":" + emma["user_id"].split(":", 1)[1])
        visible("Join this room")
        passed("full_room_alias_still_resolves")
        # A newly submitted query must win over any earlier directory response.
        search("Garden")
        search("design")
        visible("Design Studio")
        time.sleep(1)
        assert not any("Rinx Garden" in row["text"] for row in app.ocr())
        passed("replacement_search_keeps_latest_results")
        app.click_id("back")
        visible("Explore Groups")
        membership_after = checked(fixture["url"], "GET", "joined_rooms", token=alex["access_token"])["joined_rooms"]
        assert set(membership_before) == set(membership_after)
        passed("search_and_preview_never_auto_join")
        result["passed"] = True
    finally:
        app.stop()
        native_log = app.output / "native.log"
        result["native_errors"] = sum(any(marker in line for marker in ("[E]", "panicked at", "Assertion failed:"))
            for line in native_log.read_text(errors="replace").splitlines()) if native_log.exists() else None
        result["passed"] = result["passed"] and result["native_errors"] == 0
        (root / "native-explore.json").write_text(json.dumps(result, indent=2))
    assert result["passed"], "Native errors detected"


if __name__ == "__main__":
    main()
