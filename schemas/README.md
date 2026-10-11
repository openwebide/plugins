# Authoring schemas

These schemas validate the current authoring format with JSON Schema 2020-12.
Plugin API 1 supports skills; API 3 adds executable Rust tools, context hooks
and events. API 2 tool groups are historical and cannot activate on current hosts.
Compatibility refers to the plugin API, not the application version.

## Package manifest

`plugin.json` requires `schemaVersion`, `publisher`, `name`, `version`,
`displayName`, `description`, `license`, `compatibility` and `contributions`.
`readme`, a structured `icon` and an editor `$schema` association are optional.
See the [authoring guide](../docs/authoring.md) for examples.

Publisher/name values use lowercase letters, digits and single hyphens, up to
64 characters. Versions use SemVer, including prerelease/build identifiers.
Publisher strings identify a namespace; they do not establish publisher trust.
Installation identity must also include the configured catalog source.

The `skills` contribution is an array of objects with a `path`
to a `SKILL.md`. Each skill lives in a directory matching its frontmatter name;
supporting resources are collected from that directory. Paths are normalized,
package-relative POSIX paths; traversal, absolute paths and symlinks are rejected.
Use portable filenames supported by the schema. Existing frontmatter fields
such as license and metadata are preserved, not duplicated in the manifest.

Validation checks name/description/instruction limits and the current project
skill resource limits (16 resources, 128 KiB total, and bounded individual
resources, accounting for base64 size for binary content). Text resources use a
conservative 32 KiB byte limit. Validation never executes a skill or its scripts.

API 3 requires `executable`: `manifest: "Cargo.toml"`, a Rust `library` name,
`sdkVersion: "0.1.0"`, and unique `capabilities`. The manifest declares
`contributions.tools` with name, description, object `parameters` schema and
`requires_approval`; these must exactly match `Plugin::tools()`, including order.
`contributions.events` must likewise match `Plugin::events()`. Always include
`skills`, using `[]` when absent. A plugin must contribute skills, tools or events.
Context hooks are implemented through `Plugin::context`, not a manifest field.
Optional `contributions.background` names a declared event and an
`intervalSeconds` from 60 to 86400; it requires the `jobs` capability.

Supported grants are `http`, `clock`, `completion`, `records`, `collections`,
`jobs` and `runs`. `workspace` is reserved and has no implemented host adapter.
See the [capability reference](../docs/custom-marketplaces.md#rust-source-plugins).
API 2 `toolGroups` remains in the schema only for historical metadata; do not use
it to author new plugins. API 3 cannot activate tool groups.

Other contributions, arbitrary extra fields, configuration values and credentials
are rejected. Add schemas for MCP, languages and UI only when their contracts are
implemented. Package dependencies and runtime provisioning are also deferred.

## Catalog

`marketplace.json` requires `schemaVersion`, a display `name` and a `plugins`
array. An empty array is valid. Listings require package identity, display name,
description, categories and at least one release.

Each release contains a SemVer `version` and `source` with an immutable full
commit ID and package directory `path` (`.` for the root). The repository is
inherited from the configured marketplace source; listings and releases cannot
provide a repository override. All packages belong to the marketplace's Git
repository. Branches and tags are not valid installed release references. Both
SHA-1 and SHA-256 commit IDs are accepted. Marketplace repository URLs and
credentials are app configuration, not catalog fields. No forge release API is
required.

The `skills` and `tools` categories describe implemented contributions. Duplicate identities/releases
are rejected. For a listed version matching a local package, display metadata
and categories must match that package. Historical packages may be absent from the current checkout.

This validator is offline: it checks commit-ID syntax, not remote reachability,
commit contents, publisher ownership or runtime compatibility. The app verifies pinned
source contents during host preparation. Archive sources and
language-discovery metadata will be added with those features.

The official catalog lists immutable first-party releases. Publishing manifests
and their source commit happens before adding that commit to the catalog.
