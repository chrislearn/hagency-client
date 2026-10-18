#!/usr/bin/env python3
"""Focused native layout/link check; accepts only a disposable fixture profile."""
import argparse
import json
import os
import time
from pathlib import Path
from native_probe import NativeApp


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("profile_root", type=Path)
    args = parser.parse_args()
    root = args.profile_root
    fixture = json.loads((root / "fixture.json").read_text())
    assert fixture["users"]["alex"]["user_id"].startswith("@robrix_ux_")
    os.environ.update(MAKEPAD_HIDE_WINDOWS="1", MAKEPAD_NO_FOCUS="1")
    os.environ.pop("MAKEPAD_FOCUS", None)
    report = {"passed": False, "checks": [], "runs": []}
    language = root / "profile/ui-language.json"
    original_language = language.read_text()
    app = None
    try:
        language.write_text('"zh-CN"')
        app = NativeApp(root, 8363, size=(430, 820))
        app.start()
        report["runs"].append(str(app.output))
        app.wait_text("全部聊天", timeout=60)
        for widget in ["discover_tab", "discover_article", "article_continue", "article_allow"]:
            app.click_id(widget)
        app.click_text("把周末还给山野")
        app.click_id("article_theme")
        app.capture("theme-layout-zh")
        app.click_id("theme_done")
        app.click_id("rich")
        app.request("/k", c="A", cmd=1, wait=1)
        app.click_id("article_link")
        app.click_id("article_link_url")
        app.request("/k", c="A", cmd=1, wait=1)
        app.request("/t", t="https://example.com/article", wait=1)
        app.click_id("link_apply")
        app.wait_text("编辑文章")
        app.click_id("article_save")
        time.sleep(.5)
        library = json.loads(next((root / "profile").glob("mini-apps/**/library-v2.json")).read_text())
        assert any(mark.get("link") == "https://example.com/article" for block in library["documents"][0]["blocks"] for mark in block["marks"])
        app.capture("visual-link-editor-zh")
        report["checks"].append("native_selected_text_link_persists")
        app.stop()
        language.write_text('"en"')
        app = NativeApp(root, 8363, size=(1440, 960))
        app.start()
        report["runs"].append(str(app.output))
        app.wait_text("All Chats", timeout=60)
        app.click_text("Emma Wilson")
        for widget in ["open_popup_menu_button", "article_editor_button", "article_continue", "article_allow"]:
            app.click_id(widget)
        app.click_text("把周末还给山野")
        app.wait_text("Edit article")
        assert any(w["i"] == "inspector_theme" for w in app.snap())
        app.capture("desktop-editor-en")
        report["checks"].append("desktop_three_columns_after_polish")
        report["passed"] = True
    finally:
        if app:
            app.stop()
        language.write_text(original_language)
        (root / "layout-result.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report), flush=True)


if __name__ == "__main__":
    main()
