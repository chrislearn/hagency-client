#!/usr/bin/env python3
"""Compare KaTeX in WKWebView with Rinx's native math + production Blitz pipeline.

Requires KaTeX 0.16.22, Node, the WKWebView capture executable, and output
from the article_render_comparison Rust example. The two sides typeset math
independently; Rinx uses Makepad's native math layout and generated PNG glyphs.
"""
import argparse
import hashlib
import html
import json
from pathlib import Path
import re
import shutil
import subprocess
from PIL import Image, ImageDraw, ImageFont

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--node", required=True, type=Path)
parser.add_argument("--katex", required=True, type=Path)
args = parser.parse_args()
repo = Path(__file__).resolve().parents[3]
root = repo / "lab/article-editor/render-comparison"
out = root / "math"
out.mkdir(exist_ok=True)
source = (root / "evidence/09-math/source.md").read_text()
formulas = []

def placeholder(match):
    kind = match[1] or "dollars"
    original = match[2] if match[1] else match[3]
    tex = original.strip()
    normalizations = []
    if tex.startswith(r"\(") and tex.endswith(r"\)"):
        tex = tex[2:-2]
        normalizations.append("Remove redundant outer \\( \\) inside $$")
    if kind in ["math", "katex", "latex"] and r"\_" in tex:
        tex = tex.replace(r"\_", "_")
        normalizations.append("Interpret Editor.md Markdown-escaped underscores as TeX subscripts")
    display = bool(match[1]) or (
        not source[:match.start()].split("\n")[-1].strip()
        and not source[match.end():].split("\n")[0].strip())
    token = f"@@RINXMATH{len(formulas)}@@"
    formulas.append({"source":original,"tex":tex,"kind":kind,"display":display,"normalizations":normalizations})
    return token

marked = re.sub(r"```(math|katex|latex)\n([\s\S]*?)```|\$\$([\s\S]*?)\$\$", placeholder, source)
assert len(formulas) == 9
# This fixture subsection consists only of a heading, paragraphs and math.
# Extract math before Markdown escaping can consume TeX backslashes.
body = "\n".join(
    "<h3>" + html.escape(p[4:]) + "</h3>" if p.startswith("### ")
    else "<p>" + html.escape(p) + "</p>"
    for p in (p.strip() for p in marked.strip().split("\n\n")) if p)
package = args.katex.resolve()
version = json.loads((package / "package.json").read_text())["version"]
assert version == "0.16.22"
rendered = subprocess.check_output([str(args.node), "-e", '''
const fs=require('fs'),katex=require(process.argv[1]);
const formulas=JSON.parse(fs.readFileSync(0,'utf8'));
for(const f of formulas) f.html=katex.renderToString(f.tex,{displayMode:f.display,output:'html',throwOnError:true,trust:false,maxExpand:1000});
process.stdout.write(JSON.stringify(formulas));
''', str(package / "dist/katex.js")], input=json.dumps(formulas), text=True)
formulas = json.loads(rendered)
for i, formula in enumerate(formulas):
    body = body.replace(f"@@RINXMATH{i}@@", formula["html"])
assert "@@RINXMATH" not in body and "katex-error" not in body
css = (package / "dist/katex.min.css").read_text()
css = re.sub(r"src:[^;}]+", lambda m: next(
    ("src:" + item for item in m[0][4:].split(",") if ".ttf" in item), m[0]), css)
fonts = re.findall(r"url\(([^)]+)\)", css)
assert len(fonts) <= 32
resources = {}
for font in fonts:
    path = out / font
    path.parent.mkdir(exist_ok=True)
    shutil.copy2(package / "dist" / font, path)
    resources["https://comparison.invalid/" + font] = str(path)
