# Skill Authoring

Create, improve and manage reusable project skills.

Version 0.2.0 is an unlisted Rust source implementation for the development SDK
and plugin API 3. The marketplace retains the historical API 2 release until the
full installation/run lifecycle and executable defaults are verified.

This plugin owns its six tool handlers: search and list pagination, instruction
and resource paging, draft validation, preserving omitted fields when updating,
revision checks, result formatting, context catalog selection and its embedded
authoring/evaluation workflow. It depends only on the public SDK, Serde and Base64.
It does not call built-in skill tools or application implementation crates.

The `collections` capability accesses the shared `skills` data schema. The host
enforces account/project ownership, persistence constraints, revisions and the
user's opt-outs. It preserves existing UI records, resources and metadata and
exposes read-only plugin provenance. Managed skills are updated through their
owning plugin. Resources remain data; reading a resource never executes it.

The context hook contributes a bounded skill catalog and disables this plugin's
tools when project skills are turned off. The creator tool returns the workflow
and optional existing instructions; it does not claim to have evaluated or saved
a skill. The user and agent continue that workflow with the approved CRUD tools.

See [Rust source authoring](../../docs/custom-marketplaces.md#rust-source-plugins-in-development).
With the development SDK checkout:

```sh
export CARGO_TARGET_DIR=/path/to/open-webide/target
cargo test --locked --manifest-path plugins/skill-authoring/Cargo.toml \
  --config 'patch.crates-io.openwebide-plugin-sdk.path="/path/to/open-webide/crates/plugin-sdk"'
```
