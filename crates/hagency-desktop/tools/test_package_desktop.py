"""Regression checks for resource collection with an external Cargo target."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("package_desktop", Path(__file__).with_name("package-desktop.py"))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class ResourcePaths(unittest.TestCase):
    def collect(self, triple):
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / "external-cargo-target"
            with patch.dict(os.environ, {}, clear=True), \
                    patch.object(package.subprocess, "check_output", return_value=json.dumps({"target_directory": str(target)})), \
                    patch.object(package.subprocess, "run") as run:
                if triple:
                    os.environ["CARGO_BUILD_TARGET"] = triple
                package.main()
                collection = run.call_args_list[0]
                command = collection.args[0]
                binary = Path(command[command.index("--path-to-binary") + 1])
                directory = target / triple if triple else target
                self.assertEqual(binary.parent, directory / "release")
                self.assertEqual(binary.stem, "rinx")
                self.assertEqual(collection.kwargs["cwd"], package.ROOT)
                self.assertEqual(Path(run.call_args_list[1].args[0][1]), package.ROOT / "tools/package-octos.py")

    def test_external_host_target(self):
        self.collect(None)

    def test_explicit_target_triple(self):
        self.collect("aarch64-apple-darwin")


if __name__ == "__main__":
    unittest.main()
