#!/usr/bin/env python3
"""Isolated, reproducible real-app Sparkle experiment. No production keychain/release.

sdk: download/check the fixed upstream binary SDK.
run: stage two ad-hoc signed Hagency bundles, Ed25519-sign a DMG, serve only
on loopback, and verify replacement/relaunch or rejection using process evidence.
"""
import argparse
import base64
import functools
import hashlib
import http.server
import json
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import tarfile
import tempfile
import threading
import time
import urllib.request
import xml.etree.ElementTree as ET

APP = Path(__file__).resolve().parents[1]
REPO = APP.parents[1]
SDK_VERSION = "2.10.0"
SDK_SHA256 = "c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c"
SDK_URL = f"https://github.com/sparkle-project/Sparkle/releases/download/{SDK_VERSION}/Sparkle-{SDK_VERSION}.tar.xz"
BUNDLE_ID = "org.hagency.desktop.sparkle-prototype"
APP_NAME = "Hagency Sparkle Prototype.app"
NS = "http://www.andymatuschak.org/xml-namespaces/sparkle"


def run(*args, **kwargs):
    return subprocess.run([str(a) for a in args], check=True, **kwargs)


def digest(path):
    value = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def fetch_sdk(destination):
    destination = destination.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    archive = destination / f"Sparkle-{SDK_VERSION}.tar.xz"
    if not archive.is_file():
        urllib.request.urlretrieve(SDK_URL, archive)
    if digest(archive) != SDK_SHA256:
        raise ValueError("Sparkle SDK SHA256 mismatch; refusing extraction")
    # Extract only after validating the pinned official archive. Framework symlinks
    # are essential; reject traversal but preserve internal links/permissions.
    with tarfile.open(archive) as source:
        for member in source.getmembers():
            if not (destination / member.name).resolve().is_relative_to(destination):
                raise ValueError("SDK archive escapes destination")
        source.extractall(destination)
    print(json.dumps({"sdk": str(destination), "version": SDK_VERSION, "sha256": SDK_SHA256}))


def test_key(root):
    """Generate a disposable seed without reading/writing the user's keychain."""
    pem = root / "test-key.pem"
    run("openssl", "genpkey", "-algorithm", "ed25519", "-out", pem, stdout=subprocess.DEVNULL)
    private = subprocess.check_output(["openssl", "pkey", "-in", str(pem), "-outform", "DER"])[-32:]
    public = subprocess.check_output(["openssl", "pkey", "-in", str(pem), "-pubout", "-outform", "DER"])[-32:]
    key = root / "test-key.base64"
    key.write_bytes(base64.b64encode(private))
    pem.chmod(0o600)
    key.chmod(0o600)
    return key, base64.b64encode(public).decode()


def bundle(root, destination, binary, sdk, version, feed, public_key, resource_sources):
    contents = destination / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    resources = contents / "Resources"
    resources.mkdir()
    for name, source in resource_sources:
        run("ditto", source, resources / name / "resources")
    shutil.copy2(binary, contents / "MacOS/hagency")
    (contents / "Frameworks").mkdir()
    run("ditto", sdk / "Sparkle.framework", contents / "Frameworks/Sparkle.framework")
    shutil.copy2(APP / "resources/icon.icns", resources / "icon.icns")
    shutil.copy2(sdk / "LICENSE", resources / "Sparkle-LICENSE")
    # This is a same-source two-build installation fixture; independent Info.plist
    # signatures change the executable, and the resource verifies whole-bundle replacement.
    (resources / "sparkle-prototype-build.txt").write_text(version + "\n")
    info = plistlib.loads((APP / "packaging/macos/Info.plist").read_bytes())
    info.update(CFBundleIdentifier=BUNDLE_ID, CFBundleName="Hagency Sparkle Prototype",
                CFBundleDisplayName="Hagency Sparkle Prototype", CFBundleVersion=version,
                CFBundleShortVersionString=version, LSMinimumSystemVersion="12.0",
                SUFeedURL=feed, SUPublicEDKey=public_key, SUEnableAutomaticChecks=False,
                SUAutomaticallyUpdate=False, SUAllowsAutomaticUpdates=False,
                SUEnableSystemProfiling=False, HagencySparklePrototypeRoot=str(root),
                SUDefaultsDomain="org.hagency.sparkle-prototype." + root.name,
                HagencySparkleTestPreferences=root.name.startswith(("preferences-", "signal-")),
                HagencySparkleSignalTest=root.name.startswith("signal-"))
    # Loopback HTTP is confined to disposable test bundles, never product Info.plist.
    info["NSAppTransportSecurity"] = {"NSAllowsLocalNetworking": True}
    info.pop("CFBundleURLTypes", None)
    (contents / "Info.plist").write_bytes(plistlib.dumps(info))
    framework = contents / "Frameworks/Sparkle.framework/Versions/B"
    for target in [framework / "XPCServices/Installer.xpc", framework / "XPCServices/Downloader.xpc",
                   framework / "Autoupdate", framework / "Updater.app", contents / "Frameworks/Sparkle.framework",
                   contents / "MacOS/hagency", destination]:
        if target.exists():
            # Test only: ad-hoc signing without Hardened Runtime/library validation.
            run("codesign", "--force", "--sign", "-", target)
    run("codesign", "--verify", "--strict", destination)


