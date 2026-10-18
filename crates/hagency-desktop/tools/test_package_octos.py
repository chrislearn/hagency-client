"""Release checks for the standalone assistant executable (no compiler needed)."""
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("package_octos", Path(__file__).with_name("package-octos.py"))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PackagingTests(unittest.TestCase):
    def test_repository_runtime_and_ohos_contract_have_the_same_pin(self):
        package.read_lock()

    def test_prebuilt_revision_accepts_git_abbreviations(self):
        revision = package.read_lock()["revision"]
        for length in (7, 8, 12, 40):
            with self.subTest(length=length):
                self.assertTrue(package.matches_revision(
                    f"octos 2.0.3-rc.13 ({revision[:length]} 2026-09-26)", revision))

    def test_prebuilt_revision_rejects_mismatch_or_unverifiable_output(self):
        revision = package.read_lock()["revision"]
        for version in (
            "octos 2.0.3-rc.13 (abcdef0 2026-09-26)",
            f"octos 2.0.3-rc.13 ({revision[:8]}0 2026-09-26)",
            f"octos 2.0.3-rc.13 ({revision[:6]} 2026-09-26)",
            f"unrelated output containing {revision}",
            "octos 2.0.3-rc.13",
        ):
            with self.subTest(version=version):
                self.assertFalse(package.matches_revision(version, revision))

    def test_mismatched_kernel_pin_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "packaging").mkdir()
            lock = package.read_lock()
            (root / "packaging/octos.lock.json").write_text(json.dumps(lock))
            (root / "Cargo.lock").write_text(
                'name = "octos-cli"\nversion = "2"\nsource = '
                '"git+https://github.com/octos-org/octos.git?rev=' + "a" * 40 + '#abc"\n')
            with self.assertRaisesRegex(ValueError, "differs from Cargo.lock"):
                package.read_lock(root)

    def test_android_refuses_a_host_binary_before_staging(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "octos"
            binary.write_bytes(b"\xcf\xfa\xed\xfe" + bytes(32))
            out = root / "dist"
            with self.assertRaisesRegex(ValueError, "ELF PIE"):
                package.stage(binary, package.read_lock(), package.ANDROID, root, out)
            self.assertFalse(out.exists())

    def test_android_package_records_integrity_and_includes_license(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "octos"
            header = bytearray(64)
            header[:6] = b"\x7fELF\x02\x01"
            struct.pack_into("<HH", header, 16, 3, 183)
            binary.write_bytes(header)
            (root / "LICENSE").write_text("Test license\n")
            lock = package.read_lock()
            staged = package.stage(binary, lock, package.ANDROID, root, root / "dist")
            self.assertEqual(staged.read_bytes(), header)
            evidence = json.loads(Path(str(staged) + ".json").read_text())
            self.assertEqual(evidence["sha256"], hashlib.sha256(header).hexdigest())
            self.assertEqual(evidence["revision"], lock["revision"])
            self.assertEqual((root / "dist/licenses/octos-LICENSE").read_text(), "Test license\n")


if __name__ == "__main__":
    unittest.main()
