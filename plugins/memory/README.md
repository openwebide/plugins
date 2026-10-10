# Project Memory

Search and manage durable project memories.

Version 0.2.0 is an unlisted source implementation for the development Rust SDK
and plugin API 3. The marketplace still lists the earlier API 2 release until
run-planning and lifecycle integration are verified. This source depends only on
the public SDK and Serde, with no application internals or built-in tool calls.

The plugin owns argument validation, case-insensitive search, recent-first
ranking, bounded excerpts, fallback title generation, CRUD result formatting and
context selection/formatting. Its `collections` capability reads and writes the
app-visible `memories` collection through general record operations. Existing
Memory UI records retain their IDs and revisions. The host enforces account and
project scope, persistence schema, quotas, revision checks and the user's project
Memory switch. It does not implement the plugin's search or context policy.

The context hook reads stored facts as data and emits bounded JSON excerpts.
When Memory is disabled it contributes no prompt and disables its own tools.
Context hooks cannot mutate collections. The SDK's planning integration and
plugin-owned model-assisted naming remain in progress; this release is not yet
the installed default.

See [Rust source authoring](../../docs/custom-marketplaces.md#rust-source-plugins-in-development) for host prerequisites,
SDK checkout and isolated compilation checks. With the development SDK checkout:

```sh
export CARGO_TARGET_DIR=/path/to/open-webide/target
cargo test --locked --manifest-path plugins/memory/Cargo.toml \
  --config 'patch.crates-io.openwebide-plugin-sdk.path="/path/to/open-webide/crates/plugin-sdk"'
```
