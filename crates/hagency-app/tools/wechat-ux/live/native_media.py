#!/usr/bin/env python3
"""Check downloaded photo pixels and native open/close against known fixtures.

This is a media regression check, not a WeChat design-similarity score. Fixture
events are seeded through Matrix APIs; this does not claim native upload coverage.
"""
import argparse
import json
from pathlib import Path
import time

from PIL import Image, ImageChops, ImageStat

from native_probe import NativeApp
from seed import checked


def locate_photo(capture, source, x, width, height):
    """Search the timeline vertically using the actual reference photo pixels."""
    screen = Image.open(capture).convert("RGB")
    target = Image.open(source).convert("RGB").resize((24, 24))
    best = (float("inf"), None)
    for y in range(160, screen.height - height - 100, 2):
        sample = screen.crop((x, y, x + width, y + height)).resize((24, 24))
        error = sum(ImageStat.Stat(ImageChops.difference(sample, target)).mean) / 3
        if error < best[0]:
            best = (error, y)
    assert best[0] < 12, f"Photo missing or wrong pixels (mean RGB error {best[0]:.2f})"
    return {"bounds": [x, best[1], width, height], "mean_rgb_error": best[0]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mock-root", type=Path, required=True)
    parser.add_argument("--root", type=Path, default=Path("lab/wechat-ux/evidence/live"))
    args = parser.parse_args()
    app = NativeApp(args.root)
    result = {"passed": False, "scope": "API-seeded incoming/outgoing image rendering and native viewer return", "evidence": str(app.output)}
    try:
        fixture = json.loads((args.root / "fixture.json").read_text())
        media = json.loads((args.root / "media-fixture.json").read_text())
        result["fixture_events"] = []
        # Put this run's photos at the end, independent of earlier messaging tests.
        for sender, name in (("emma", "post2.jpg"), ("alex", "post1.jpg")):
            asset = media["assets"]["resources/img/" + name]
            event = checked(fixture["url"], "PUT", f"rooms/{fixture['rooms']['emma']}/send/m.room.message/media-{app.output.name}-{sender}",
                            {"msgtype": "m.image", "body": name, "url": asset["uri"], "info": asset["info"]}, fixture["users"][sender]["access_token"])
            result["fixture_events"].append(event["event_id"])
        app.start()
        app.wait_text("Emma Wilson")
        app.capture("chats-media-native")
        app.click_text("Emma Wilson")
        app.wait_text("Message (unencrypted)", pixels=True)
        # The isolated fixture puts two photo messages last. Wait for decoding,
        # then check their rendered contents, not only the message metadata.
        deadline = time.monotonic() + 30
        while True:
            try:
                before = app.capture("media-before-viewer")
                incoming = locate_photo(before, args.mock_root / "resources/img/post2.jpg", 124, 480, 293)
                outgoing = locate_photo(before, args.mock_root / "resources/img/post1.jpg", 208, 458, 560)
                break
            except AssertionError:
                if time.monotonic() >= deadline:
                    raise
                time.sleep(.5)
        result["photos"] = {"incoming": incoming, "outgoing": outgoing}
        x, y, w, h = incoming["bounds"]
        app.click((x + w / 2) / 2, (y + h / 2) / 2)
        opened = app.capture("media-viewer-native")
        result["viewer"] = locate_photo(opened, args.mock_root / "resources/img/post2.jpg", 0, 812, 496)
        # Tap the dark backdrop above the photo to return, preserving the anchor.
        app.click(10, 100)
        after = app.capture("media-after-viewer")
        restored = locate_photo(after, args.mock_root / "resources/img/post2.jpg", 124, 480, 293)
        assert abs(restored["bounds"][1] - incoming["bounds"][1]) <= 2, "Viewer changed the timeline anchor"
        result["timeline_anchor_restored"] = True
        errors = [line for line in (app.output / "native.log").read_text().splitlines() if line.startswith("[E]")]
        assert not errors, "Native script errors"
        result["passed"] = True
        print(json.dumps({"passed": True, "photos": 2, "viewer_return": True}))
    except Exception as error:
        result["error"] = str(error)
        raise
    finally:
        app.stop()
        (app.output / "result.json").write_text(json.dumps(result, indent=2))
        (args.root / "native-media.json").write_text(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