def appcast(path, url, length, signature):
    ET.register_namespace("sparkle", NS)
    rss = ET.Element("rss", version="2.0")
    channel = ET.SubElement(rss, "channel")
    ET.SubElement(channel, "title").text = "Hagency isolated Sparkle experiment"
    item = ET.SubElement(channel, "item")
    ET.SubElement(item, "title").text = "Prototype build 2.0.0"
    ET.SubElement(item, f"{{{NS}}}minimumSystemVersion").text = "12.0"
    ET.SubElement(item, "enclosure", {
        "url": url, "length": str(length), "type": "application/octet-stream",
        f"{{{NS}}}version": "2.0.0", f"{{{NS}}}shortVersionString": "2.0.0",
        f"{{{NS}}}edSignature": signature,
    })
    ET.ElementTree(rss).write(path, encoding="utf-8", xml_declaration=True)


def events(root):
    path = root / "events.jsonl"
    if not path.exists():
        return []
    result = []
    for line in path.read_text().splitlines():
        try:
            result.append(json.loads(line))
        except json.JSONDecodeError:
            pass  # The live writer may not have finished its last line yet.
    return result


def wait_for(root, condition, description, timeout=120, process=None):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        observed = events(root)
        if condition(observed):
            return observed
        if process is not None and process.poll() is not None:
            raise RuntimeError(f"App exited {process.returncode} before {description}; see {root / 'app.log'}")
        time.sleep(0.1)
    raise TimeoutError(f"Timed out waiting for {description}; evidence: {root}")


def has(observed, kind):
    return any(e["kind"] == "sparkle" and e["detail"]["kind"] == kind for e in observed)


def command(root, pid, cycle, action):
    pending = root / "action.json.pending"
    pending.write_text(json.dumps({"pid": pid, "cycle": cycle, "action": action}))
    pending.replace(root / "action.json")


def stop_owned(root):
    """Only terminate PIDs with both launch evidence and this exact executable path."""
    expected = str(root / "installed" / APP_NAME / "Contents/MacOS/hagency")
    for pid in {e["pid"] for e in events(root) if e["kind"] == "launch"}:
        result = subprocess.run(["ps", "-p", str(pid), "-o", "comm="], capture_output=True, text=True)
        if result.stdout.strip() == expected:
            os.kill(pid, signal.SIGTERM)


def collect_resources():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"],
        cwd=REPO, stderr=subprocess.DEVNULL))
    return [(p["name"].replace("-", "_"), Path(p["manifest_path"]).parent / "resources")
            for p in metadata["packages"] if (Path(p["manifest_path"]).parent / "resources").is_dir()]


