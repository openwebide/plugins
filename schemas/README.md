# Authoring schemas

These schemas validate the current authoring format with JSON Schema 2020-12.
Plugin API 1 supports skills; API 2 adds the supported platform tool groups.
Compatibility refers to the plugin API, not the application version.

## Package manifest

`plugin.json` requires `schemaVersion`, `publisher`, `name`, `version`,
`displayName`, `description`, `license`, `compatibility` and `contributions`.
`readme` and an editor `$schema` association are optional.

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

API 2 adds `toolGroups`, a unique array of `web`, `memory`, `scheduling` and
`skill-authoring`. A manifest must contribute at least one skill or tool group;
tool-only plugins use `skills: []`. These select shared platform service handlers,
without changing permissions or executing browser code.

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
