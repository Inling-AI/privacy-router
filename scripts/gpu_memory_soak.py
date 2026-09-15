"""Exercise a GPU router over HTTP with an isolated database and a local fake upstream.

Build with cargo build --release -p privacy-router --features gpu first.
RSS is sampled with ps (KiB); request and upstream contents never leave loopback.
"""

import argparse
import concurrent.futures
import http.server
import json
import re
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request


class Upstream(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers["Content-Length"]))
        if b"@example.com" in body:
            self.send_error(500, "email was not redacted")
            return
        response = json.dumps({"choices": [{"message": {"role": "assistant", "content": "ok"}}]}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)

    def log_message(self, *_):
        pass


class Soak:
    def __init__(self, args):
        self.args = args
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            self.port = listener.getsockname()[1]

    def request(self, path, data=None, headers=None):
        request = urllib.request.Request(
            f"http://127.0.0.1:{self.port}{path}",
            data=None if data is None else json.dumps(data).encode(),
            headers={"Content-Type": "application/json", **(headers or {})},
        )
        with urllib.request.urlopen(request, timeout=600) as response:
            return json.load(response)

    def infer(self, index):
        text = (f"Session {index}. Please email candidate.{index}@example.com with the deployment summary. "
                "The engineering team reviewed the rollout and requested another verification of the configuration. ")
        body = {"model": "soak", "messages": [
            {"role": "user", "content": f"Message {field}. " + text * (self.args.repeat + index % self.args.vary)}
            for field in range(self.args.fields)
        ]}
        result = self.request("/v1/chat/completions", body, {"x-privacy-router-provider": "soak"})
        assert result["choices"][0]["message"]["content"] == "ok"

    def run(self):
        with tempfile.TemporaryDirectory(prefix="privacy-gpu-soak-") as temporary:
            upstream = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Upstream)
            threading.Thread(target=upstream.serve_forever, daemon=True).start()
            log_path = Path(self.args.log).resolve()
            with log_path.open("w") as log:
                process = subprocess.Popen([
                    str(Path(self.args.binary).resolve()), "--bind", f"127.0.0.1:{self.port}",
                    "--database", str(Path(temporary) / "router.db"), "--model-dir", self.args.model_dir,
                    "--backend", "gpu", "--log-filter", "info", "--log-format", "json",
                ], stdout=log, stderr=log)
                try:
                    for _ in range(180):
                        if process.poll() is not None:
                            raise RuntimeError(f"router exited; see {log_path}")
                        try:
                            self.request("/api/health")
                            break
                        except (urllib.error.URLError, ConnectionError):
                            time.sleep(1)
                    else:
                        raise TimeoutError("router startup")
                    if '"backend":"gpu"' not in log_path.read_text():
                        raise RuntimeError("this probe requires a GPU-enabled router binary")
                    session = self.request("/api/setup", {"username": "soak", "password": "isolated-soak-password"})
                    self.request("/api/providers", {
                        "name": "soak", "base_url": f"http://127.0.0.1:{upstream.server_port}/v1",
                        "api_format": "openai_chat", "enabled": True,
                    }, {"Authorization": f"Bearer {session['token']}"})
                    print(f"pid={process.pid} log={log_path}", flush=True)
                    self.report(process, 0)
                    with concurrent.futures.ThreadPoolExecutor(max_workers=self.args.concurrency) as pool:
                        for round_index in range(self.args.rounds):
                            start = round_index * self.args.concurrency
                            list(pool.map(self.infer, range(start, start + self.args.concurrency)))
                            footprint = self.report(process, start + self.args.concurrency)
                            if footprint > self.args.stop_gib * 1024:
                                print("stopped at configured physical footprint ceiling", flush=True)
                                break
                            time.sleep(self.args.pause)
                finally:
                    process.terminate()
                    try:
                        process.wait(timeout=15)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
                    upstream.shutdown()
                    upstream.server_close()

    def report(self, process, completed):
        rss = int(subprocess.check_output(["ps", "-o", "rss=", "-p", str(process.pid)], text=True).strip())
        snapshot = subprocess.check_output(["footprint", "-p", str(process.pid), "-f", "bytes"], text=True)
        path = Path(self.args.log).with_suffix(f".{completed}.footprint.txt")
        path.write_text(snapshot)
        match = re.search(r"phys_footprint:\s+(\d+) B", snapshot)
        if match is None:
            raise RuntimeError(f"could not read physical footprint: {path}")
        footprint = int(match.group(1)) / 1024**2
        print(f"completed={completed} rss_mib={rss / 1024:.1f} footprint_mib={footprint:.1f} snapshot={path}", flush=True)
        return footprint


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/release/privacy-router")
    parser.add_argument("--model-dir", default="models/privacy-filter")
    parser.add_argument("--rounds", type=int, default=10)
    parser.add_argument("--concurrency", type=int, default=8)
    parser.add_argument("--repeat", type=int, default=8)
    parser.add_argument("--fields", type=int, default=1)
    parser.add_argument("--vary", type=int, default=7)
    parser.add_argument("--pause", type=float, default=0)
    parser.add_argument("--stop-gib", type=float, default=22)
    parser.add_argument("--log", default="/tmp/privacy-gpu-soak.log")
    Soak(parser.parse_args()).run()
