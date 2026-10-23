#!/usr/bin/env python3
"""Configure a backed-up release manifest/plist before the signed macOS build.

This prepares a candidate; it neither publishes a feed nor signs an update.
Use the fixed SDK downloaded by sparkle-prototype.py sdk.
"""
import argparse
import base64
import json
import os
from pathlib import Path
import plistlib
import re
import tomllib


def prepare(manifest, plist, sdk, architecture, public_key, team):
    source = manifest.read_text()
    cargo = tomllib.loads(source)
    version = cargo["package"]["version"]
    if not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version):
        raise ValueError("Sparkle candidates require a stable x.y.z Cargo version")
    arches = {"aarch64": "arm64", "arm64": "arm64", "x86_64": "amd64", "amd64": "amd64"}
    if architecture not in arches:
        raise ValueError("Unsupported macOS executable architecture")
    if len(base64.b64decode(public_key, validate=True)) != 32:
        raise ValueError("Expected a Sparkle Ed25519 public key")
    if not re.fullmatch(r"[A-Z0-9]{10}", team):
        raise ValueError("Expected the Developer ID signing team")
    sdk = sdk.resolve(strict=True)
    framework = sdk / "Sparkle.framework"
    sdk_info = plistlib.loads((framework / "Resources/Info.plist").read_bytes())
    if sdk_info["CFBundleShortVersionString"] != "2.10.0" or not (framework / "Headers/Sparkle.h").is_file():
        raise ValueError("Expected the pinned Sparkle 2.10.0 SDK")
    if not (sdk / "LICENSE").is_file():
        raise ValueError("Sparkle license is missing")
    config = cargo["package"]["metadata"]["packager"]
    if config["identifier"] != "org.hagency.desktop":
        raise ValueError("Refusing a non-product manifest")
    defaults = cargo["features"]["default"]
    if "macos-sparkle" not in defaults:
        defaults.append("macos-sparkle")
    frameworks = config["macos"]["frameworks"]
    if str(framework) not in frameworks:
        frameworks.append(str(framework))
    source, changed_default = re.subn(r"^default\s*=\s*\[[^\n]*\]", "default = " + json.dumps(defaults), source, count=1, flags=re.M)
    source, changed_frameworks = re.subn(r"^frameworks\s*=\s*\[[^\n]*\]", "frameworks = " + json.dumps(frameworks), source, count=1, flags=re.M)
    if changed_default != 1 or changed_frameworks != 1:
        raise ValueError("Could not locate the default features/frameworks declarations")
    license_entry = {"src": str(sdk / "LICENSE"), "target": "licenses/Sparkle-LICENSE"}
    if license_entry not in config["resources"]:
        source = source.replace("resources = [\n", "resources = [\n    { src = " + json.dumps(license_entry["src"]) + ', target = "licenses/Sparkle-LICENSE" },\n', 1)
    tomllib.loads(source)  # Validate every output before modifying either input.
    info = plistlib.loads(plist.read_bytes())
    if info["CFBundleIdentifier"] != config["identifier"]:
        raise ValueError("Manifest and bundle identities differ")
    info.update(CFBundleVersion=version, CFBundleShortVersionString=version,
                LSMinimumSystemVersion="12.0", HagencySparkleEnabled=True,
                SUFeedURL=f'https://github.com/hagency-org/hagency-client/releases/latest/download/appcast-macos-{arches[architecture]}.xml',
                SUPublicEDKey=public_key, SUEnableAutomaticChecks=True,
                SUScheduledCheckInterval=86400, SUAutomaticallyUpdate=False,
                SUAllowsAutomaticUpdates=False, SUEnableSystemProfiling=False,
                SUVerifyUpdateBeforeExtraction=True)
    info.pop("SUDefaultsDomain", None)
    payload = plistlib.dumps(info)
    manifest.write_text(source)
    plist.write_bytes(payload)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--plist", type=Path, required=True)
    parser.add_argument("--arch", required=True)
    args = parser.parse_args()
    prepare(args.manifest, args.plist, Path(os.environ["HAGENCY_SPARKLE_SDK"]),
            args.arch, os.environ["HAGENCY_SPARKLE_PUBLIC_KEY"], os.environ["HAGENCY_SPARKLE_TEAM_ID"])


if __name__ == "__main__":
    main()
