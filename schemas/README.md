# Authoring schemas

These schemas validate the current repository authoring format using JSON Schema
2020-12. They do not establish compatibility with a released OpenWebIDE installer.
`compatibility.pluginApi: 1` identifies this initial draft contract; it is not an
OpenWebIDE application version or a claim that the app can activate packages.

## Package manifest

`plugin.json` requires `schemaVersion`, `publisher`, `name`, `version`,
`displayName`, `description`, `license`, `compatibility` and `contributions`.
`readme` and an editor `$schema` association are optional.

Publisher/name values use lowercase letters, digits and single hyphens, up to
64 characters. Versions use SemVer, including prerelease/build identifiers.
Publisher strings identify a namespace; they do not establish publisher trust.
Installation identity must also include the configured catalog source.

The initial contribution is `skills`, a nonempty array of objects with a `path`
to a `SKILL.md`. Each skill lives in a directory matching its frontmatter name;
supporting resources are collected from that directory. Paths are normalized,
package-relative POSIX paths; traversal, absolute paths and symlinks are rejected.
Use portable filenames supported by the schema. Existing frontmatter fields
such as license and metadata are preserved, not duplicated in the manifest.

Validation checks name/description/instruction limits and the current project
skill resource limits (16 resources, 128 KiB total, and bounded individual
resources, accounting for base64 size for binary content). Text resources use a
conservative 32 KiB byte limit. Validation never executes a skill or its scripts.

Other contributions, arbitrary extra fields, configuration values and credentials
are rejected. Add schemas for MCP, languages and UI only when their contracts are
implemented. Package dependencies and runtime provisioning are also deferred.

## Catalog

`marketplace.json` requires `schemaVersion`, a display `name` and a `plugins`
array. An empty array is valid. Listings require package identity, display name,
description, categories and at least one release.

Each release contains a SemVer `version` and `source` with a Git repository URL,
an immutable full commit ID and package directory `path` (`.` for the root).
Branches and tags are not valid installed release references. Both SHA-1 and
SHA-256 commit IDs are accepted. Supported repository URL schemes are HTTP,
HTTPS, SSH and Git; URL credentials, queries and fragments are rejected (SSH
usernames such as `git` are allowed). No forge release API is required.

Only `skills` categories are supported initially. Duplicate identities/releases
are rejected. For a listed version matching a local package, display metadata
and categories must match that package. External listings need no local checkout.

This validator is offline: it checks commit-ID syntax, not remote reachability,
commit contents, publisher ownership or runtime compatibility. Verifying pinned
source content belongs to future install/publish tooling. Archive sources and
language-discovery metadata will be added with those features.

The official catalog stays empty until actual package releases are published.
Draft packages can still be validated and their skills imported manually.
