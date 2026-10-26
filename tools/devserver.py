#!/usr/bin/env python3
"""Development server: serves ui/ and runs the debug binary as a CGI for
api.cgi/*, the way DSM's web server does. Authentication is replaced by
BTRBK_NAS_DEV_USER, which only debug builds honour.

    cargo build && tools/devserver.py [--sudo] [--port 8765]

With --demo the API answers come from tools/demo.py instead: made-up data
for screenshots, no binary and no btrfs needed. The page then also loads
tools/demo-shot.js, which opens a dialog or queues deletions when the URL
asks (?shot=settings, ?shot=queue).
"""

import argparse
import http.server
import json
import os
import subprocess
import sys
from pathlib import Path
from urllib.parse import parse_qsl, urlsplit

import demo

ROOT = Path(__file__).resolve().parent.parent
UI = ROOT / "ui"
BINARY = ROOT / "target" / "debug" / "btrbk-nas"
VAR = ROOT / "target" / "devvar"
TOOLS = ROOT / "tools"


class Handler(http.server.SimpleHTTPRequestHandler):
    use_sudo = False
    demo = False

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(UI), **kwargs)

    def do_GET(self):
        if self.path.startswith("/webman/login.cgi"):
            return self.reply(200, "application/json", json.dumps({"SynoToken": "dev"}).encode())
        if self.path.startswith("/api.cgi/"):
            return self.cgi()
        if self.demo and urlsplit(self.path).path in ("/", "/index.html"):
            page = (UI / "index.html").read_text().replace(
                "</body>", '<script src="demo-shot.js"></script>\n</body>')
            return self.reply(200, "text/html; charset=utf-8", page.encode())
        if self.demo and self.path == "/demo-shot.js":
            return self.reply(200, "text/javascript", (TOOLS / "demo-shot.js").read_bytes())
        return super().do_GET()

    def do_POST(self):
        if self.path.startswith("/api.cgi/"):
            return self.cgi()
        self.send_error(404)

    def reply(self, status, content_type, body):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def cgi(self):
        url = urlsplit(self.path)
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length) if length else b""
        if self.demo:
            params = dict(parse_qsl(url.query))
            status, answer = demo.respond(self.command, url.path[len("/api.cgi/"):], params,
                                          json.loads(body or b"{}"))
            return self.reply(status, "application/json", json.dumps(answer).encode())
        env = {
            "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
            "GATEWAY_INTERFACE": "CGI/1.1",
            "REQUEST_METHOD": self.command,
            "PATH_INFO": url.path[len("/api.cgi"):],
            "QUERY_STRING": url.query,
            "CONTENT_TYPE": self.headers.get("Content-Type", ""),
            "CONTENT_LENGTH": str(length),
            "BTRBK_NAS_DEV_USER": "dev",
            "SYNOPKG_PKGVAR": str(VAR),
        }
        cmd = [str(BINARY)]
        if self.use_sudo:
            cmd = ["sudo", "-n", "env", *(f"{k}={v}" for k, v in env.items()), *cmd]
        out = subprocess.run(cmd, input=body, env=env, capture_output=True, check=False).stdout
        head, _, payload = out.partition(b"\r\n\r\n")
        status = 200
        headers = []
        for line in head.decode("latin-1").split("\r\n"):
            name, _, value = line.partition(": ")
            if name.lower() == "status":
                status = int(value.split()[0])
            elif name:
                headers.append((name, value))
        self.send_response(status)
        for name, value in headers:
            self.send_header(name, value)
        self.end_headers()
        self.wfile.write(payload)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--sudo", action="store_true", help="run the CGI as root, for btrfs metadata")
    parser.add_argument("--demo", action="store_true", help="answer with made-up data (tools/demo.py)")
    args = parser.parse_args()
    if not args.demo and not BINARY.exists():
        sys.exit("build first: cargo build")
    VAR.mkdir(parents=True, exist_ok=True)
    Handler.use_sudo = args.sudo
    Handler.demo = args.demo
    print(f"http://127.0.0.1:{args.port}/index.html")
    http.server.ThreadingHTTPServer(("127.0.0.1", args.port), Handler).serve_forever()


if __name__ == "__main__":
    main()
