"""Check executable plugin sources against a checkout of the public host SDK."""

import argparse
from contextlib import nullcontext
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sdk", required=True, type=Path,
                        help="Path to crates/plugin-sdk in an Open WebIDE checkout")
    parser.add_argument("--package", action="append", type=Path,
                        help="Plugin directory to check; defaults to all executable plugins")
    parser.add_argument("--language-toolkit", type=Path,
                        help="Open WebIDE checkout containing a public parser toolkit under development")
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
        configuration = ["--config", patch]
        if args.language_toolkit:
            checkout = args.language_toolkit.resolve(strict=True)
            for name in ["core", "language-grammars", "language-syntax"]:
                directory = checkout / "crates" / name
                if not (directory / "Cargo.toml").is_file():
                    raise ValueError(f"Missing public toolkit crate: {directory}")
                configuration += ["--config", f'patch."https://github.com/openwebide/openwebide.git".openwebide-{name}.path={json.dumps(str(directory))}']
        print(f"Checking {plugin['displayName']} {plugin['version']}", flush=True)
        scratch = (tempfile.TemporaryDirectory(prefix="openwebide-public-toolkit-")
                   if args.language_toolkit else nullcontext(None))
        with scratch as directory:
            if directory is not None:
                # Path patches need a different lock resolution from the pinned
                # release. Keep that development-only lock out of package sources.
                package = Path(directory) / "package"
                shutil.copytree(path.parent, package, symlinks=True,
                                ignore=shutil.ignore_patterns("target", ".git"))
                manifest = package / manifest.relative_to(path.parent)
                subprocess.run([*cargo, "metadata", "--offline",
                                "--format-version", "1", "--manifest-path", str(manifest),
                                *configuration], check=True, cwd=root, stdout=subprocess.DEVNULL)
            common = ["--locked", "--manifest-path", str(manifest), *configuration]
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
