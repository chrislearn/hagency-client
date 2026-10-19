#!/usr/bin/env python3
"""Check mixed English/Chinese native composition and delivery on Apple fonts.

Use a dedicated profile so an interactive Palpo client can stay open. The Rust
font tests separately verify face selection, glyph coverage and fallback order.
"""
import json
import os
from pathlib import Path
import shutil
import time

from native_probe import NativeApp
from seed import checked


def main():
    live = Path("lab/wechat-ux/evidence/live")
    root = live / "font-check"
    root.mkdir(mode=0o700, exist_ok=True)
    shutil.copyfile(live / "fixture.json", root / "fixture.json")
    os.chmod(root / "fixture.json", 0o600)
    fixture = json.loads((root / "fixture.json").read_text())
    room = fixture["rooms"]["emma"]
    app = NativeApp(root)
    result = {"passed": False, "platform": "macOS", "evidence": str(app.output), "checks": []}
    body = "PingFang Hello Rinx\n中文聊天：你好，世界！\n繁體中文：訊息與聯絡人 👋\n" + app.output.name[:8]
    incoming = "English and 中文\n简体中文，繁體中文。🙂"
    try:
        app.start()
        app.wait_text("Emma Wilson", timeout=60)
        app.capture("pingfang-chats")
        app.click_id("contacts_tab")
        app.wait_text("Emma Wilson")
        app.capture("pingfang-contacts")
        app.click_id("me_tab")
        app.wait_text("Alex Chen")
        app.capture("pingfang-me")
        app.click_id("discover_tab")
        app.wait_text("Explore Groups")
        app.capture("pingfang-discover")
        app.click_id("chats_tab")
        row = next(w for w in app.snap() if w.get("t") == "Emma Wilson")
        x, y, w, h = row["r"]
        app.click(x + w / 2, y + h / 2)
        app.wait_text("Message (unencrypted)", pixels=True)
        app.click_text("Message (unencrypted)")
        app.request("/k", c="A", cmd=1, wait=1)
        app.request("/t", t=body, wait=1)
        app.capture("pingfang-composer-raw")
        app.wait_text("PingFang Hello Rinx", pixels=True)
        app.wait_text("你好", pixels=True)
        app.capture("pingfang-composer")
        result["checks"].append("mixed_english_chinese_composer_rendered")
        app.click_text("Send")
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            events = checked(fixture["url"], "GET", f"rooms/{room}/messages?dir=b&limit=20",
                             token=fixture["users"]["emma"]["access_token"])["chunk"]
            matches = [e for e in events if e["sender"] == fixture["users"]["alex"]["user_id"]
                       and e.get("content", {}).get("body") == body]
            if matches:
                assert len(matches) == 1, "Duplicate native send"
                result["sent_event_id"] = matches[0]["event_id"]
                break
            time.sleep(.3)
        else:
            raise AssertionError("Mixed-language message did not reach the recipient")
        result["checks"].append("exact_utf8_body_delivered_once")
        checked(fixture["url"], "PUT", f"rooms/{room}/send/m.room.message/font-{app.output.name}",
                {"msgtype": "m.text", "body": incoming, "format": "org.matrix.custom.html",
                 "formatted_body": "<b>English and 中文</b><br>简体中文，繁體中文。🙂"},
                fixture["users"]["emma"]["access_token"])
        app.wait_text("English and", pixels=True)
        app.wait_text("简体中文", pixels=True)
        app.capture("pingfang-conversation")
        result["checks"].append("incoming_bold_and_regular_mixed_text_rendered")
        log = (app.output / "native.log").read_text()
        assert "Apple typography: PingFang SC Regular/Semibold" in log, "PingFang was not installed"
        assert not any(line.startswith("[E]") for line in log.splitlines()), "Native runtime errors"
        result["checks"].append("pingfang_active_without_native_errors")
        result["passed"] = True
        print(json.dumps({"passed": True, "checks": result["checks"], "evidence": str(app.output)}))
    except Exception as error:
        result["error"] = str(error)
        raise
    finally:
        app.stop()
        (app.output / "result.json").write_text(json.dumps(result, indent=2, ensure_ascii=False))
        (live / "native-fonts.json").write_text(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