shutil.copy2(package / "LICENSE", out / "KaTeX-LICENSE.txt")
article_css = (root / "article.css").read_text()
page = f'<!doctype html><html><head><meta charset="utf-8"><style>{article_css}\n{css}</style></head><body><article>{body}</article></body></html>'
(out / "math.html").write_text(page)
(out / "source.md").write_text(source)
(out / "formulas.json").write_text(json.dumps(formulas, ensure_ascii=False, indent=2))
(out / "resources.json").write_text(json.dumps({"full_page":True,"resources":resources}, indent=2))
(out / "webview-jobs.json").write_text(json.dumps([{"input":"math.html","output":"webview-math"}]))
subprocess.run([str(repo / "target/article-markdown-webview"), str(out)], check=True, timeout=35)
wk = json.loads((out / "webview-math.json").read_text())
blitz = next(s for s in json.loads((root / "evidence/renders.json").read_text())["sections"] if s["id"] == "09-math")
assert wk["math"] == {"formulas":9,"errors":0}
assert wk["source_sha256"] == hashlib.sha256((out / "math.html").read_bytes()).hexdigest()
assert not blitz["denied_resources"] and blitz["math_formulas"] == 9 and not blitz["clipped"]
shutil.copy2(root / "evidence/09-math/blitz.png", out / "blitz.png")
web = Image.new("RGB", (880, wk["height"] * 2), "white")
for tile in wk["tiles"]:
    web.paste(Image.open(out / tile["file"]).convert("RGB"), (0, tile["y_css"] * 2))
web.save(out / "webview.png")
native = Image.open(out / "blitz.png").convert("RGB")
pair = Image.new("RGB", (1784, max(web.height, native.height) + 64), "#edf1f5")
draw = ImageDraw.Draw(pair)
font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 25)
draw.text((16,18), "WKWebView + KaTeX", fill="#142332", font=font)
draw.text((920,18), "Makepad math + Blitz / Rinx", fill="#142332", font=font)
pair.paste(web,(0,64));pair.paste(native,(904,64));pair.save(out / "pair.png")
report = {"formulas":9,"katex":version,"source_unchanged":True,
    "reference_html_sha256":wk["source_sha256"],
    "rinx_html_sha256":hashlib.sha256((root / "evidence/09-math/rinx.html").read_bytes()).hexdigest(),
    "font_resources":len(fonts),"denied_resources":blitz["denied_resources"],
    "renderer_sha256":hashlib.sha256((repo / "target/debug/examples/article_render_comparison").read_bytes()).hexdigest(),
    "normalizations":[{"index":i,"changes":f["normalizations"]} for i,f in enumerate(formulas) if f["normalizations"]],
    "production_math_added":True,"native_math":"makepad-latex-math + NewCMMath; 2x generated PNG glyphs",
    "native_compatibility":"Leading displaystyle/textstyle interpreted; Bigl/Bigr pairs use content-sized delimiters",
    "comparison":"Independent math typesetters; KaTeX/WebKit reference versus production Rinx/Blitz"}
(out / "result.json").write_text(json.dumps(report,ensure_ascii=False,indent=2))
(out / "index.html").write_text('''<!doctype html><html><head><meta charset="utf-8"><title>Math · WebView vs Blitz</title><style>
*{box-sizing:border-box}body{font:15px -apple-system,sans-serif;background:#edf1f5;color:#172737;margin:0}header{padding:18px 28px;background:white}h1{font-size:24px;margin:0 0 10px}p{line-height:1.5;margin:8px 0}main{padding:18px 28px}.columns{display:grid;grid-template-columns:440px 440px;gap:24px;justify-content:center}.title{font-weight:650;margin-bottom:12px}img{display:block;width:440px;height:auto;background:white}.note{font-size:13px;color:#536274}a{color:#087545}.warning{border-left:4px solid #d27b14;padding:8px 14px;background:#fff4df}footer{padding:12px 28px}
</style></head><body><header><h1>Math is now typeset before rendering</h1><p>Nine formulas · independent typesetters · 440 CSS px at 2×.</p><p>Left: KaTeX 0.16.22 in WebKit. Right: Rinx typesets TeX with Makepad’s native math engine, then gives local formula images to Blitz. The native Rinx reader uses the same math engine. TeX source stays editable.</p></header><main><div class="columns"><div><div class="title">WKWebView + KaTeX</div><img id="web" src="webview.png" alt="Nine typeset math formulas in WebKit"></div><div><div class="title">Makepad math + Blitz · Rinx preview</div><img id="blitz" src="blitz.png" alt="Nine formulas rendered by the production Rinx pipeline"></div></div></main><footer><p><a href="pair.png">Full comparison PNG</a> · <a href="math.html">Live WebView HTML</a> · <a href="source.md">Original source</a> · <a href="result.json">Capture details</a> · <a href="../index.html">All sections</a></p><p class="note">Source retained. For Editor.md compatibility, rendering removes redundant \\( \\) inside $$ and interprets Markdown-escaped underscores in math fences as subscripts. These changes are recorded per formula. No screenshot corrections were applied.</p></footer></body></html>''')
print(json.dumps(report), flush=True)
