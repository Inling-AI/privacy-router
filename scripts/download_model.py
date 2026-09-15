"""Compatibility entry point; model metadata and downloads are owned by Rust."""

import argparse
from pathlib import Path
import subprocess


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", nargs="?")
    args = parser.parse_args()
    command = ["cargo", "run", "--release", "--manifest-path",
               str(Path(__file__).resolve().parents[1] / "Cargo.toml"),
               "-p", "privacy-router", "--"]
    if args.directory is not None:
        command += ["--model-dir", args.directory]
    command += ["model", "download"]
    raise SystemExit(subprocess.call(command))
