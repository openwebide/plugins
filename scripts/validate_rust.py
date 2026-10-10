"""Check executable plugin sources against a checkout of the public host SDK."""

import argparse
import json
from pathlib import Path
import subprocess
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sdk", required=True, type=Path,
                        help="Path to crates/plugin-sdk in an Open WebIDE checkout")
    parser.add_argument("--package", action="append", type=Path,
                        help="Plugin directory to check; defaults to all executable plugins")
    args = parser.parse_args()
    sdk = args.sdk.resolve(strict=True)
    sdk_manifest = tomllib.loads((sdk / "Cargo.toml").read_text())
    sdk_version = sdk_manifest["package"]["version"]
    toolchain = tomllib.loads((sdk.parents[1] / "rust-toolchain.toml").read_text())
    channel = toolchain["toolchain"]["channel"]
    root = Path(__file__).resolve().parents[1]
    manifests = ([directory.resolve() / "plugin.json" for directory in args.package]
                 if args.package else sorted(
                     path for directory in (root / "plugins", root / "examples")
                     for path in directory.glob("**/plugin.json")))
    checked = 0
    for path in manifests:
        plugin = json.loads(path.read_text())
        executable = plugin.get("executable")
        if executable is None:
            continue
        if executable["sdkVersion"] != sdk_version:
            raise ValueError(f"{path}: SDK version does not match checkout {sdk_version}")
        manifest = (path.parent / executable["manifest"]).resolve(strict=True)
        manifest.relative_to(path.parent.resolve())
        if not (manifest.parent / "Cargo.lock").is_file():
            raise ValueError(f"{manifest}: source releases require Cargo.lock")
        patch = f"patch.crates-io.openwebide-plugin-sdk.path={json.dumps(str(sdk))}"
        cargo = ["cargo", f"+{channel}"]
        common = ["--locked", "--manifest-path", str(manifest), "--config", patch]
        print(f"Checking {plugin['displayName']} {plugin['version']}", flush=True)
        for command in (
            [*cargo, "test", *common],
            [*cargo, "clippy", *common, "--all-targets", "--", "-D", "warnings"],
            [*cargo, "clippy", *common, "--target", "wasm32-wasip2", "--", "-D", "warnings"],
            [*cargo, "build", *common, "--target", "wasm32-wasip2", "--release"],
        ):
            subprocess.run(command, check=True, cwd=root)
        checked += 1
    if checked == 0:
        raise ValueError("No executable plugins selected")
    print(f"Checked {checked} executable plugins", flush=True)


if __name__ == "__main__":
    main()
