#!/usr/bin/env python3
"""Package an already-built native binary; Python 3.9+, standard library only."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import struct
import sys
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent
TARGETS = {
    "aarch64-apple-darwin", "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc",
    "x86_64-unknown-freebsd",
}


def release_label():
    found = re.search(r'pub const VERSION: &str = "(v0\.[0-9↊↋]+\.[0-9↊↋]+)";',
                      (ROOT / "src/lib.rs").read_text(encoding="utf-8"))
    if found is None:
        raise ValueError("Cannot find VRX's dozenal release label")
    return found.group(1)


def verify_target(binary, target):
    with binary.open("rb") as source:
        header = source.read(64)
        if "apple" in target:
            expected = 0x0100000C if target.startswith("aarch64") else 0x01000007
            valid = header[:4] == b"\xcf\xfa\xed\xfe" and len(header) >= 8 and struct.unpack_from("<I", header, 4)[0] == expected
        elif "windows" in target:
            valid = False
            if len(header) >= 64 and header[:2] == b"MZ":
                source.seek(struct.unpack_from("<I", header, 60)[0])
                pe = source.read(6)
                valid = pe == b"PE\x00\x00\x64\x86"
        else:
            valid = len(header) >= 20 and header[:6] == b"\x7fELF\x02\x01" and struct.unpack_from("<H", header, 18)[0] == 62
            if valid:
                valid = header[7] == 9 if "freebsd" in target else header[7] in (0, 3)
    if not valid:
        raise ValueError(f"Binary header does not match release target {target}")


def copy_documentation(destination):
    destination.mkdir(parents=True, exist_ok=True)
    for name in ["README.md", "LICENSE"]:
        shutil.copy2(ROOT / name, destination / name)
    shutil.copytree(ROOT / "docs", destination / "docs")
    shutil.copy2(ROOT / "packaging/THIRD_PARTY.md", destination / "THIRD_PARTY.md")
    shutil.copytree(ROOT / "packaging/licenses", destination / "licenses")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", choices=sorted(TARGETS), required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    verify_target(binary, args.target)
    label = release_label()
    if os.environ.get("GITHUB_REF_TYPE") == "tag" and os.environ.get("GITHUB_REF_NAME") != label:
        raise ValueError("Git tag must match src/lib.rs VERSION exactly")
    args.output.mkdir(parents=True, exist_ok=True)
    name = f"vrx-{label}-{args.target}"
    archive = args.output / (name + (".zip" if "windows" in args.target or "apple" in args.target else ".tar.gz"))
    if archive.exists():
        raise FileExistsError(f"Archive already exists: {archive}")
    with tempfile.TemporaryDirectory(prefix="vrx-package-") as temporary:
        stage = Path(temporary)
        if "apple" in args.target:
            bundle = stage / "VRX.app/Contents"
            executable = bundle / "MacOS/VRX"
            executable.parent.mkdir(parents=True)
            shutil.copy2(binary, executable)
            executable.chmod(0o755)
            shutil.copy2(ROOT / "packaging/Info.plist", bundle / "Info.plist")
            documentation = bundle / "Resources"
        else:
            package = stage / name
            package.mkdir()
            executable = package / ("vrx.exe" if "windows" in args.target else "vrx")
            shutil.copy2(binary, executable)
            if "windows" not in args.target:
                executable.chmod(0o755)
            documentation = package
            if "windows" not in args.target:
                shutil.copy2(ROOT / "packaging/vrx.desktop", package / "vrx.desktop")
        copy_documentation(documentation)
        shutil.copytree(ROOT / "assets", documentation / "assets")
        if "apple" in args.target:
            shutil.copy2(ROOT / "assets/icons/VRX.icns", documentation / "VRX.icns")
        info = {
            "version": label, "target": args.target,
            "source_commit": os.environ.get("GITHUB_SHA", "local"),
            "packaged_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "input_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "notes": "Native archive; external system libraries may be required. FFmpeg is optional and not bundled.",
        }
        (documentation / "BUILD.json").write_text(json.dumps(info, indent=2) + "\n", encoding="utf-8")
        if "apple" in args.target and sys.platform == "darwin":
            subprocess.run(["codesign", "--force", "--deep", "--sign", "-", str(stage / "VRX.app")], check=True)
        with tempfile.TemporaryDirectory(prefix=".vrx-archive-", dir=args.output) as archive_temporary:
            complete = Path(archive_temporary) / archive.name
            if archive.suffix == ".zip":
                # Cargo's vendored notice files can carry Unix-epoch mtimes.
                # ZIP starts in 1980; clamp those timestamps without editing files.
                with zipfile.ZipFile(complete, "x", zipfile.ZIP_DEFLATED, strict_timestamps=False) as output:
                    for path in sorted(stage.rglob("*")):
                        if path.is_file():
                            output.write(path, path.relative_to(stage))
            else:
                with tarfile.open(complete, "x:gz", format=tarfile.PAX_FORMAT) as output:
                    for path in sorted(stage.iterdir()):
                        output.add(path, arcname=path.name)
            complete.replace(archive)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_name(archive.name + ".sha256").write_text(f"{digest}  {archive.name}\n", encoding="utf-8")
    print(archive)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"VRX packaging: {error}", file=sys.stderr)
        sys.exit(1)
