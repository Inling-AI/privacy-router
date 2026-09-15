"""Stage one native executable; Cargo.toml owns the release version."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tomllib


class ReleasePackage:
    def __init__(self):
        self.root = Path(__file__).resolve().parent.parent
        with (self.root / "Cargo.toml").open("rb") as manifest:
            self.version = tomllib.load(manifest)["workspace"]["package"]["version"]

    def verify_tag(self, tag):
        expected = f"v{self.version}"
        if tag != expected:
            raise SystemExit(f"Release tag {tag!r} must match Cargo.toml: {expected}")

    def build(self, target):
        if os.environ.get("GITHUB_REF_TYPE") == "tag":
            self.verify_tag(os.environ["GITHUB_REF_NAME"])
        executable = "privacy-router.exe" if "windows" in target else "privacy-router"
        binary = self.root / "target" / target / "release" / executable
        subprocess.run([str(binary), "--help"], check=True, timeout=30)
        version = subprocess.check_output([str(binary), "--version"], text=True, timeout=30)
        if version.strip() != f"privacy-router {self.version}":
            raise SystemExit(f"Unexpected binary version: {version!r}")

        name = f"privacy-router-{self.version}-{target}"
        if "windows" in target:
            name += ".exe"
        destination = self.root / "dist"
        destination.mkdir(exist_ok=True)
        output = destination / name
        shutil.copy2(binary, output)
        print(output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target")
    parser.add_argument("--verify-release")
    args = parser.parse_args()
    release = ReleasePackage()
    if args.verify_release:
        release.verify_tag(args.verify_release)
    elif args.target:
        release.build(args.target)
    else:
        parser.error("provide --target, or --verify-release")
