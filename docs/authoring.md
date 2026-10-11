# Author a plugin

Current alpha hosts support API 1 skills and API 3 Rust tools, read-only context
hooks and durable events. Start with a skill for instructions/resources, or Rust
for executable behavior. A plugin can combine them. MCP servers, language
services, editor hooks and declarative panels are not authorable yet.

## Start from a working example

Clone this repository and run commands from its root. For Rust, copy
`examples/rust-plugin` into `plugins/my-plugin`; for skills only, copy
`plugins/pr-review`. Keep the files inside that directory, including the license
and any skill resources. Do not copy host implementation crates.

For Rust, edit `plugin.json`, `Cargo.toml` and `src/lib.rs` together:

- Choose your publisher/name identity and SemVer version, display metadata and
  license. A publisher namespace is not a verified identity.
- Set `compatibility.pluginApi` to `3`. Keep `schemaVersion` at `1`.
- Keep `executable.manifest` as `Cargo.toml`; match `executable.library` to the
  Cargo `[lib].name` and keep `crate-type = ["cdylib"]`.
- Keep SDK `=0.1.0` in Cargo and `sdkVersion: "0.1.0"` in the manifest. The SDK
  is independently versioned from the app and is not published on crates.io.
  The checker and installation host supply it; plain `cargo build` without the
  SDK patch is not the current authoring workflow.
- Export your implementation with `openwebide_plugin_sdk::export!(YourPlugin)`.
  Tool definitions must match `contributions.tools` exactly, including order,
  descriptions, schemas and approval flags. Event names must likewise match.
- Declare only needed capabilities. Tools with side effects should request
  approval; declaring `false` cannot weaken the host's permission policy.
- Commit `Cargo.lock` with source. If dependencies change, regenerate it using
  the same SDK patch as the checker, then rerun locked validation.

The Echo example returns `Plugin says: ...` from its own Rust handler and needs
no capabilities. Add meaningful native unit tests for your parsing and policy;
fixture execution below exercises the compiled component boundary.

For skills, use API `1`, declare `contributions.skills` paths and match each
`SKILL.md` frontmatter name to its enclosing directory. No SDK/compiler is needed.
See [manifest fields and limits](../schemas/README.md).

## Validate and build

Use Python 3.12 or newer. These commands check manifests, resources and the catalog:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-dev.txt
.venv/bin/python scripts/validate.py --package plugins/my-plugin
.venv/bin/python scripts/validate.py
.venv/bin/python -m unittest discover -s tests -v
```

For Rust, follow the [SDK checkout and build commands](../examples/rust-plugin/README.md#build-and-check-locally),
replacing `--package examples/rust-plugin` with `--package plugins/my-plugin`.
They run locked native tests, strict native/WASI Clippy and a release
`wasm32-wasip2` component build. Copy the pinned SDK checkout from CI rather than
using an arbitrary app branch. Reuse one `CARGO_TARGET_DIR` for app and plugin
builds instead of creating another large build directory. After editing Cargo
dependencies, regenerate the lockfile from the marketplace root (adjust paths):

```sh
CARGO_TARGET_DIR="$(cd ../openwebide-sdk && pwd)/target" \
  cargo +1.98.1 generate-lockfile --manifest-path plugins/my-plugin/Cargo.toml \
  --config "patch.crates-io.openwebide-plugin-sdk.path=\"$(cd ../openwebide-sdk/crates/plugin-sdk && pwd)\""
```

Review and commit the changed lockfile; subsequent checker builds use `--locked`.

Compilation alone does not verify manifest/export agreement. From that SDK app
checkout, validate and execute the original Echo component with:

```sh
printf '{}\n' > /tmp/openwebide-plugin-fixtures.json
CARGO_TARGET_DIR="$PWD/target" cargo run --locked \
  -p openwebide-plugin-runtime --example check_source -- \
  ../plugins/examples/rust-plugin echo_message \
  '{"message":"Hello"}' /tmp/openwebide-plugin-fixtures.json
```

Replace `../plugins/examples/rust-plugin` with the actual path to your package.
With no trailing tool/arguments/fixture file, the checker only compiles and
validates exported tools/events. For context use `--context '{"budget_bytes":512}'`;
for an event use `--event '{"name":"job_due","payload":{}}'` in place of the
tool/arguments pair, retaining the fixture file. The event must be declared.
Fixture files map capability names to canned JSON responses; they never access
real host services and cannot simulate sequences of state changes. Test those
flows on a real paired and server host.

The isolated checker needs the same compiler sandbox as installation: macOS's
native sandbox, bubblewrap for non-root Linux, or the packaged build helper and
Landlock ABI 3 in a root Linux container. Do not bypass isolation to make a
plugin build. The [SDK capability reference](custom-marketplaces.md#rust-source-plugins)
describes host calls, scope, limits and fixture response shapes.

## Exercise the installation lifecycle

Push the complete source to a public Git repository and obtain its full commit
ID. In Plugins → menu → **Install a pinned plugin manually**, enter that
repository, commit and package directory. A custom catalog is optional for this
test. The browser does not compile or execute code: remote projects use the
server bridge and local projects use their paired native bridge.

Check tools/skills, denied capabilities and malformed inputs in both modes.
Check install cancellation/failure, project disablement, uninstall/reinstall and
updates with active runs. Installs enable plugins by default; project opt-outs
persist. Updates default to Notify and additional capabilities require review.
Active executions retain their source snapshot. Durable events are delivered at
least once: use stable idempotency keys and revision checks, and test duplicate
delivery and cancellation. Plugin code receives no ambient filesystem,
environment or networking access; Rust does not grant shell/process access.

## Contribute or publish

For first-party contributions, open a pull request with the plugin source,
manifest, lockfile, README, license and tests. Describe behavior, capabilities and
how you verified both execution hosts. First-party plugins must implement their
behavior through the same SDK as community plugins; built-in feature proxies or
special identity privileges are not accepted. Core workspace tools remain built in.

Merge/push source before adding its full immutable commit to a catalog in a
subsequent change. Keep that commit reachable and previous releases available;
publish a new version for changes. All plugins in a marketplace inherit its
repository. See [custom marketplace setup](custom-marketplaces.md) for catalog
JSON, publisher CI and connecting the source. Add optional
[structured icons](custom-marketplaces.md#plugin-icons) to both manifest and listing.

Common failures: export mismatches require changing Rust and manifest together;
SDK mismatches require a compatible host/SDK, not access to app internals; lockfile
errors require regeneration before publishing; unsupported contributions require
waiting for their contracts. A failed preparation leaves the installed version
intact. API 2 tool-group switches are historical, not an executable alternative.
