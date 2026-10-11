# Rust source plugin example

Working API 3 source-compilation example for current Open WebIDE alpha hosts.
It is deliberately unlisted in the official marketplace; API 1/2 hosts cannot
execute it. The Rust crate owns the tool behavior; it contains no built-in feature proxy.

The host supplies the pinned SDK, builds with Cargo.lock in isolation, verifies
that exported tool definitions match plugin.json, and executes the resulting WASM
component on the bridge. No author-side artifact publishing is required.

## Build and check locally

The SDK has not been published to crates.io yet. Check out the same public SDK
commit used by this repository's validation workflow:

```sh
git clone https://github.com/openwebide/openwebide.git ../openwebide-sdk
git -C ../openwebide-sdk checkout c80998a02b01e8d0b6ab61ea11c7d801cbb691e6
rustup toolchain install 1.98.1 --profile minimal --component clippy --target wasm32-wasip2
CARGO_TARGET_DIR="$(cd ../openwebide-sdk && pwd)/target" \
  python3 scripts/validate_rust.py \
  --sdk ../openwebide-sdk/crates/plugin-sdk --package examples/rust-plugin
```

Run these commands from the marketplace repository root with Python 3.12 or
newer. The checker supplies a Cargo patch for the SDK dependency, tests the Rust
code, runs strict Clippy, and builds a release WASM component. Keep the exact SDK
dependency and Cargo.lock in your plugin; do not add a machine-specific SDK path
to its published Cargo.toml.

Copy this directory into `plugins/your-plugin` to start a plugin. Change its
publisher, name, description and tool declarations in plugin.json, then implement
the same tool definitions and handlers in src/lib.rs. Declare only the host
capabilities your implementation uses. The example needs none.

Publish the source commit before adding its immutable commit and directory to
marketplace.json. See [custom marketplace setup](../../docs/custom-marketplaces.md)
for the catalog format and publisher CI. Users install the source; their host
provides the SDK and compiler, validates exports and grants, and caches the WASM.

For export validation, fixture execution, contribution rules and troubleshooting,
see the [authoring guide](../../docs/authoring.md).
