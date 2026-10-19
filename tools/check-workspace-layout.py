"""Validate the monorepo boundary and local dependency graph without compiling."""
import json
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--locked", "--format-version", "1"],
        cwd=ROOT, text=True,
    ))
    members = set(metadata["workspace_members"])
    packages = [p for p in metadata["packages"] if p["id"] in members]
    assert packages, "Empty workspace"
    for package in packages:
        manifest = Path(package["manifest_path"]).resolve()
        assert manifest.parent.parent == ROOT / "crates", manifest
        assert not (manifest.parent / "Cargo.lock").exists(), manifest
    app = next(p for p in packages if p["name"] == "hagency-app")
    assert Path(app["manifest_path"]).parent == ROOT / "crates/hagency-app"
    assert app["default_run"] == "hagency"
    assert [t["name"] for t in app["targets"] if "bin" in t["kind"]] == ["hagency"]
    engine = next(p for p in app["dependencies"] if p["name"] == "hagency")
    assert engine["source"] is None and Path(engine["path"]).resolve() == ROOT / "crates/hagency"
    files = subprocess.check_output([
        "git", "ls-files", "--cached", "--others", "--exclude-standard", "-z",
    ], cwd=ROOT).decode().split("\0")
    for name in files:
        if not name.endswith("Cargo.toml") or not (ROOT / name).is_file():
            continue
        data = tomllib.loads((ROOT / name).read_text(encoding="utf-8"))
        if "package" in data:
            assert Path(name).parts[0] == "crates", f"Crate outside crates/: {name}"
            assert "workspace" not in data, f"Nested workspace: {name}"
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
    assert not any("github.com/chrislearn/hagency-client" in p.get("source", "") for p in lock["package"]), "Duplicate Git engine"
    cli = next(p for p in packages if p["name"] == "hagency")
    assert cli["default_run"] == "hagency-cli"
    binaries = [t["name"] for p in packages for t in p["targets"] if "bin" in t["kind"]]
    assert len(binaries) == len(set(binaries)), "Colliding workspace executable names"
    print(json.dumps({"packages": len(packages), "app_engine": engine["path"],
                      "target_directory": metadata["target_directory"]}))


if __name__ == "__main__":
    main()
