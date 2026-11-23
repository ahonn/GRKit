#!/usr/bin/env python3
"""Serve a locally signed release fixture for emulator testing."""

import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import tempfile

APP = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True,
                        help="Extracted official site artifact supplying effect files and guides")
    parser.add_argument("--port", type=int, default=18081)
    args = parser.parse_args()
    cache = APP / ".nsh"
    cache.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mock-release-", dir=cache) as temporary:
        site = Path(temporary) / "site"
        subprocess.run([
            "cargo", "run", "--quiet", "--manifest-path", str(APP / "Cargo.toml"),
            "--example", "mock_release", "--", "--source", str(args.source.resolve()),
            "--out", str(site),
        ], check=True)
        key = (site / "test-public-key.hex").read_text().strip()
        handler = partial(SimpleHTTPRequestHandler, directory=str(site))
        with ThreadingHTTPServer(("127.0.0.1", args.port), handler) as server:
            print(f"GRKIT_RELEASE_BASE=http://127.0.0.1:{args.port}", flush=True)
            print(f"GRKIT_RELEASE_PUBLIC_KEY={key}", flush=True)
            print(f"adb reverse tcp:{args.port} tcp:{args.port}", flush=True)
            print(f"Fixture directory: {site}", flush=True)
            print("Test-signed fixture only; production camera services reject this key.", flush=True)
            try:
                server.serve_forever()
            except KeyboardInterrupt:
                pass


if __name__ == "__main__":
    main()
