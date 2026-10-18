#!/usr/bin/env python3
"""Run on Mini 1 to provision an isolated, loopback-only Rinx test homeserver.

Idempotent: preserves the database and credentials on repeat runs. Does not touch
other Palpo deployments. Credentials stay in a mode-0600 file on the server.
"""
import json
import os
from pathlib import Path
import secrets
import subprocess

NAME = "robrix-mobile-soak"
PORT = 18120
IMAGE = "palpo-url-cas:056f5966f5b186d6"
DOCKER = "/opt/homebrew/bin/docker"
root = Path.home() / NAME
root.mkdir(mode=0o700, exist_ok=True)
os.chmod(root, 0o700)


def docker(*args, check=True):
    return subprocess.run([DOCKER, *args], check=check, capture_output=True, text=True)


def exists(kind, name):
    return docker(kind, "inspect", name, check=False).returncode == 0


credentials = root / "credentials.json"
if credentials.exists():
    config = json.loads(credentials.read_text())
else:
    config = {"db_password": secrets.token_urlsafe(32), "registration_token": secrets.token_urlsafe(32)}
    credentials.write_text(json.dumps(config))
os.chmod(credentials, 0o600)
server_name = "robrix-mobile.test"
toml = root / "palpo.toml"
toml.write_text(f'''server_name = "{server_name}"
allow_registration = true
registration_token = "{config['registration_token']}"
rc_login = {{ per_second = 0.1, burst = 20 }}
rc_registration = {{ per_second = 0, burst = 1 }}
rc_message = {{ per_second = 0, burst = 1 }}
[[listeners]]
address = "0.0.0.0:8008"
[db]
url = "postgres://palpo:{config['db_password']}@{NAME}-db:5432/palpo"
[well_known]
server = "{server_name}:{PORT}"
client = "http://127.0.0.1:{PORT}"
''')
os.chmod(toml, 0o600)
env = root / "postgres.env"
env.write_text(f"POSTGRES_USER=palpo\nPOSTGRES_DB=palpo\nPOSTGRES_PASSWORD={config['db_password']}\n")
os.chmod(env, 0o600)
if not exists("network", NAME):
    docker("network", "create", NAME)
for suffix in ("db", "data"):
    (root / suffix).mkdir(exist_ok=True)
if not exists("container", f"{NAME}-db"):
    docker("run", "-d", "--name", f"{NAME}-db", "--network", NAME,
           "--restart", "unless-stopped", "--env-file", str(env),
           "-v", f"{root / 'db'}:/var/lib/postgresql/data", "postgres:16-alpine")
else:
    docker("start", f"{NAME}-db")
# pg_isready waits inside the database container; no credentials printed.
docker("exec", f"{NAME}-db", "sh", "-c",
       "until pg_isready -U palpo -d palpo >/dev/null; do sleep 1; done")
if not exists("container", f"{NAME}-hs"):
    docker("run", "-d", "--name", f"{NAME}-hs", "--network", NAME,
           "--restart", "unless-stopped", "-p", f"127.0.0.1:{PORT}:8008",
           "-v", f"{toml}:/var/palpo/palpo.toml:ro",
           "-v", f"{root / 'data'}:/var/palpo/data", IMAGE)
else:
    docker("start", f"{NAME}-hs")
print(json.dumps({"namespace": NAME, "server_name": server_name, "port": PORT, "image": IMAGE}))
