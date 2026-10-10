# Set up a custom marketplace

A marketplace is a public Git repository containing a catalog and its plugins.
It can use any public Git provider; it does not need registration with Open WebIDE.
Users add the repository URL, optional Git ref, and catalog path in Settings →
Plugins. They browse and install its releases through the Plugins interface.

## Supported today

Plugin API 1 supports skills. API 2 also supports a fixed set of platform tool
groups. Those tool groups select built-in behavior and are transitional; they do
not let authors supply new executable handlers. WASM artifacts, custom handlers,
MCP contributions remain future work. The Rust SDK and source compiler are in
development; executable first-party releases are not listed yet.

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

The prototype SDK is in `crates/plugin-sdk` in the app repository, with versioned
WIT interfaces and an isolated authoring checker. Until a registry release is
published, follow [the Rust example's checkout instructions](../examples/rust-plugin/README.md)
to use it. Host compilation supplies its embedded SDK; installed plugins do not
reach into the app's implementation crates.

The public `http`, `clock`, `completion` and private `records` primitives are available in the
prototype. Private records are scoped by authenticated account, project and
plugin source, with bounded pages and revision checks. The host attaches authority
outside plugin code; plugins cannot supply account or project IDs. Source updates
can retain those records, but a different marketplace source does not inherit
another plugin's namespace. The separate `collections` grant enables general CRUD
on documented app-visible project collections. The prototype's `memories`
adapter preserves existing UI data; it enforces ownership, the user's opt-out,
schema bounds and revision checks. Plugin code owns search, ranking, naming and
formatting. See the unlisted [Memory source reference](../plugins/memory/README.md).
The `skills` collection preserves existing UI skill IDs, revisions, resources and
metadata. Writes accept `{"draft": {...}}`, using the documented `SkillDraft`
schema; reads return `{"draft": {...}, "origin": ...}`. `origin` is either null
for personal skills or the owning plugin's publisher, name, version and commit.
It is read-only. Managed skills cannot be edited or deleted through collections;
manage their owning plugin instead. Disabled skills are omitted from reads and
cannot be mutated through this interface. List pages contain at most eight full
skills and approximately 1 MiB of serialized records, with the usual `next` cursor.
Skills mutations admit the existing 128 KiB data schema with up to 1 MiB of JSON
payload for escaping; private records retain their 64 KiB value limit.
Schema validation and revision conflicts are persistence
constraints; authoring, search and prompt policy belong in plugin source.
See the unlisted [Skill Authoring source reference](../plugins/skill-authoring/README.md)
for tool handlers, draft validation, preserved update fields, resource paging and
bounded context catalogs implemented entirely through the public SDK.

The HTTP capability supports ordinary public and LAN services. The host adds
a transport marker that plugin headers cannot remove; Open WebIDE rejects these
requests to its authenticated backend and bridge control APIs, including credential
bootstrap and WebSocket upgrades. Plugins must use declared SDK capabilities for
host operations. HTTP does not supply TCP tunnels or allow its marker to be
converted into a hop-by-hop header.

The prototype `jobs` capability supplies a durable one-shot event queue. Declare
`jobs` in the executable capabilities and the event name in `contributions.events`
and `Plugin::events()`. Plugin source computes due times, recurrence and event
behavior. The host supplies storage, immutable program snapshots and leases.

```rust
let queued: serde_json::Value = openwebide_plugin_sdk::jobs(&serde_json::json!({
    "action": "schedule",
    "key": "task-42-occurrence-1",
    "due_at": 1900000000,
    "expires_at": null,
    "event": "task_due",
    "payload": { "task_id": 42 }
}))?;
```

Times are Unix seconds. New jobs require a current or future due time, and an
optional expiry strictly after it. Keys contain 1–128 letters, numbers or `-_.:`.
An identical key/time/event/payload/source request returns the original job,
including after its due time; changing those values under the same key is a
conflict. Account, project, plugin identity and execution host are attached by
the host and cannot be supplied in this request. Jobs keep the creating source
version and model context across plugin updates. Chained jobs retain the
originating conversation; background callback grants receive neither chat
authority nor a manual-editing override.

Operations `list` (optional `after` cursor), `read` (`id`), `cancel` (`id`,
`revision`) and `delete` (`id`, `revision`) return `{jobs, next_after}`. States are
`pending`, `leased`, `completed`, `failed`, `cancelled` and `expired`. Cancel
accepts pending/leased jobs. Delete accepts only terminal jobs and also removes
their retained idempotency keys. Each namespace/project holds at most 1,000 jobs;
plugins own retention policy. Payloads are limited to 64 KiB of serialized JSON;
list pages hold at most 64 jobs and 1 MiB. The host leases bounded delivery
batches, renews active leases and rejects callbacks from cancelled or stale
attempts. Disabled or removed plugins receive no new delivery claims.

The bridge prototype polls for due events and delivers them through the shared
plugin executor, renewing leases during compilation and execution. Losing a lease
cancels the active actor. Delivery is at least once, so plugin code must use stable
keys and revision checks to make effects repeatable. Run submission/completion,
workspace primitives and full deployment verification remain in development. These foundations do not make the transitional
Scheduling catalog release an executable plugin.

The prototype `runs` grant supports general durable prompt submissions:

```rust
let submitted: serde_json::Value = openwebide_plugin_sdk::runs(&serde_json::json!({
    "action": "submit",
    "key": "check:1",
    "prompt": "Inspect the current state and report the result.",
    "target": {"kind": "origin"}
}))?;
```

Targets are `origin`, `session` with an owned conversation `id`, or `new` with a
source-supplied `title`. The host enforces the grant's account/project scope and
retains the origin of a background event without granting chat authority. An
optional `model` is `{server_id, model}`; otherwise the originating execution's
pinned primary is used where available. It does not change conversation settings.
Repeat an identical submission key to receive its original run; a changed request
with that key is a conflict. Operations `list` (optional `after`), `read` (`id`),
`cancel` (`id`, `revision`) and terminal `delete` (`id`, `revision`) return
`{runs, next_after}`. Prompts are limited to 32 KiB, history to 1,000 entries, and
pages to 16 entries. Delete releases retained keys and quota. Plugin prompts use
the ordinary durable conversation queue, with host-only delivery and atomic
message insertion/consumption; users can remove them to cancel pending runs.
The app prototype supplies internal host claims, renewable leases, status reports
and busy-claim release. Reclaimed prompts receive fresh revisions and runner
identities. Recovery requeues only unconsumed prompts; a stopped host after
consumption produces `interrupted` without replay. Local workspace execution
requires a user-authorized host folder binding. These are daemon primitives;
plugin code cannot supply lease authority or choose another execution host.
The bridge prototype dispatches raw submissions through the shared agent runner
on server and paired hosts. Busy conversations release unconsumed claims; status
reporting retries transient failures while leases stay alive. Lease loss or
shutdown cancels owned preparation/execution. Reports retain bounded raw output,
without interpreting outcomes. Submit an optional completion event with your
prompt:

```rust
let submitted: serde_json::Value = openwebide_plugin_sdk::runs(&serde_json::json!({
    "action": "submit",
    "key": "build-check:42",
    "prompt": "Check the build and report the result.",
    "target": {"kind": "origin"},
    "completion": {"event": "build_finished", "payload": {"check": 42}}
}))?;
```

Declare `build_finished` in `contributions.events` and `Plugin::events()`. Its
`EventInput.payload` will contain `{run, data}`: `run` is the terminal run record
with bounded raw output in `detail`, and `data` is your submitted payload (at most
64 KiB). Submission reserves queue capacity; a full queue rolls back the prompt.
The reserved event is `waiting` until a terminal status commits, including queued
cancellation, preflight failure or interrupted-host recovery. It becomes `pending`
in that same transaction. The event retains the submitting plugin version across
updates and survives subsequent run-history deletion. Existing leased event
delivery and disablement rules apply; use stable keys/revision checks because
callbacks can execute again after a lost acknowledgement. A cancelled event stays
cancelled when its run finishes. Deleted origin conversations do not grant new
background authority. Executable Scheduling remains unfinished; plugins supply
timing, prompts, recurrence and result interpretation through these primitives.

The `completion` capability accepts a `CompletionRequest` with `system_prompt`,
`prompt`, `profile` (`primary` or `fast`) and `max_output_tokens`. The SDK's
`complete` function returns a `CompletionResponse` containing `text`. The host
uses the session's configured model and supplies its credentials; a plugin cannot
select a server ID, URL or account. Inputs are bounded to 32 KiB, requested outputs
to 1–1024 tokens and returned text to 16 KiB. The host enforces model context and
output limits, disables tools and uses a 30-second completion deadline. Missing
fast configuration is an error. Plugins choose prompts, result validation and
fallback behavior; the Memory reference tries fast, then primary, then its own
content-derived title. Context hooks cannot call completion.

Event handlers declare their names in `contributions.events` and return the same
names from `Plugin::events()`. Implement `Plugin::event(EventInput)` to own the
callback behavior; the input contains a name and JSON payload, with a 256 KiB
payload limit. The host checks manifest/export agreement and invokes handlers
with the same capability rules and isolated WASM runtime as tools. Event-only
plugins can use `skills: []` and omit `tools`. The source checker accepts `--event`
with `{"name":"job_due","payload":{}}` to exercise a declared handler. Durable
job registration and delivery are still under development; declaring an event
does not subscribe it to a working scheduler yet.

The public `Plugin::context(ContextInput)` hook returns a `ContextContribution`:
optional prompt text and a list of the plugin's own tools to disable. The default
implementation contributes nothing. Context requests carry a byte budget (at
most 8 KiB). The runtime permits only clock calls and list/read operations on
private records, shared collections or durable jobs; mutations are denied before reaching the
host. Plugins cannot disable another plugin's tools. This hook is supported in
shared local and remote run planning before model tool selection, including when
model tools are disabled. Full deployment verification remains in development.
The source checker accepts `--context` in place of a tool name to exercise the
hook with a JSON input such as `{"budget_bytes":512}` and capability fixtures.

The prototype compiler uses the native macOS sandbox or bubblewrap for native
Linux users. Root containers use the app's packaged compiler launcher, which
drops privileges and requires fully enforced Landlock ABI 3. The default Docker
security profile is sufficient. Dependencies are retrieved in an isolated fetch
phase, then package build scripts and macros compile without network access.
Sources and the SDK/toolchain remain read-only. These host requirements do not
change the marketplace manifest or add an author-side artifact publishing step.

An update that adds runtime capabilities requires explicit review, even under
Automatic updates and even for a compatible version. Compilation alone does not
activate that update. The previous installed version remains active until the
user approves the prepared version's additions.

First-party executable plugins must use exactly the same public extension
contract as community plugins and own their implementation. General host
capabilities are allowed; plugin-specific built-in feature proxies, privileged
interfaces and fallback implementations are not.
