#!/usr/bin/env python3
"""Native article selection regressions, using an isolated local-only profile."""
import argparse
import copy
import json
import os
from pathlib import Path
import socket
import subprocess
import time
import uuid
from native_probe import NativeApp


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args()
    root = Path("target/article-selection-regressions") / uuid.uuid4().hex
    root.mkdir(parents=True)
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    app = NativeApp(root, port, auto_login=False)
    app.output.mkdir(parents=True)
    app.log = (app.output / "native.log").open("w")
    env = dict(os.environ, MAKEPAD_REMOTE=str(port), MAKEPAD_HIDE_WINDOWS="1", MAKEPAD_NO_FOCUS="1")
    app.process = subprocess.Popen([str(args.binary.resolve()), "--data-dir=" + str((root / "profile").resolve())],
                                   env=env, stdin=subprocess.DEVNULL, stdout=app.log, stderr=subprocess.STDOUT)
    report = {"passed": False, "checks": []}
    try:
        for _ in range(80):
            if app.process.poll() is not None:
                raise RuntimeError("Selection probe exited during startup")
            try:
                if app.request("/s")["pid"] == app.process.pid:
                    break
            except OSError:
                pass
            time.sleep(.25)
        app.wait_text("Loaded / 已打开")

        def rows():
            return sorted((w for w in app.snap() if w["i"] == "rich"), key=lambda w: w["r"][1])

        def save():
            app.click_id("save")
            path = next((root / "profile").glob("mini-apps/**/library-v2.json"))
            return path, json.loads(path.read_text())

        path, library = save()
        original = copy.deepcopy(library["documents"][0])

        def reset(blocks):
            library["documents"][0] = copy.deepcopy(original)
            library["documents"][0]["blocks"] = copy.deepcopy(blocks)
            path.write_text(json.dumps(library, ensure_ascii=False))
            app.click_id("reload")

        def point(row, end=False):
            x, y, width, _ = row["r"]
            return x + (width - 20 if end else 9), y + 19

        def select_all():
            x, y = point(rows()[0])
            app.click(x, y)
            app.request("/k", c="A", cmd=1, wait=1)

        def drag(reverse=False):
            first, last = rows()[0], rows()[-1]
            start, end = point(first), point(last, True)
            if reverse:
                start, end = end, start
            app.request("/m", k="down", x=start[0], y=start[1], wait=1)
            app.request("/m", k="move", x=end[0], y=end[1], wait=1)
            app.request("/m", k="up", x=end[0], y=end[1], wait=1)

        select_all()
        app.capture("all-paragraphs-selected")
        app.request("/t", t="全选替换 · English 👩‍💻", wait=1)
        _, state = save()
        assert [b["text"] for b in state["documents"][0]["blocks"]] == ["全选替换 · English 👩‍💻"]
        app.click(*point(rows()[0]))
        app.request("/k", c="Z", cmd=1, wait=1)
        _, state = save()
        assert state["documents"][0]["blocks"] == original["blocks"]
        report["checks"].append("select_all_replace_and_one_step_undo")

        for reverse in (False, True):
            reset(original["blocks"])
            drag(reverse)
            app.capture("reverse-drag" if reverse else "forward-drag")
            app.click_id("bold")
            _, state = save()
            blocks = state["documents"][0]["blocks"]
            assert all(any(m["bold"] and m["start"] == 0 and m["end"] == len(b["text"].encode())
                           for m in b["marks"]) for b in blocks)
        report["checks"].append("forward_and_reverse_cross_paragraph_drag_formatting")

        app.click(*point(rows()[0]))
        app.request("/t", t="插入", wait=1)
        _, state = save()
        assert len(state["documents"][0]["blocks"]) == 3
        assert state["documents"][0]["blocks"][0]["text"] == "插入" + original["blocks"][0]["text"]
        report["checks"].append("click_inside_selection_collapses_to_clicked_caret")

        reset(original["blocks"])
        app.click(*point(rows()[0]))
        end = point(rows()[-1], True)
        app.request("/click", x=end[0], y=end[1], shift=1, wait=1)
        app.request("/t", t="Shift 跨段替换", wait=1)
        _, state = save()
        assert [b["text"] for b in state["documents"][0]["blocks"]] == ["Shift 跨段替换"]
        report["checks"].append("shift_click_extends_across_paragraphs")

        blocks = []
        for index in range(80):
            block = copy.deepcopy(original["blocks"][0])
            block.update(id=uuid.uuid4().hex, text=f"段落 {index}：滚动到屏幕外也仍然属于正文。", marks=[])
            blocks.append(block)
        reset(blocks)
        select_all()
        app.request("/t", t="全部八十段已替换", wait=1)
        _, state = save()
        assert [b["text"] for b in state["documents"][0]["blocks"]] == ["全部八十段已替换"]
        report["checks"].append("select_all_includes_80_virtualized_paragraphs")

        reset(blocks)
        start = point(rows()[0])
        viewport = next(w["r"] for w in app.snap() if w["i"] == "blocks")
        end = (viewport[0] + viewport[2] - 24, viewport[1] + viewport[3] + 18)
        app.request("/m", k="down", x=start[0], y=start[1], wait=1)
        app.request("/m", k="move", x=end[0], y=end[1], wait=1)
        time.sleep(.8)
        app.request("/m", k="up", x=end[0], y=end[1], wait=1)
        app.click_id("bold")
        _, state = save()
        count = sum(any(m["bold"] for m in b["marks"]) for b in state["documents"][0]["blocks"])
        assert count > 5, count
        report["checks"].append("drag_past_viewport_scrolls_and_selects_new_paragraphs")

        empty = copy.deepcopy(original["blocks"])
        for block in empty:
            block.update(text="", marks=[])
        reset(empty)
        app.capture("one-empty-body-placeholder")
        text = "\n".join(row["text"] for row in app.ocr())
        assert text.count("写下你的文章") == 1, text
        empty[-1]["text"] = "正文存在时，前面的空段落不应出现占位提示。"
        reset(empty)
        text = "\n".join(row["text"] for row in app.ocr())
        assert "写下你的文章" not in text
        report["checks"].append("placeholder_only_once_for_empty_body_never_for_blank_paragraphs")
        report["passed"] = True
    finally:
        if not report["passed"]:
            try:
                app.capture("failure")
            except Exception:
                pass
        app.stop()
        (root / "result.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))
        print(json.dumps(dict(report, evidence=str(root)), ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