def preference_experiment(root, installed):
    legacy = root / "data/updates/macos/preferences.json"
    legacy.parent.mkdir(parents=True)
    legacy.write_text(json.dumps({"auto_check": False, "auto_download": True, "last_check": 0}))
    pids = []
    def launch(expected):
        with (root / "app.log").open("ab") as logfile:
            process = subprocess.Popen([str(installed / "Contents/MacOS/hagency")], cwd=REPO, stdout=logfile, stderr=subprocess.STDOUT)
        pids.append(process.pid)
        wait_for(root, lambda es: any(e["pid"] == process.pid and e["kind"] == "sparkle" and e["detail"]["kind"] == "preferences" and e["detail"]["auto_check"] == expected for e in es), "persisted Sparkle preference", process=process)
        return process
    def choose(process, value):
        prior = len(events(root))
        es = events(root)
        cycle = max([e["detail"].get("cycle", 0) for e in es if e["pid"] == process.pid and e["kind"] == "sparkle"] or [0])
        command(root, process.pid, cycle, "auto_check_on" if value else "auto_check_off")
        wait_for(root, lambda es: any(e["pid"] == process.pid and e["kind"] == "sparkle" and e["detail"]["kind"] == "preferences" and e["detail"]["auto_check"] == value for e in es[prior:]), "explicit preference change")
    def quit_app(process):
        until = time.monotonic() + 15
        while process.poll() is None and time.monotonic() < until:
            es = events(root)
            cycle = max([e["detail"].get("cycle", 0) for e in es if e["pid"] == process.pid and e["kind"] == "sparkle"] or [0])
            command(root, process.pid, cycle, "request_quit")
            time.sleep(0.3)
        assert process.wait(timeout=1) == 0
    first = launch(False)  # Historical JSON imported once; auto_download is ignored.
    choose(first, True)
    wait_for(root, lambda es: any(e["pid"] == first.pid and e["kind"] == "sparkle" and e["detail"]["kind"] == "checking" for e in es), "Sparkle scheduled check after enabling automatic checks")
    wait_for(root, lambda es: any(e["pid"] == first.pid and e["kind"] == "sparkle" and e["detail"]["kind"] == "current" for e in es), "automatic check completes without an update")
    quit_app(first)
    second = launch(True)  # The unchanged old JSON must not override the new choice.
    quit_app(second)
    legacy.write_text("{broken historical preferences")
    third = launch(True)  # Even an unreadable old file is no longer consulted.
    choose(third, False)
    quit_app(third)
    fourth = launch(False)
    quit_app(fourth)
    domain = "org.hagency.sparkle-prototype." + root.name
    run("defaults", "delete", domain, "HagencySparklePreferencesV1")
    run("defaults", "write", domain, "SUEnableAutomaticChecks", "-bool", "true")
    fifth = launch(True)  # A pre-existing Sparkle choice wins even before the marker.
    quit_app(fifth)
    assert not any(e["kind"] == "user_authorized_update" for e in events(root))
    assert not has(events(root), "downloading")
    return {"preference_launch_pids": pids, "migration_once": True, "automatic_check_observed": True, "changed_preference_survives_relaunch": True, "malformed_old_json_ignored_after_migration": True}


