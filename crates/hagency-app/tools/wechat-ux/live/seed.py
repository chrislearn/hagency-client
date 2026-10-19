#!/usr/bin/env python3
"""Create test-only Matrix users and deterministic rooms on the isolated Palpo.

Secrets are read over SSH and written only to a private, ignored evidence file.
Repeat runs reuse the fixture rather than create duplicate accounts or rooms.
"""
import argparse
import json
import os
from pathlib import Path
import secrets
import subprocess
import urllib.error
import urllib.parse
import urllib.request


def api(base, method, path, data=None, token=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + token
    request = urllib.request.Request(base + "/_matrix/client/v3/" + path,
                                     data=None if data is None else json.dumps(data).encode(),
                                     headers=headers, method=method)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, json.load(error)


def checked(*args, **kwargs):
    status, result = api(*args, **kwargs)
    if not 200 <= status < 300:
        raise RuntimeError(f"Matrix {args[1]} {args[2]}: HTTP {status}, {result.get('errcode')}: {result.get('error')}")
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", default="http://127.0.0.1:18120")
    parser.add_argument("--ssh", default=os.environ.get("ROBRIX_TEST_SSH"),
                        help="SSH destination for the isolated fixture server (or ROBRIX_TEST_SSH)")
    parser.add_argument("--output", type=Path, default=Path("lab/wechat-ux/evidence/live/fixture.json"))
    args = parser.parse_args()
    if args.output.exists():
        fixture = json.loads(args.output.read_text())
        for account in fixture["users"].values():
            checked(args.url, "GET", "account/whoami", token=account["access_token"])
        print(json.dumps({"status": "reused", "users": len(fixture["users"]), "rooms": len(fixture["rooms"])}))
        return
    if not args.ssh:
        parser.error("set --ssh or ROBRIX_TEST_SSH to provision an isolated fixture")
    remote = subprocess.run(["ssh", "-o", "BatchMode=yes", args.ssh,
                             "cat ~/robrix-mobile-soak/credentials.json"],
                            check=True, capture_output=True, text=True)
    registration_token = json.loads(remote.stdout)["registration_token"]
    users = {}
    # The fixture suffix avoids clashes if a previous run ended before persistence.
    suffix = secrets.token_hex(3)
    for key, name in (("alex", "Alex Chen"), ("emma", "Emma Wilson"), ("leo", "Leo Zhang"), ("nora", "Nora Patel")):
        password = secrets.token_urlsafe(28)
        body = {"username": f"robrix_ux_{key}_{suffix}", "password": password,
                "initial_device_display_name": "Rinx UX fixture"}
        status, response = api(args.url, "POST", "register", body)
        for _ in range(5):
            if status != 401:
                break
            completed = response.get("completed", [])
            stages = next((flow["stages"] for flow in response.get("flows", [])
                           if set(flow["stages"]) <= {"m.login.registration_token", "m.login.dummy"}), [])
            pending = [stage for stage in stages if stage not in completed]
            if not pending:
                raise RuntimeError("Unsupported registration challenge")
            auth = {"type": pending[0], "session": response["session"]}
            if pending[0] == "m.login.registration_token":
                auth["token"] = registration_token
            status, response = api(args.url, "POST", "register", {**body, "auth": auth})
        if status != 200:
            raise RuntimeError(f"Registration failed: HTTP {status}, {response.get('errcode')}")
        users[key] = {**response, "password": password, "display_name": name}
        checked(args.url, "PUT", f"profile/{response['user_id']}/displayname", {"displayname": name}, response["access_token"])

    alex = users["alex"]
    rooms = {}
    direct = {}
    for key, preview in (("emma", "See you at the café tomorrow ☕"), ("leo", "The photos look great!"), ("nora", "Thanks! See you soon.")):
        friend = users[key]
        room = checked(args.url, "POST", "createRoom", {
            "is_direct": True, "preset": "trusted_private_chat", "invite": [friend["user_id"]],
        }, alex["access_token"])["room_id"]
        checked(args.url, "POST", f"join/{room}", {}, friend["access_token"])
        direct[friend["user_id"]] = [room]
        checked(args.url, "PUT", f"user/{friend['user_id']}/account_data/m.direct", {alex["user_id"]: [room]}, friend["access_token"])
        for i, (sender, text) in enumerate(((alex, "Hey! How is your day going?"), (friend, "Really good, thanks. How about you?"), (alex, "Looking forward to the weekend."), (friend, preview))):
            checked(args.url, "PUT", f"rooms/{room}/send/m.room.message/seed-{i}", {"msgtype": "m.text", "body": text}, sender["access_token"])
        rooms[key] = room
    checked(args.url, "PUT", f"user/{alex['user_id']}/account_data/m.direct", direct, alex["access_token"])
    for key, name, message in (("weekend", "Weekend Plans", "Saturday at 10? Let’s meet by the lake."), ("design", "Design Studio", "I’ve shared the latest design. Take a look when you can.")):
        room = checked(args.url, "POST", "createRoom", {"name": name, "preset": "private_chat", "invite": [users[k]["user_id"] for k in ("emma", "leo", "nora")]}, alex["access_token"])["room_id"]
        for friend in (users[k] for k in ("emma", "leo", "nora")):
            checked(args.url, "POST", f"join/{room}", {}, friend["access_token"])
        checked(args.url, "PUT", f"rooms/{room}/send/m.room.message/seed-0", {"msgtype": "m.text", "body": message}, users["emma"]["access_token"])
        rooms[key] = room

    args.output.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as output:
        json.dump({"url": args.url, "users": users, "rooms": rooms}, output, indent=2)
    print(json.dumps({"status": "created", "users": len(users), "rooms": len(rooms)}))


if __name__ == "__main__":
    main()
