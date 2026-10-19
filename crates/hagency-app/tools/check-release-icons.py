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
print("Release icons validated: explicit ICNS, Windows ICO and Linux PNG inputs")