def experiment(binary, sdk, scenario, keep_open):
    if os.uname().sysname != "Darwin":
        raise RuntimeError("Requires a native macOS desktop session")
    if not (sdk / "Sparkle.framework/Sparkle").is_file():
        raise ValueError("Use the pinned SDK from the sdk subcommand")
    # Home avoids removable-volume TCC resource/profile stalls observed in this project.
    parent = Path.home() / "Library/Application Support/HagencySparklePrototype"
    parent.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix=f"{scenario}-", dir=parent)).resolve()
    (root / "prototype-marker").write_text("hagency-sparkle-prototype-v1\n")
    (root / "data").mkdir()
    (root / "data/user-sentinel.txt").write_text("preserve profile data across update\n")
    artifacts = REPO / ".run/sparkle-product-artifacts" / root.name
    artifacts.mkdir(parents=True, exist_ok=True)
    (root / "artifact-root.txt").write_text(str(artifacts) + "\n")
    public = artifacts / "public"
    public.mkdir()
    requests = []
    class Handler(http.server.SimpleHTTPRequestHandler):
        def copyfile(self, source, output):
            if scenario == "download-cancel" and self.path.endswith(".dmg"):
                try:
                    while chunk := source.read(64 * 1024):
                        output.write(chunk)
                        output.flush()
                        time.sleep(0.1)
                except (BrokenPipeError, ConnectionResetError):
                    pass  # Expected when Sparkle cancels this exact test response.
            else:
                super().copyfile(source, output)
        def log_message(self, fmt, *args):
            requests.append({"path": self.path, "message": fmt % args})
            (root / "http.json").write_text(json.dumps(requests, indent=2))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(public)))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    feed = f"http://127.0.0.1:{server.server_port}/appcast.xml"
    key, public_key = test_key(root)
    installed = root / "installed" / APP_NAME
    candidate = artifacts / "candidate" / APP_NAME
    print(json.dumps({"root": str(root), "scenario": scenario, "feed": feed}), flush=True)
    report = {"root": str(root), "scenario": scenario, "signing": "ad-hoc app; check/preferences only" if scenario in ("preferences", "signal") else "ad-hoc app + disposable Ed25519 update signature", "notarized": False}
    try:
        resources = collect_resources()
        # Strip once on the project data volume, then independently copy/sign
        # both bundles. Stripping in Home needs another executable-sized temp.
        staging = REPO / ".run"
        staging.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="sparkle-binary-", dir=staging) as scratch:
            staged = Path(scratch) / "hagency"
            shutil.copy2(binary, staged)
            run("strip", "-u", "-r", staged)
            bundle(root, installed, staged, sdk, "1.0.0", feed, public_key, resources)
            if scenario not in ("preferences", "signal"):
                bundle(root, candidate, staged, sdk, "2.0.0", feed, public_key, resources)
        if scenario in ("preferences", "signal"):
            (public / "appcast.xml").write_text('<rss version="2.0"><channel><title>Preference test</title></channel></rss>')
            if scenario == "preferences":
                report.update(preference_experiment(root, installed), result="pass")
            else:
                legacy = root / "data/updates/macos/preferences.json"
                legacy.parent.mkdir(parents=True)
                legacy.write_text(json.dumps({"auto_check": True, "auto_download": False, "last_check": 0}))
                with (root / "app.log").open("wb") as logfile:
                    process = subprocess.Popen([str(installed / "Contents/MacOS/hagency")], cwd=REPO, stdout=logfile, stderr=subprocess.STDOUT)
                observed = wait_for(root, lambda es: has(es, "current"), "signal-driven automatic check without an idle updater timer", process=process)
                assert any(e["kind"] == "bridge_started" and e["detail"]["idle_poll"] is False for e in observed)
                assert any(r["path"] == "/appcast.xml" for r in requests)
                assert not has(observed, "downloading")
                assert process.poll() is None
                report.update(result="pass", signal_driven_check=True, idle_updater_timer=False)
            return
        old_hash = digest(installed / "Contents/MacOS/hagency")
        new_hash = digest(candidate / "Contents/MacOS/hagency")
        assert old_hash != new_hash, "Independently signed executable images must differ"
        dmg = public / "hagency-sparkle-prototype.dmg"
        run("hdiutil", "create", "-quiet", "-volname", "Hagency Sparkle Prototype", "-srcfolder", candidate.parent,
            "-format", "UDZO", dmg)
        signature = subprocess.check_output([str(sdk / "bin/sign_update"), "--ed-key-file", str(key), "-p", str(dmg)], text=True).strip()
        run(sdk / "bin/sign_update", "--verify", "--ed-key-file", key, dmg, signature, stdout=subprocess.DEVNULL)
        # The signed DMG now preserves the exact candidate. Release staging
        # space before Sparkle downloads/extracts a second copy on this volume.
        shutil.rmtree(artifacts / "candidate")
        if scenario == "bad-signature":
            raw = bytearray(base64.b64decode(signature))
            raw[0] ^= 1
            signature = base64.b64encode(raw).decode()
        appcast(public / "appcast.xml", f"http://127.0.0.1:{server.server_port}/{dmg.name}", dmg.stat().st_size, signature)
        with (root / "app.log").open("wb") as logfile:
            process = subprocess.Popen([str(installed / "Contents/MacOS/hagency")], cwd=REPO, stdout=logfile, stderr=subprocess.STDOUT)
        observed = wait_for(root, lambda es: any(e["kind"] == "bridge_started" for e in es), "Makepad/Sparkle startup", process=process)
        pid = next(e["pid"] for e in observed if e["kind"] == "launch")
        command(root, pid, 0, "check")
        observed = wait_for(root, lambda es: has(es, "available") or has(es, "error"), "update selection")
        assert has(observed, "available"), "Sparkle failed to select the test update"
        cycle = next(e["detail"]["cycle"] for e in observed if e["kind"] == "sparkle" and e["detail"]["kind"] == "available")
        if scenario == "busy-cancel":
            command(root, pid, cycle, "hold_task")
            wait_for(root, lambda es: any(e["kind"] == "test_task_held" for e in es), "held task")
        command(root, pid, cycle, "update")
        if scenario == "busy-cancel":
            wait_for(root, lambda es: any(e["kind"] == "user_authorized_update" for e in es), "waiting action")
            time.sleep(1)
            assert not any(e["kind"] == "admission_fenced" for e in events(root))
            assert not any(r["path"].endswith(".dmg") for r in requests)
            command(root, pid, cycle, "request_quit")
            wait_for(root, lambda es: any(e["kind"] == "quit_blocked" for e in es), "AppKit quit blocked while original task runs")
            assert process.poll() is None
            assert not any(r["path"].endswith(".dmg") for r in requests)
            command(root, pid, cycle, "later")
            wait_for(root, lambda es: any(e["kind"] == "waiting_canceled" for e in es), "waiting cancellation")
            command(root, pid, cycle, "release_task")
            wait_for(root, lambda es: any(e["kind"] == "test_task_released" for e in es), "task release")
            time.sleep(1)
            assert not any(r["path"].endswith(".dmg") for r in requests)
            command(root, pid, cycle, "request_quit")
            assert process.wait(timeout=15) == 0, "ordinary quit must work after canceling the wait"
        elif scenario == "download-cancel":
            wait_for(root, lambda es: has(es, "downloading") and any(e["kind"] == "admission_fenced" for e in es), "guarded download")
            command(root, pid, cycle, "later")
            observed = wait_for(root, lambda es: any(e["kind"] == "installation_canceled" for e in es) and any(e["kind"] == "admission_released" for e in es), "native cancellation and admission release")
            assert any(r["path"].endswith(".dmg") for r in requests)
            assert not has(observed, "ready")
            assert not any(e["kind"] == "preparation_started" for e in observed)
            assert digest(installed / "Contents/MacOS/hagency") == old_hash
            assert process.poll() is None
            command(root, pid, cycle, "check")
            wait_for(root, lambda es: any(e["kind"] == "sparkle" and e["detail"]["kind"] == "available" and e["detail"]["cycle"] > cycle for e in es), "fresh check after canceled download")
            assert len([e for e in events(root) if e["kind"] == "launch"]) == 1
        elif scenario == "bad-signature":
            observed = wait_for(root, lambda es: has(es, "error") and any(e["kind"] == "admission_released" for e in es), "signature rejection")
            assert any(e["kind"] == "sparkle" and e["detail"]["kind"] == "error"
                       and "Code=3002" in e["detail"].get("message", "") for e in observed), "Expected Sparkle SUValidationError"
            assert not has(observed, "ready")
            assert digest(installed / "Contents/MacOS/hagency") == old_hash
            assert process.poll() is None
            assert len([e for e in observed if e["kind"] == "launch"]) == 1
            assert any(r["path"].endswith(".dmg") for r in requests)
        else:
            observed = wait_for(root, lambda es: any(e["kind"] == "launch" and e["version"] == "2.0.0" for e in es), "installed application relaunch", timeout=180)
            new_pid = next(e["pid"] for e in observed if e["kind"] == "launch" and e["version"] == "2.0.0")
            assert new_pid != pid
            assert process.wait(timeout=15) == 0
            assert digest(installed / "Contents/MacOS/hagency") == new_hash
            assert (installed / "Contents/Resources/sparkle-prototype-build.txt").read_text() == "2.0.0\n"
            assert any(e["kind"] == "preparation_succeeded" for e in observed)
            assert has(observed, "downloading") and has(observed, "ready")
            assert any(r["path"].endswith(".dmg") for r in requests)
            run("codesign", "--verify", "--strict", installed)
            command(root, new_pid, 0, "check")
            wait_for(root, lambda es: any(e["pid"] == new_pid and e["kind"] == "sparkle" and e["detail"]["kind"] == "current" for e in es), "new version sees no newer update")
            report.update(old_pid=pid, new_pid=new_pid, old_executable_sha256=old_hash, new_executable_sha256=new_hash)
        assert (root / "data/user-sentinel.txt").read_text() == "preserve profile data across update\n"
        report["result"] = "pass"
    except BaseException as error:
        report.update(result="fail", error=str(error))
        raise
    finally:
        (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        server.shutdown()
        server.server_close()
        # Private fixture keys are never retained with the report, even on failure.
        key.unlink(missing_ok=True)
        (root / "test-key.pem").unlink(missing_ok=True)
        if not keep_open or report.get("result") != "pass":
            stop_owned(root)
        if report.get("result") == "pass":
            # Retain logs/reports, not gigabytes of duplicate fixture candidates.
            shutil.rmtree(artifacts / "candidate", ignore_errors=True)
            shutil.rmtree(public)
        print(json.dumps(report), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sdk_parser = sub.add_parser("sdk")
    sdk_parser.add_argument("--destination", type=Path, default=REPO / ".run/sparkle-sdk")
    run_parser = sub.add_parser("run")
    run_parser.add_argument("--sdk", type=Path, default=REPO / ".run/sparkle-sdk")
    run_parser.add_argument("--binary", type=Path, default=REPO / "target/debug/hagency")
    run_parser.add_argument("--scenario", choices=["success", "bad-signature", "busy-cancel", "preferences", "download-cancel", "signal"], default="success")
    run_parser.add_argument("--keep-open", action="store_true")
    args = parser.parse_args()
    if args.command == "sdk":
        fetch_sdk(args.destination)
    else:
        experiment(args.binary.resolve(), args.sdk.resolve(), args.scenario, args.keep_open)


if __name__ == "__main__":
    main()
