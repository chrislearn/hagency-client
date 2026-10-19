#!/usr/bin/env python3
"""Build a local side-by-side review; never generate or imply a passing score."""
import argparse
import hashlib
import html
from pathlib import Path
import os


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("lab/wechat-ux/evidence/live"))
    args = parser.parse_args()
    root = args.root.resolve()
    reference = Path("lab/wechat-ux/source/mock-captures-v2").resolve()
    def panel(path, title):
        source = html.escape(os.path.relpath(path, root), quote=True)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        title = html.escape(title, quote=True)
        return f'<figure><figcaption>{title}</figcaption><img loading="lazy" src="{source}" alt="{title}"><small>SHA256 {digest}</small></figure>'

    cards = []
    for name in ("chats", "contacts", "conversation", "discover", "me"):
        native, target = root / f"{name}-native.png", reference / f"{name}.png"
        if not native.exists() or not target.exists():
            continue
        cards.append(f'<section id="{name}"><h2>{name.title()}</h2><div class="pair">{panel(target, "Public Makepad WeChat mock")}{panel(native, "Native Rinx development capture")}</div></section>')
    detail_cards = []
    for filename, title in [
        ("groups-native", "Joined groups"), ("contact-profile-native", "Contact profile"),
        ("settings-native", "Settings"), ("privacy-native", "Privacy controls"),
        ("notifications-current-mode", "Synced notification choice"), ("muted-chat", "Muted conversation"),
        ("actions-reply-composer-native", "Native reply composer"), ("actions-edit-native", "Native edit input"),
        ("actions-reaction-native", "Reaction input"), ("actions-complete-native", "Edited message and reaction"),
        ("media-viewer-native", "Downloaded photo viewer"), ("media-after-viewer", "Return to the same timeline position"),
        ("quote-collapsed-native", "Compact long quote"), ("quote-expanded-native", "Expanded long quote"),
        ("quote-original-native", "Jump to the original Matrix event"),
        ("chat-info-native", "Chat Info and Matrix members"), ("chat-info-muted-native", "Chat Info notification state"),
        ("chat-info-group-native", "Group Chat Info and Matrix members"),
        ("chat-info-return-native", "Conversation draft after returning from Chat Info"),
    ]:
        path = root / (filename + ".png")
        if path.exists():
            detail_cards.append(panel(path, title))
    cards.append('<section id="details"><h2>Native flow captures</h2><p>These additional states have functional evidence but no paired visual acceptance review. Captures come from the separate runs recorded in validation-summary.json.</p><div class="pair">' + ''.join(detail_cards) + '</div></section>')
    document = '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Rinx native UX review</title><style>
body{margin:0;background:#f1f2f3;color:#191919;font:16px system-ui}header,main{max-width:980px;margin:auto;padding:24px}header{position:sticky;top:0;background:#fffffff5;z-index:2}h1{font-size:24px;margin:0 0 8px}p{max-width:80ch;line-height:1.5}nav{display:flex;flex-wrap:wrap;gap:20px}a{color:#166b43}.pair{display:grid;grid-template-columns:1fr 1fr;gap:24px}figure{margin:0}figcaption{margin-bottom:8px;font-weight:600}img{width:100%;height:auto;border:1px solid #ddd}small{display:block;overflow-wrap:anywhere;font:11px monospace;margin-top:6px}section{scroll-margin-top:210px;margin-bottom:40px}label{display:block;margin-top:10px}@media(max-width:560px){header,main{padding:14px}.pair{gap:10px}figcaption{font-size:12px}}
</style><header><h1>Native mobile UX — review in progress</h1><p>No similarity score has been awarded. References come from the public Apache-2.0-licensed Makepad mock, not a captured Tencent WeChat release. Viewports, fixture content, and window chrome differ; this view supports qualitative review only.</p><nav>''' + ''.join(f'<a href="#{name}">{name.title()}</a>' for name in ("chats", "contacts", "conversation", "discover", "me")) + '''</nav><label>Preview width <input type="range" min="560" max="1400" value="980" oninput="document.querySelector('main').style.maxWidth=this.value+'px'"></label></header><main>''' + ''.join(cards) + '''<p>Review layout, typography, colors, icons, density, and states. The full acceptance gate also requires native journey and backend evidence. The current sparse Discover/Me content and remaining deep flows are gaps, not passing fixtures.</p></main></html>'''
    (root / "review.html").write_text(document)
    print(root / "review.html")


if __name__ == "__main__":
    main()
