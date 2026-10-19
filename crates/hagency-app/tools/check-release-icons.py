"""Check the real packager icon inputs before expensive installer builds."""

from pathlib import Path
import re
import struct


root = Path(__file__).resolve().parents[1]
manifest = (root / "Cargo.toml").read_text(encoding="utf-8")
block = re.search(r"(?ms)^icons\s*=\s*\[(.*?)^\]", manifest)
assert block, "Missing packager icons"
icons = [root / name for name in re.findall(r'"([^"]+)"', block[1])]
assert all(path.is_file() for path in icons), "Missing packager icon input"
icns = [path for path in icons if path.suffix == ".icns"]
assert len(icns) == 1, "macOS requires the generated ICNS input"
data = icns[0].read_bytes()
assert data[:4] == b"icns" and struct.unpack(">I", data[4:8])[0] == len(data), "Invalid ICNS container"
offset = 8
sizes = set()
while offset < len(data):
    kind, length = struct.unpack(">4sI", data[offset:offset + 8])
    assert length >= 8 and offset + length <= len(data), "Invalid ICNS chunk"
    payload = data[offset + 8:offset + length]
    if payload.startswith(b"\x89PNG\r\n\x1a\n"):
        assert payload[12:16] == b"IHDR", "Missing ICNS PNG dimensions"
        sizes.add(struct.unpack(">II", payload[16:24]))
    offset += length
assert offset == len(data) and {(128, 128), (256, 256), (512, 512), (1024, 1024)} <= sizes, "Missing macOS icon representations"
assert any(path.suffix == ".ico" for path in icons), "Missing Windows icon"
assert any(path.suffix == ".png" for path in icons), "Missing Linux icons"

# The exe's ICO does not control Makepad's unbundled window/taskbar icon.
# Check the dependency build-script inputs too: its automatic discovery fails
# with an external target directory or the application's nested source layout.
workspace = root.parent.parent
environment = (workspace / ".cargo/config.toml").read_text(encoding="utf-8")


def runtime_icon_path(name):
    # Keep this release preflight compatible with Python 3.10 runners.
    entry = re.search(rf"(?m)^{re.escape(name)}\s*=\s*\{{([^\n}}]+)\}}", environment)
    assert entry, f"Missing runtime icon override: {name}"
    assert re.search(r"\brelative\s*=\s*true", entry[1]), "Runtime icon path must be source-relative"
    assert re.search(r"\bforce\s*=\s*true", entry[1]), "Runtime icon override must be explicit"
    value = re.search(r'\bvalue\s*=\s*"([^"]+)"', entry[1])
    assert value, f"Missing runtime icon path: {name}"
    return workspace / value[1]


for size in (32, 64, 128, 256, 512, 1024):
    path = runtime_icon_path(f"MAKEPAD_APP_ICON_{size}")
    assert path == root / f"resources/icon_{size}.png", "Runtime/packaged icon mismatch"
    png = path.read_bytes()
    assert png[:8] == b"\x89PNG\r\n\x1a\n"
    assert struct.unpack(">II", png[16:24]) == (size, size), "Incorrect runtime icon dimensions"
assert runtime_icon_path("MAKEPAD_APP_ICON_ICO") == root / "resources/icon.ico"
print("Release and development window icons validated: explicit ICNS, ICO and multi-size PNG inputs")
