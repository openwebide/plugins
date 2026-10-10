# Set up a custom marketplace

A marketplace is a public Git repository containing a catalog and its plugins.
It can use any public Git provider; it does not need registration with Open WebIDE.
Users add the repository URL, optional Git ref, and catalog path in Settings →
Plugins. They browse and install its releases through the Plugins interface.

## Supported today

Plugin API 1 supports skills. API 2 also supports a fixed set of platform tool
groups. Those tool groups select built-in behavior and are transitional; they do
not let authors supply new executable handlers. WASM artifacts, custom handlers,
MCP contributions and the Rust SDK are not implemented yet.

Start with a skills-only plugin. Copy this repository's schemas, validator,
requirements and validation workflow into your repository, keeping their relative
paths. Use `plugins/pr-review` as a working package example, changing its publisher,
name, display metadata, skill instructions and version to describe your plugin.
Keep your license and attribution accurate. The package layout is:

```text
marketplace.json
plugins/
  my-plugin/
    plugin.json
    README.md
    skills/
      my-skill/
        SKILL.md
schemas/
scripts/validate.py
requirements-dev.txt
.github/workflows/validate.yml
```

For an API 1 manifest, set `compatibility.pluginApi` to `1` and declare your
package-relative skill path in `contributions.skills`. The skill directory must
match its frontmatter `name`. See [the authoring format](../schemas/README.md)
for required fields and limits. A publisher name is a namespace, not verified
identity or a trust badge.

## Publish an immutable release

1. Commit the complete plugin package with its intended SemVer version.
2. Push that source commit to the public repository. Keep published commits
   reachable, for example with a release tag.
3. Record the full commit ID with `git rev-parse HEAD`.
4. Add the release to `marketplace.json` in a subsequent commit, using that source
   commit. The catalog cannot point to the commit that contains its own new hash.

For example, this catalog lists one plugin. Replace the example commit with the
actual full source commit ID before publishing; copy listing metadata from the
package manifest exactly.

```json
{
  "schemaVersion": 1,
  "name": "My marketplace",
  "plugins": [
    {
      "publisher": "example",
      "name": "my-plugin",
      "displayName": "My Plugin",
      "description": "A reusable project workflow.",
      "categories": ["skills"],
      "releases": [
        {
          "version": "0.1.0",
          "source": {
            "commit": "0123456789abcdef0123456789abcdef01234567",
            "path": "plugins/my-plugin"
          }
        }
      ]
    }
  ]
}
```

All listed plugins belong to the configured marketplace repository. Releases
cannot override its repository URL. Paths are repository-relative; use `.` for a
package at the root. The catalog can track a branch, but installed releases use
immutable full commit IDs. Preserve earlier entries to support version selection
and rollback; publish new versions rather than changing an existing release pin.

## Validate and connect

From your repository root, use Python 3.12 and run:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-dev.txt
.venv/bin/python scripts/validate.py
```

The copied workflow runs validation on pushes and pull requests. If copying its
unit-test step, also copy this repository's `tests/` directory; otherwise remove
that step. Offline validation checks format and package resources, not whether a
remote commit exists or contains the claimed version. Open WebIDE verifies the
actual pinned package during installation.

Push the catalog commit, then add a source in Settings → Plugins:

- Repository: your public clone URL, such as `https://git.example.org/team/plugins.git`.
- Ref: your default branch, or an explicit branch/tag/commit.
- Catalog path: `marketplace.json`, or your chosen repository-relative path.

Refresh marketplaces in the Plugins menu. Inspect the listing, install it, and
check its skills in both local and remote projects. Local projects require a
paired native bridge; remote projects use the server bridge. Installation never
executes plugin code on a phone or in the browser. Installs are enabled across
projects by default, with per-project opt-outs. Update notifications default to
Notify; publishing a release does not replace that pinned installation silently.

## Rust source plugins (in development)

The intended executable contract lets authors implement behavior in Rust using a
public, versioned SDK and host interfaces. A marketplace release pins the source
commit and directory containing plugin.json, Cargo.toml, Cargo.lock and Rust
source. The execution host compiles that source into a WASM component on install,
using the declared SDK and a pinned toolchain. Compilation is isolated from host
files and network access after dependency retrieval. The bridge validates exported
tools and runs the component using granted general host capabilities.

Publish source rather than adding artifact URLs to marketplace.json. Packages
continue to belong to the marketplace repository; the catalog uses the same
immutable commit/directory release shape as skills. API 3's executable block
declares Cargo.toml, the library name, SDK version and capabilities; contributions
declares the exported tools. See examples/rust-plugin for an unlisted authoring
example. Publisher CI should compile and validate source builds before publishing.
The host caches compiled output by source, SDK and toolchain; a failed build must
preserve the working installation. Full installation/runtime support is still in
development and the example is not an official marketplace release.

The proposed SDK home is the Open WebIDE app's Rust workspace, published as a
versioned crate with versioned component interface definitions. Plugins consume
released interfaces without accessing app internals. A separate SDK repository
is not required to use it from a custom marketplace.

First-party executable plugins must use exactly the same public extension
contract as community plugins and own their implementation. General host
capabilities are allowed; plugin-specific built-in feature proxies, privileged
interfaces and fallback implementations are not.
