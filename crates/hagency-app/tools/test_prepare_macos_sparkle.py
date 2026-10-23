import base64
import importlib.util
import os
from pathlib import Path
import plistlib
import tempfile
import tomllib
import unittest
import shutil
import subprocess
import sys

spec = importlib.util.spec_from_file_location("prepare_sparkle", Path(__file__).with_name("prepare-macos-sparkle.py"))
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)
APP = Path(__file__).resolve().parents[1]


class CandidateConfiguration(unittest.TestCase):
    def fixture(self, root):
        manifest, info = root / "Cargo.toml", root / "Info.plist"
        manifest.write_bytes((APP / "Cargo.toml").read_bytes())
        info.write_bytes((APP / "packaging/macos/Info.plist").read_bytes())
        sdk = root / "sdk"
        resources = sdk / "Sparkle.framework/Resources"
        resources.mkdir(parents=True)
        (resources / "Info.plist").write_bytes(plistlib.dumps({"CFBundleShortVersionString": "2.10.0"}))
        headers = sdk / "Sparkle.framework/Headers"
        headers.mkdir()
        (headers / "Sparkle.h").touch()
        (sdk / "LICENSE").write_text("test license")
        return manifest, info, sdk

    def test_architecture_version_features_framework_and_safe_defaults_are_bound_together(self):
        for arch, feed_arch in [("aarch64", "arm64"), ("x86_64", "amd64")]:
            with self.subTest(arch=arch), tempfile.TemporaryDirectory() as directory:
                manifest, info, sdk = self.fixture(Path(directory))
                key = base64.b64encode(bytes(range(32))).decode()
                for _ in range(2):  # Reconfiguration must be idempotent.
                    prepare.prepare(manifest, info, sdk, arch, key, "TESTTEAM01")
                cargo = tomllib.loads(manifest.read_text())
                plist = plistlib.loads(info.read_bytes())
                self.assertEqual(plist["CFBundleVersion"], cargo["package"]["version"])
                self.assertEqual(plist["CFBundleShortVersionString"], plist["CFBundleVersion"])
                self.assertTrue(plist["SUFeedURL"].endswith(f"appcast-macos-{feed_arch}.xml"))
                self.assertEqual(cargo["features"]["default"].count("macos-sparkle"), 1)
                self.assertEqual(cargo["package"]["metadata"]["packager"]["macos"]["frameworks"], [str(sdk.resolve() / "Sparkle.framework")])
                self.assertFalse(plist["SUAllowsAutomaticUpdates"])
                self.assertFalse(plist["SUAutomaticallyUpdate"])
                self.assertTrue(plist["SUVerifyUpdateBeforeExtraction"])

    def test_invalid_configuration_does_not_modify_inputs(self):
        for key, team, arch in [("", "TESTTEAM01", "arm64"), ("invalid", "TESTTEAM01", "arm64"), (base64.b64encode(bytes(32)).decode(), "", "arm64"), (base64.b64encode(bytes(32)).decode(), "TESTTEAM01", "other")]:
            with self.subTest(key=key, team=team, arch=arch), tempfile.TemporaryDirectory() as directory:
                manifest, info, sdk = self.fixture(Path(directory))
                before = manifest.read_bytes(), info.read_bytes()
                with self.assertRaises(ValueError):
                    prepare.prepare(manifest, info, sdk, arch, key, team)
                self.assertEqual(before, (manifest.read_bytes(), info.read_bytes()))

    @unittest.skipUnless(sys.platform == "darwin", "macOS build script uses PlistBuddy and BSD sed")
    def test_failed_packaging_restores_both_original_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest, info, sdk = self.fixture(root)
            (root / "packaging/macos").mkdir(parents=True)
            shutil.copy2(info, root / "packaging/macos/Info.plist")
            info = root / "packaging/macos/Info.plist"
            for name in ["Entitlements.plist", "dmg-background.png"]:
                (info.parent / name).touch()
            script = root / "packaging/build-macos-dmg.sh"
            shutil.copy2(APP / "packaging/build-macos-dmg.sh", script)
            (root / "tools").mkdir()
            shutil.copy2(Path(__file__).with_name("prepare-macos-sparkle.py"), root / "tools/prepare-macos-sparkle.py")
            bins = root / "bin"
            bins.mkdir()
            for name, text in [("cargo", "#!/bin/sh\nexit 41\n"), ("python3", '#!/bin/sh\nexec "' + sys.executable + '" "$@"\n')]:
                path = bins / name
                path.write_text(text)
                path.chmod(0o700)
            before = manifest.read_bytes(), info.read_bytes()
            env = os.environ | {"PATH": str(bins) + os.pathsep + os.environ["PATH"],
                "APPLE_ID": "fixture", "APPLE_PASSWORD": "fixture", "APPLE_TEAM_ID": "TESTTEAM01",
                "APPLE_SIGNING_IDENTITY": "fixture", "HAGENCY_MACOS_SPARKLE": "1",
                "HAGENCY_SPARKLE_SDK": str(sdk), "HAGENCY_SPARKLE_PUBLIC_KEY": base64.b64encode(bytes(32)).decode()}
            result = subprocess.run(["bash", str(script)], env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 41, result.stdout + result.stderr)
            self.assertEqual(before, (manifest.read_bytes(), info.read_bytes()))
            self.assertFalse(manifest.with_suffix(".toml.bak").exists())
            self.assertFalse(info.with_suffix(".plist.bak").exists())


if __name__ == "__main__":
    unittest.main()
