#!/usr/bin/env python3
"""Capture the supplied Markdown with native WebKit and Rinx's pinned renderer.

Run the Rust article_render_comparison example first. Requires macOS and Pillow.
Remote image snapshots feed both pipelines; SVGs become PNGs in Rinx.
"""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import html
import json
from pathlib import Path
import re
import shutil
import subprocess
import urllib.request
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[3]
REPORT = ROOT / "lab/article-editor/render-comparison"
OUT = REPORT / "evidence"
data = json.loads((OUT / "renders.json").read_text())
assets = OUT / "assets"
assets.mkdir(exist_ok=True)
# WKWebView may read only within this capture root. Keep KaTeX fonts inside
# it as well; references outside that grant silently fall back to system fonts.
shutil.copytree(REPORT / "math/fonts", assets / "katex-fonts", dirs_exist_ok=True)
urls = sorted(set(re.findall(r'<img[^>]*src="(https://[^"]+)"', "".join(
    (OUT / section["id"] / "browser-reference.html").read_text()
    for section in data["sections"]))))

def download(url):
    try:
        request = urllib.request.Request(html.unescape(url), headers={"User-Agent": "Rinx-render-comparison/1.0"})
        with urllib.request.urlopen(request, timeout=18) as response:
            content = response.read(2_000_001)
            assert len(content) <= 2_000_000
            mime = response.headers.get_content_type()
        extension = {"image/png": ".png", "image/jpeg": ".jpg", "image/svg+xml": ".svg"}[mime]
        name = hashlib.sha256(url.encode()).hexdigest()[:16] + extension
        (assets / name).write_bytes(content)
        return {"url":url,"file":name,"bytes":len(content),"sha256":hashlib.sha256(content).hexdigest()}
    except Exception as error:
        return {"url":url,"error":str(error)}

existing = {r["url"]:r for r in json.loads((OUT / "resources.json").read_text()) if "file" in r}
def snapshot(url):
    r = existing.get(url)
    if r and (assets / r["file"]).exists():
        assert hashlib.sha256((assets / r["file"]).read_bytes()).hexdigest() == r["sha256"]
        return r
    return download(url)
with ThreadPoolExecutor(max_workers=8) as pool:
    resources = list(pool.map(snapshot, urls))
(OUT / "resources.json").write_text(json.dumps(resources, indent=2))
print(f"Browser reference assets: {sum('file' in r for r in resources)}/{len(resources)} fetched", flush=True)
jobs = []
for section in data["sections"]:
    folder = OUT / section["id"]
    original = (folder / "browser-reference.html").read_text()
    if section["id"] == "09-math":
        original = (REPORT / "math/math.html").read_text().replace("url(fonts/", "url(../assets/katex-fonts/")
        (folder / "browser-reference.html").write_text(original)
    for resource in resources:
        target = "../assets/" + resource.get("file", "unavailable-image")
        original = original.replace('src="' + resource["url"] + '"', 'src="' + target + '"')
    (folder / "browser-snapshot.html").write_text(original)
    for mode, source in [("same", "rinx.html"), ("reference", "browser-snapshot.html")]:
        jobs.append({"input":section["id"] + "/" + source,"output":section["id"] + "/webview-" + mode})
(OUT / "webview-jobs.json").write_text(json.dumps(jobs))
subprocess.run([str(ROOT / "target/article-markdown-webview"), str(OUT)], check=True, timeout=240)

font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 25)
for section in data["sections"]:
    folder = OUT / section["id"]
    blitz = Image.open(folder / "blitz.png").convert("RGB")
    for mode in ["same", "reference"]:
        capture = json.loads((folder / f"webview-{mode}.json").read_text())
        if section["id"] == "09-math" and mode == "reference":
            assert capture["math"] == {"formulas": 9, "errors": 0}
            assert all(f["status"] != "error" for f in capture["fonts"]), capture["fonts"]
        original = folder / ("rinx.html" if mode == "same" else "browser-snapshot.html")
        assert capture["source_sha256"] == hashlib.sha256(original.read_bytes()).hexdigest()
        web = Image.new("RGB", (880, capture["height"] * 2), "white")
        for tile in capture["tiles"]:
            web.paste(Image.open(OUT / tile["file"]).convert("RGB"), (0, tile["y_css"] * 2))
        web.save(folder / f"webview-{mode}.png")
        section[f"webview_{mode}_height_css"] = capture["height"]
        section[f"webview_{mode}_images"] = capture["images"]
        pair = Image.new("RGB", (1784, max(web.height, blitz.height) + 64), "#edf1f5")
        draw = ImageDraw.Draw(pair)
        draw.text((16, 18), "WKWebView" + (" / same HTML" if mode == "same" else " / browser reference"), fill="#142332", font=font)
        draw.text((920, 18), "Makepad + Blitz / Rinx import", fill="#142332", font=font)
        pair.paste(web, (0, 64)); pair.paste(blitz, (904, 64))
        pair.save(folder / f"pair-{mode}.png")
    assert not section["clipped"], section["id"]
data["image_resources"] = resources
(REPORT / "results.json").write_text(json.dumps(data, ensure_ascii=False, indent=2))

