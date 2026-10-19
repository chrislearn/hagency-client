#!/usr/bin/env python3
"""Publish only fixture screenshots and test receipts, never test profiles/logs."""
import argparse
import html
import json
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("copy_evidence", type=Path)
    parser.add_argument("--output", type=Path, default=Path("lab/article-editor/native-markdown-evidence"))
    args = parser.parse_args()
    result = json.loads((args.evidence / "result.json").read_text())
    copy_result = json.loads((args.copy_evidence / "result.json").read_text())
    assert result["passed"] and copy_result["passed"]
    assert all(n == 0 for n in result["visible_window_samples"])
    args.output.mkdir(parents=True, exist_ok=True)
    cards = []
    for name in result["captures"]:
        shutil.copyfile(args.evidence / (name + ".png"), args.output / (name + ".png"))
        cards.append(f'<figure><a href="{html.escape(name)}.png"><img loading="lazy" src="{html.escape(name)}.png" alt="{html.escape(name)}"></a><figcaption>{html.escape(name)}</figcaption></figure>')
    shutil.copyfile(args.copy_evidence / "native-selected.png", args.output / "native-copy-selection.png")
    cards.append('<figure><a href="native-copy-selection.png"><img loading="lazy" src="native-copy-selection.png" alt="Native selection"></a><figcaption>Native text-copy fixture</figcaption></figure>')
    shutil.copyfile(args.evidence / "result.json", args.output / "result.json")
    shutil.copyfile(args.copy_evidence / "result.json", args.output / "copy-result.json")
    shutil.copyfile(args.copy_evidence / "selection.json", args.output / "selection.json")
    checks = ''.join(f'<li>{html.escape(name)}</li>' for name in result["checks"])
    (args.output / "index.html").write_text(f'''<!doctype html>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Rinx — native Makepad Markdown</title>
<style>body{{font:16px system-ui;margin:28px;color:#18302b;background:#f3f6f5}}h1{{font-size:28px}}p{{max-width:900px;line-height:1.6}}code{{overflow-wrap:anywhere}}.grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:20px}}figure{{margin:0;padding:12px;background:white;border-radius:12px;box-shadow:0 2px 9px #0001}}img{{width:100%;height:500px;object-fit:contain;object-position:top}}figcaption{{margin-top:12px}}li{{margin:6px 0}}</style>
<h1>Rinx · native Makepad Markdown</h1>
<p>Actual Makepad/Metal screenshots from the signed app, driven through Makepad instrumentation. No WebView or Blitz is used for these article previews. All {len(result['visible_window_samples'])} window-visibility samples were zero. Click a screenshot for its original resolution.</p>
<p>Signed binary SHA-256: <code>{html.escape(result['binary_sha256'])}</code></p>
<details><summary>{len(result['checks'])} Rinx checks passed, plus native selection/copy</summary><ul>{checks}</ul><a href="selection.json">Actual copied text</a></details>
<p><a href="../native-markdown.md">Markdown coverage</a> · <a href="../native-diagrams.md">Diagram coverage and limits</a> · <a href="result.json">Rinx receipt</a> · <a href="copy-result.json">Copy receipt</a></p>
<div class="grid">{''.join(cards)}</div>
''')
    print(args.output / "index.html")


if __name__ == "__main__":
    main()
