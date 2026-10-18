#!/usr/bin/env python3
"""Optional Blitz preview in the real Rinx UI, using a disposable Palpo user.

Requires a binary built with agent_chat,html_preview. Seeds an already normalized
test illustration; the OS file picker and arbitrary HTML import are not tested.
"""
import json
import os
from pathlib import Path
import shutil
import socket
import time
import uuid

from native_probe import NativeApp


def main():
    os.environ.update(MAKEPAD_HIDE_WINDOWS="1", MAKEPAD_NO_FOCUS="1")
    os.environ.pop("MAKEPAD_FOCUS", None)
    fixture_path = Path("lab/wechat-ux/evidence/live/fixture.json")
    fixture = json.loads(fixture_path.read_text())
    assert all(u["user_id"].startswith("@robrix_ux_") for u in fixture["users"].values())
    root = fixture_path.parent / "article-blitz" / uuid.uuid4().hex
    root.mkdir(parents=True, mode=0o700)
    shutil.copyfile(fixture_path, root / "fixture.json")
    os.chmod(root / "fixture.json", 0o600)
    (root / "profile").mkdir(mode=0o700)
    (root / "profile/ui-language.json").write_text('"zh-CN"')
    report = {"passed": False, "checks": [], "runs": [], "system_file_picker_tested": False}
    app = None

    def free_port():
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            return listener.getsockname()[1]

    def mark(name):
        report["checks"].append(name)
        print("PASS " + name, flush=True)

    def fill(widget, text):
        app.click_id(widget)
        app.request("/k", c="A", cmd=1, wait=1)
        app.request("/k", c="Backspace", wait=1)
        app.request("/t", t=text, wait=1)

    def open_editor():
        if any(w["i"] == "discover_tab" for w in app.snap()):
            app.click_id("discover_tab")
            app.click_id("discover_article")
        else:
            app.click_text("Emma Wilson")
            app.click_id("open_popup_menu_button")
            app.click_id("article_editor_button")
        app.click_id("article_continue")
        app.click_id("article_allow")

    def source(text):
        app.click_id("article_source")
        fill("article_markdown", text)
        app.click_id("source_apply")

    try:
        app = NativeApp(root, free_port(), size=(430, 820))
        app.start()
        report["runs"].append(str(app.output))
        app.wait_text("全部聊天", timeout=90)
        open_editor()
        app.click_id("article_new")
        fill("article_title", "山野来信 · 原生排版")
        fill("article_author", "林间工作室")
        body = "## 中文与 English\n\n**晨光穿过松林。** 在山路与晨光之间，找回生活的节奏。\n\n> 慢一点，才能看见更多。\n\n" + "\n\n".join(
            f"第 {i} 段。风穿过松林，光落在石阶。This article is rendered with native HTML and CSS."
            for i in range(1, 10)
        ) + "\n\n全文结束 END OF ARTICLE"
        source(body)
        app.click_id("article_save")
        time.sleep(.5)
        file = next((root / "profile").glob("mini-apps/**/library-v2.json"))
        library = json.loads(file.read_text())
        asset = json.loads(Path("lab/article-editor-v2/artwork/mountains.json").read_text())
        library["assets"][asset["id"]] = asset
        (file.parent / "assets").mkdir(exist_ok=True)
        shutil.copyfile("lab/article-editor-v2/artwork/mountains.png", file.parent / "assets" / asset["id"])
        os.chmod(file.parent / "assets" / asset["id"], 0o600)
        file.write_text(json.dumps(library))
        app.click_id("article_images")
        app.click_text("晨光里的山谷")
        app.click_id("article_theme")
        app.click_text("暖纸色")
        app.click_id("theme_done")
        app.click_id("article_cover")
        app.click_id("cover_pick")
        app.click_text("晨光里的山谷")
        app.click_id("article_done")
        app.click_id("article_save")
        app.click_id("article_preview")
        app.click_id("css_preview_open")
        app.wait_text("HTML/CSS 排版预览已就绪", timeout=60)
        app.wait_text("轻点链接即可打开")
        app.wait_text("山野来信", pixels=True)
        app.capture("mobile-top")
        app.request("/m", k="scroll", x=210, y=490, dy=4000, wait=1)
        app.wait_text("END OF ARTICLE", pixels=True)
        app.capture("mobile-bottom")
        mark("mobile_chinese_english_css_theme_authorized_images_and_full_scroll")
        app.click_id("article_back")
        app.wait_text("全文预览")
        app.click_id("article_back")
        source("\n\n".join("Native long article paragraph %d. " % i + "Readable text and layout. " * 4 for i in range(160)))
        app.click_id("article_preview")
        app.click_id("css_preview_open")
        app.wait_text("预览已达到长度限制", timeout=60)
        app.capture("mobile-clipped")
        app.click_id("article_back")
        app.wait_text("全文预览")
        mark("long_preview_reports_clipping_and_returns_to_complete_native_reader")
        app.click_id("article_back")
        source(body)
        app.click_id("article_save")
        app.stop()
        app = None

        (root / "profile/ui-language.json").write_text('"en"')
        app = NativeApp(root, free_port(), size=(1440, 960))
        app.start()
        report["runs"].append(str(app.output))
        app.wait_text("All Chats", timeout=90)
        open_editor()
        app.click_text("山野来信")
        app.click_id("article_preview")
        app.click_id("css_preview_open")
        app.wait_text("HTML/CSS preview ready", timeout=60)
        app.wait_text("山野来信", pixels=True)
        app.capture("desktop-top")
        app.click_id("css_preview_refresh")
        app.wait_text("HTML/CSS preview ready", timeout=60)
        app.click_id("css_preview_refresh")
        app.click_id("article_back")
        app.wait_text("Full preview")
        time.sleep(.5)
        assert not any(w["i"] == "css_preview_refresh" for w in app.snap())
        app.click_id("css_preview_open")
        app.wait_text("HTML/CSS preview ready", timeout=60)
        app.wait_text("Tap links to open them")
        mark("refresh_back_discards_old_session_and_reopen_renders")
        app.click_id("article_back")
        app.wait_text("Full preview")
        app.click_id("article_close")
        assert not any(w["i"] == "css_preview_refresh" for w in app.snap())
        mark("desktop_english_preview_refresh_back_and_close")
        report["passed"] = True
    finally:
        if app:
            if not report["passed"] and app.process and app.process.poll() is None:
                try:
                    app.capture("failure")
                    (app.output / "failure-widgets.json").write_text(json.dumps(app.request("/snap"), ensure_ascii=False))
                except Exception:
                    pass
            app.stop()
        (root / "result.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps({"passed": report["passed"], "evidence": str(root), "checks": report["checks"]}), flush=True)


if __name__ == "__main__":
    main()
