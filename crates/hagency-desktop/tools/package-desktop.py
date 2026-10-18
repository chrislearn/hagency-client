"""Prepare Desktop resources using the unified workspace's actual output path."""
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--locked", "--format-version", "1"],
        cwd=ROOT, text=True,
    ))
    extension = ".exe" if os.name == "nt" else ""
    target = Path(metadata["target_directory"])
    # cargo-packager normally uses the host target. An explicit packaging
    # target can select the corresponding Cargo target subdirectory.
    triple = os.environ.get("CARGO_BUILD_TARGET")
    binary_dir = target / triple if triple else target
    binary = binary_dir / "release" / ("rinx" + extension)
    subprocess.run([
        "robius-packaging-commands", "before-each-package",
        "--force-makepad", "--binary-name", "rinx",
        "--path-to-binary", str(binary), "--add-deb-dep", "geoclue-2.0",
    ], cwd=ROOT, check=True)
    subprocess.run([sys.executable, str(ROOT / "tools/package-octos.py"), "desktop"],
                   cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