options = "".join(f'<option value="{s["id"]}">{html.escape(s["name"])}</option>' for s in data["sections"])
page = '''<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Editor.md · WKWebView vs Makepad + Blitz</title>
<style>
*{box-sizing:border-box}body{margin:0;background:#edf1f5;color:#172737;font:15px -apple-system,BlinkMacSystemFont,sans-serif}header{padding:22px 26px 16px;background:white;border-bottom:1px solid #dce2e8}h1{font-size:24px;margin:0 0 8px}p{margin:8px 0;line-height:1.5}label{font-weight:600;margin-right:20px;display:inline-block}select{font:inherit;margin:6px;padding:7px}main{padding:16px 26px}.columns{display:grid;grid-template-columns:440px 440px;gap:24px;justify-content:center}.title{font-weight:650;padding:10px 0}.pane{height:600px;overflow:auto;background:white;box-shadow:0 0 0 1px #d5dde5}.pane img{display:block;width:440px;height:auto}.note{font-size:13px;color:#536274}a{color:#087545}#limits{max-width:920px;margin:14px auto}#facts{font-variant-numeric:tabular-nums}.links{display:flex;gap:20px;margin:12px 0}
</style></head><body><header><h1>Editor.md: WebView vs Makepad + Blitz</h1>
<p class="note">Earlier optional HTML/CSS comparison. Current Rinx renders Markdown, Mermaid and sequence diagrams with Makepad only. <a href="../native-diagrams.md">Current native coverage and test captures</a>.</p>
<p>Actual macOS WKWebView snapshots and Rinx’s pinned Blitz render output. 440 CSS px · 2× pixels · the same Rinx Classic CSS.</p>
<label>Section <select id="section">OPTIONS</select></label><label>Comparison <select id="mode"><option value="same">Same imported HTML — engine comparison</option><option value="reference">Browser Markdown — import + engine comparison</option></select></label>
<p id="explain" class="note"></p></header><main><div class="columns"><div><div class="title" id="leftTitle">macOS WKWebView</div><div class="pane" id="left"><img id="web" alt="Actual WKWebView screenshot"></div></div><div><div class="title">Makepad + Blitz · Rinx article renderer</div><div class="pane" id="right"><img id="blitz" alt="Actual Blitz render output"></div></div></div>
<div id="limits"><p id="facts" class="note"></p><div class="links"><a id="pair" target="_blank">Open full side-by-side PNG</a><a id="source" target="_blank">Section Markdown</a><a href="source.md">Complete Markdown</a><a href="results.json">Capture details</a></div>
<p><b>Current limits:</b> Images, SVG badges, table styling, language-aware code highlighting and emoji now render in both modes. Math renders in both modes. The browser reference uses KaTeX; Rinx uses native Makepad math layout and generated formula images. Same-HTML mode shares Rinx’s formula images between the two engines. Flowcharts render with shared Rust layout: browser SVG in reference mode, native rasterized pixels in Rinx. TOC and sequence diagrams remain literal. The reference and Rinx share syntax highlighting and shortcode expansion; the reference retains browser-native SVG images. The complete Blitz preview hits its height limit; these ten sections are captured separately so you can inspect all content.</p>
<p class="note">Both panes scroll independently. No screenshot alignment, reflow or visual corrections were applied. Both pipelines use the same captured remote image bytes; Rinx rasterizes SVGs before granting pixels to Blitz.</p></div></main><script>
const data=DATA;
function update(){const s=data.sections.find(s=>s.id===document.getElementById('section').value),m=document.getElementById('mode').value,b='evidence/'+s.id+'/';document.getElementById('web').src=b+'webview-'+m+'.png';document.getElementById('blitz').src=b+'blitz.png';document.getElementById('pair').href=b+'pair-'+m+'.png';document.getElementById('source').href=b+'source.md';document.getElementById('leftTitle').textContent=m==='same'?'WKWebView · identical imported HTML':'WKWebView · Markdown + KaTeX + images';document.getElementById('explain').textContent=m==='same'?'Identical HTML bytes in both engines. Formula image bytes are also shared. This isolates typography, line wrapping and layout.':'Same Markdown and CSS, different import pipelines. Left retains images, links and checkboxes and typesets math with KaTeX; right uses Rinx’s importer and native math engine.';document.getElementById('facts').textContent='Section height: WebView '+s['webview_'+m+'_height_css']+' CSS px · Blitz '+s.height_css.toFixed(1)+' CSS px. Both section captures are complete.';document.getElementById('left').scrollTop=0;document.getElementById('right').scrollTop=0;}
document.querySelectorAll('select').forEach(el=>el.onchange=update);document.getElementById('section').value='10-diagrams';document.getElementById('mode').value='same';update();
</script></body></html>'''
page = page.replace("OPTIONS", options).replace("DATA", json.dumps(data, ensure_ascii=False).replace("</", "<\\/"))
(REPORT / "index.html").write_text(page)
print(json.dumps({"report":str(REPORT / "index.html"),"sections":len(data["sections"]),"full_document":data["full_document"]}), flush=True)
