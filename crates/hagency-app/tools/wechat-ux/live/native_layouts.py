#!/usr/bin/env python3
"""Capture responsive native layouts and verify the four mobile hit targets."""
import argparse
import json
from pathlib import Path
from PIL import Image

from native_probe import NativeApp


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("lab/wechat-ux/evidence/live"))
    parser.add_argument("--port", type=int, default=8199)
    args = parser.parse_args()
    results = []
    for width, height in [(320, 640), (375, 812), (430, 932), (1024, 768)]:
        app = NativeApp(args.root, args.port, (width, height))
        result = {"viewport": [width, height], "passed": False, "evidence": str(app.output)}
        try:
            app.start()
            if width < 600:
                app.wait_text("Emma Wilson")
                for tab_index, (tab, expected) in enumerate([("chats_tab", "Emma Wilson"), ("contacts_tab", "Emma Wilson"), ("discover_tab", "Explore Groups"), ("me_tab", "Alex Chen")]):
                    targets = [w for w in app.request("/snap", all=1)["s"] if w["i"] == tab and w["r"][2] > 0 and w["r"][3] > 0]
                    assert len(targets) == 1, f"Ambiguous/missing tab target: {tab}"
                    x, y, w, h = targets[0]["r"]
                    assert w >= 44 and h >= 44, f"Touch target below 44: {tab}"
                    assert x >= -1 and y >= -1 and x+w <= width+1 and y+h <= height+1, f"Tab outside viewport: {tab}"
                    app.click_id(tab)
                    app.wait_text(expected)
                    capture = app.capture(f"layout-{width}-{height}-{tab}")
                    pixels = Image.open(capture).convert("RGB")
                    scale = pixels.height / height
                    active = []
                    for index in range(4):
                        colors = pixels.crop((int(index*pixels.width/4), int((height-56)*scale), int((index+1)*pixels.width/4), pixels.height)).getdata()
                        green = sum(g > 120 and g > r*1.5 and g > b*1.2 for r, g, b in colors)
                        if green > 20:
                            active.append(index)
                    assert active == [tab_index], f"Rendered selected tab color is wrong: {active}, expected {tab_index}"
                app.click_id("chats_tab")
                app.wait_text("Emma Wilson")
            else:
                app.wait_text("Weekend Plans")
                assert not any(w["i"] == "chats_tab" for w in app.snap()), "Mobile tabs remained in desktop layout"
                app.capture(f"layout-{width}-{height}-desktop")
            errors = [line for line in (app.output / "native.log").read_text(errors="replace").splitlines() if line.startswith("[E]")]
            assert not errors, "Native layout emitted script errors: " + "\n".join(errors[:3])
            result["passed"] = True
        except Exception as error:
            result["error"] = str(error)
            raise
        finally:
            app.stop()
            results.append(result)
            (app.output / "result.json").write_text(json.dumps(result, indent=2))
            (args.root / "native-layouts.json").write_text(json.dumps(results, indent=2))
    print(json.dumps({"passed": True, "viewports": len(results)}))


if __name__ == "__main__":
    main()
