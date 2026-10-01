"""Build and stage the unprivileged agent for Tauri sidecar packaging.

This build tool never runs the agent, installs a service, or opens capture.
"""

import argparse
import json
import pathlib
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[4]
DESKTOP = ROOT / "crates/apps/rokbattles-desktop"
TARGETS = {
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
}


def host_target():
    output = subprocess.check_output(["rustc", "-vV"], text=True)
    return next(line.removeprefix("host: ") for line in output.splitlines() if line.startswith("host: "))


def stage(source, target):
    if target not in TARGETS:
        raise ValueError("unsupported agent target")
    suffix = ".exe" if "windows" in target else ""
    destination = DESKTOP / "binaries" / f"rokbattles-desktop-agent-{target}{suffix}"
    destination.parent.mkdir(exist_ok=True)
    if destination.parent.is_symlink() or destination.is_symlink():
        raise ValueError("sidecar staging must not use links")
    if not source.is_file() or source.is_symlink():
        raise ValueError("missing built agent")
    shutil.copy2(source, destination)
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=sorted(TARGETS))
    parser.add_argument("--debug", action="store_true")
    args = parser.parse_args()
    target = args.target or host_target()
    if target not in TARGETS:
        parser.error("unsupported native target")
    command = ["cargo", "build", "--locked", "-p", "rokbattles-desktop-agent", "--target", target]
    if not args.debug:
        command.append("--release")
    subprocess.run(command, cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT, text=True))
    suffix = ".exe" if "windows" in target else ""
    source = pathlib.Path(metadata["target_directory"]) / target / ("debug" if args.debug else "release") / f"rokbattles-desktop-agent{suffix}"
    print(stage(source, target))


if __name__ == "__main__":
    main()
