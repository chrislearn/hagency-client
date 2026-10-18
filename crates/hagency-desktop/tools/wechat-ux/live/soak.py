#!/usr/bin/env python3
"""Live Palpo durability/interop soak using only accounts created by seed.py.

Checks transaction idempotency, recipient visibility, edits, receipts, redaction,
profile reads, sync cursors, and concurrent client sessions. This measures the
server/API path, not the native UI; native interaction evidence is separate.
"""
import argparse
import json
from pathlib import Path
import sys
import time
import uuid
from seed import checked


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", type=Path, default=Path("lab/wechat-ux/evidence/live/fixture.json"))
    parser.add_argument("--minutes", type=float, default=30)
    parser.add_argument("--interval", type=float, default=3)
    parser.add_argument("--output", type=Path, default=Path("lab/wechat-ux/evidence/live/server-soak.jsonl"))
    args = parser.parse_args()
    fixture = json.loads(args.fixture.read_text())
    base = fixture["url"]
    sender, recipient = fixture["users"]["nora"], fixture["users"]["leo"]
    run = uuid.uuid4().hex
    room = checked(base, "POST", "createRoom", {"name": "Rinx isolated soak " + run[:8], "preset": "private_chat", "invite": [recipient["user_id"]]}, sender["access_token"])["room_id"]
    checked(base, "POST", f"join/{room}", {}, recipient["access_token"])
    started = time.monotonic()
    deadline = started + args.minutes * 60
    failures = 0
    iterations = 0
    checks = 0
    latencies = []
    cursor = None
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w") as log:
        log.write(json.dumps({"kind": "start", "run": run, "room": room, "duration_minutes": args.minutes, "server": base, "started_at": time.time()}) + "\n")
        while time.monotonic() < deadline:
            iteration_start = time.monotonic()
            try:
                body = f"soak {run} {iterations} — Hello 你好 👋"
                path = f"rooms/{room}/send/m.room.message/{run}-{iterations}"
                content = {"msgtype": "m.text", "body": body}
                event = checked(base, "PUT", path, content, sender["access_token"])["event_id"]
                repeated = checked(base, "PUT", path, content, sender["access_token"])["event_id"]
                assert repeated == event, "Repeated transaction created a second event"
                checks += 1
                received = checked(base, "GET", f"rooms/{room}/event/{event}", token=recipient["access_token"])
                assert received["content"]["body"] == body and received["sender"] == sender["user_id"], "Recipient content mismatch"
                checks += 1
                checked(base, "POST", f"rooms/{room}/receipt/m.read/{event}", {}, recipient["access_token"])
                checks += 1
                if iterations % 5 == 0:
                    edit = {"msgtype": "m.text", "body": "* " + body + " edited",
                            "m.new_content": {"msgtype": "m.text", "body": body + " edited"},
                            "m.relates_to": {"rel_type": "m.replace", "event_id": event}}
                    edit_id = checked(base, "PUT", path + "-edit", edit, sender["access_token"])["event_id"]
                    edit_read = checked(base, "GET", f"rooms/{room}/event/{edit_id}", token=recipient["access_token"])
                    assert edit_read["content"]["m.relates_to"]["event_id"] == event, "Edit relation missing"
                    checks += 1
                if iterations % 10 == 0:
                    checked(base, "PUT", f"rooms/{room}/redact/{event}/{run}-{iterations}-redact", {"reason": "Isolated soak cleanup"}, sender["access_token"])
                    redacted = checked(base, "GET", f"rooms/{room}/event/{event}", token=recipient["access_token"])
                    assert "body" not in redacted["content"], "Redacted message still has its body"
                    checks += 1
                    profile = checked(base, "GET", f"profile/{sender['user_id']}", token=recipient["access_token"])
                    assert profile["displayname"] == sender["display_name"], "Profile mismatch"
                    checks += 1
                # Sync, rather than event GET alone, catches broken live delivery.
                sync_path = "sync?timeout=0" + ("&since=" + cursor if cursor else "")
                sync = checked(base, "GET", sync_path, token=recipient["access_token"])
                cursor = sync["next_batch"]
                events = sync.get("rooms", {}).get("join", {}).get(room, {}).get("timeline", {}).get("events", [])
                assert any(e.get("event_id") == event for e in events), "Recipient sync did not deliver the message"
                checks += 1
                latency = time.monotonic() - iteration_start
                latencies.append(latency)
                record = {"kind": "iteration", "index": iterations, "passed": True, "event_id": event, "seconds": latency}
            except Exception as error:
                failures += 1
                record = {"kind": "iteration", "index": iterations, "passed": False, "error": str(error)}
            log.write(json.dumps(record, ensure_ascii=False) + "\n")
            log.flush()
            iterations += 1
            if iterations % 20 == 0:
                print(json.dumps({"iterations": iterations, "failures": failures, "elapsed_seconds": round(time.monotonic() - started)}), flush=True)
            time.sleep(min(args.interval, max(0, deadline - time.monotonic())))
        latencies.sort()
        summary = {"kind": "summary", "passed": failures == 0 and iterations > 0, "iterations": iterations,
                   "checks": checks, "failures": failures, "elapsed_seconds": time.monotonic() - started,
                   "p95_iteration_seconds": latencies[min(len(latencies)-1, int(len(latencies)*0.95))] if latencies else None}
        log.write(json.dumps(summary) + "\n")
        print(json.dumps(summary), flush=True)
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
