# Open WebIDE plugins

Official Open WebIDE plugin marketplace and reference implementations for agent
tools, language support, and editor extensions.

The catalog publishes immutable first-party releases for PR Review, Web, Project
Memory, Scheduling and Skill Authoring. The Open WebIDE app installs validated
snapshots on bridge hosts and enables them across existing/new projects by
default, with saved per-project opt-outs. Plugin API 1 supports skills; API 3
supports Rust source handlers, context hooks and events through the public SDK.
Web, Memory, Scheduling and Skill Authoring own their behavior in Rust. Historical
API 2 tool-group releases require an update before activation on current hosts.
The official catalog publishes executable releases and structured JSON icons.
Further device and lifecycle checks are tracked in the app roadmap.
Language services and editor/UI contribution contracts remain on the roadmap.

## Author a plugin

Start with the [authoring guide](docs/authoring.md): copy a working example,
validate its manifest, test Rust behavior, and publish an immutable release.
See [custom marketplaces](docs/custom-marketplaces.md) for hosting your own
catalog and the [public SDK](https://github.com/openwebide/openwebide/tree/main/crates/plugin-sdk)
for the host contract.

## Repository layout

- `marketplace.json`: the first-party catalog with immutable package releases.
- Optional structured JSON icons in manifests and listings use a shared drawing schema; see [Plugin icons](docs/custom-marketplaces.md#plugin-icons).
- `plugins/`: first-party packages, each in its own directory.
- `schemas/`: package and catalog schemas plus format documentation.
- `scripts/validate.py`: offline validation for packages and catalog entries.
- `tests/`: validator regression coverage.
- `.github/workflows/validate.yml`: catalog validation, Rust tests, strict Clippy,
  and WASM source builds against the public host SDK in CI.
- `LICENSE`: MIT license for this repository. Packaged upstream dependencies
  retain their own licenses.

## Package model

A plugin is one versioned package combining the supported skills, Rust tools,
context hooks and events. MCP servers, language support, declarative panels and
editor contributions are planned extensions of that same package format.

Packages install and execute on bridge hosts: the paired native host for local
projects, or the server/container host for remote projects. Browser and phone
clients consume their metadata and service results through Open WebIDE. Plugins
do not install browser code or native software on client devices.

The database records user-wide installations and verified contribution snapshots.
Existing and future projects inherit them; Disable saves a project opt-out.
Updates preserve opt-outs and keep active runs on their pinned content. Notify
is the default update policy, with individual updates and Update All in the app.
Automatic compatible updates and Off are per-plugin preferences. Catalog checks
run while the app is open; unattended host checks remain planned. The app bundles
a pinned default subset: Web, Project Memory, Scheduling and Skill Authoring.

## Marketplace model

See [Set up a custom marketplace](docs/custom-marketplaces.md) for repository
layout, immutable release publishing, validation and connection instructions.

The official marketplace uses the same format as custom sources. A source is a
Git repository URL, ref and `marketplace.json` path; it need not be hosted on
GitHub. All listed packages live in that marketplace repository. Releases specify
only their immutable commit and package directory; they cannot override the
repository. A standalone package can publish a marketplace file in its own repo.

Catalogs can track branches, while installed releases resolve to immutable
commits. File associations and language-plugin suggestions remain planned.

## Reference packages

| Plugin | Contributions |
| --- | --- |
| [PR Review](plugins/pr-review/README.md) | Review workflow and checklist skill |
| [Web](plugins/web/README.md) | Web search and page retrieval tools |
| [Project Memory](plugins/memory/README.md) | Project memory tools and context |
| [Scheduling](plugins/scheduling/README.md) | Scheduled prompts and monitoring tools |
| [Skill Authoring](plugins/skill-authoring/README.md) | Authoring/evaluation skill and skill management tools |

Historical API 2 tool groups are retired feature switches. Current executable
plugins own their implementation and use the same public SDK and host interfaces
as community plugins. File editing, shell execution, git and task coordination remain core.
User-created skills remain independent project data. The marketplace does not
choose which plugins should be installed on fresh accounts.
