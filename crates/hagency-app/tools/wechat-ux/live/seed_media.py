#!/usr/bin/env python3
"""Upload reference imagery to isolated test accounts, never production assets.

Requires a local project-robius/makepad_wechat checkout. Its Apache-2.0 license
is retained in lab/wechat-ux/source; hashes and commit identify each fixture.
"""
import argparse
import hashlib
import json
import mimetypes
from pathlib import Path
import subprocess
import urllib.parse
import urllib.request

from PIL import Image
from seed import checked


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mock-root", type=Path, required=True)
    parser.add_argument("--root", type=Path, default=Path("lab/wechat-ux/evidence/live"))
    args = parser.parse_args()
    fixture = json.loads((args.root / "fixture.json").read_text())
    assert all(u["user_id"].startswith("@robrix_ux_") for u in fixture["users"].values()), "Test accounts required"
    receipt_path = args.root / "media-fixture.json"
    receipt = json.loads(receipt_path.read_text()) if receipt_path.exists() else {
        "source": "https://github.com/project-robius/makepad_wechat",
        "commit": subprocess.check_output(["git", "-C", str(args.mock_root), "rev-parse", "HEAD"], text=True).strip(),
        "purpose": "Private native UX test fixture; not redistributed production artwork",
        "assets": {}, "events": {},
    }

    def persist():
        receipt_path.write_text(json.dumps(receipt, indent=2))

    def upload(relative, sender):
        path = args.mock_root / relative
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if relative in receipt["assets"]:
            assert receipt["assets"][relative]["sha256"] == digest, "Reference asset changed"
            return receipt["assets"][relative]
        mime = mimetypes.guess_type(path.name)[0]
        request = urllib.request.Request(
            fixture["url"] + "/_matrix/media/v3/upload?filename=" + urllib.parse.quote(path.name),
            data=data, method="POST", headers={"Authorization": "Bearer " + sender["access_token"], "Content-Type": mime})
        with urllib.request.urlopen(request, timeout=30) as response:
            uri = json.load(response)["content_uri"]
        width, height = Image.open(path).size
        asset = {"sha256": digest, "uri": uri, "info": {"mimetype": mime, "size": len(data), "w": width, "h": height}}
        receipt["assets"][relative] = asset
        persist()
        return asset

    for index, key in enumerate(("alex", "emma", "leo", "nora"), 1):
        user = fixture["users"][key]
        asset = upload(f"resources/img/avatars/user{index}.png", user)
        checked(fixture["url"], "PUT", f"profile/{urllib.parse.quote(user['user_id'], safe='')}/avatar_url",
                {"avatar_url": asset["uri"]}, user["access_token"])
    for key, index in (("weekend", 5), ("design", 6)):
        asset = upload(f"resources/img/avatars/user{index}.png", fixture["users"]["alex"])
        checked(fixture["url"], "PUT", f"rooms/{fixture['rooms'][key]}/state/m.room.avatar",
                {"url": asset["uri"]}, fixture["users"]["alex"]["access_token"])
    for key, sender, filename in (("incoming", "emma", "post2.jpg"), ("outgoing", "alex", "post1.jpg")):
        if key in receipt["events"]:
            continue
        asset = upload("resources/img/" + filename, fixture["users"][sender])
        event = checked(fixture["url"], "PUT", f"rooms/{fixture['rooms']['emma']}/send/m.room.message/media-fixture-{key}-v1",
                        {"msgtype": "m.image", "body": filename, "url": asset["uri"], "info": asset["info"]},
                        fixture["users"][sender]["access_token"])
        receipt["events"][key] = event["event_id"]
        persist()
    print(json.dumps({"avatars": 6, "image_events": len(receipt["events"]), "receipt": str(receipt_path)}))


if __name__ == "__main__":
    main()
