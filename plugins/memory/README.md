# Project Memory

Search and manage durable project memories.

Version 0.2.1 is an executable API 3 release in the official catalog and the
compiled default bundle. It depends only on the public SDK and Serde, with no
application internals or built-in tool calls.

The plugin owns argument validation, case-insensitive search, recent-first
ranking, bounded excerpts, fallback title generation, CRUD result formatting and
context selection/formatting. Its `collections` capability reads and writes the
app-visible `memories` collection through general record operations. Existing
Memory UI records retain their IDs and revisions. The host enforces account and
project scope, persistence schema, quotas, revision checks and the user's project
Memory switch. It does not implement the plugin's search or context policy.

The context hook reads stored facts as data and emits bounded JSON excerpts.
When Memory is disabled it contributes no prompt and disables its own tools.
Context hooks cannot mutate collections or request model completions. The host runs them through
shared planning before selecting model tools in both local and remote sessions,
including when model tools are disabled. The separate `completion` capability
lets this source own its automatic-title prompt and result validation, trying the
configured fast model, then primary, then a content-derived fallback. Manual
titles skip model requests, and project opt-out prevents naming requests.
Production checks verify fresh defaults, CRUD/context, configured automatic
naming, project opt-out and uninstall persistence through both hosts. Browser
contracts cover editing, SDK chat dispatch, context, conflicts and disabling.
The implementation is integrated into the app and marketplace main branches.
Remaining manual PWA checks are tracked in the app roadmap.

See [Rust source authoring](../../docs/custom-marketplaces.md#rust-source-plugins-in-development) for host prerequisites,
SDK checkout and isolated compilation checks. With the development SDK checkout:

```sh
export CARGO_TARGET_DIR=/path/to/open-webide/target
cargo test --locked --manifest-path plugins/memory/Cargo.toml \
  --config 'patch.crates-io.openwebide-plugin-sdk.path="/path/to/open-webide/crates/plugin-sdk"'
```
