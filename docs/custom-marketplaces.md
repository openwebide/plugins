# Set up a custom marketplace

A marketplace is a public Git repository containing a catalog and its plugins.
It can use any public Git provider; it does not need registration with Open WebIDE.
Users add the repository URL, optional Git ref, and catalog path in Settings →
Plugins. They browse and install its releases through the Plugins interface.

## Supported today

Plugin API 1 supports skills. API 2 also supports a fixed set of platform tool
groups. Those tool groups select built-in behavior and are transitional; they do
not let authors supply new executable handlers. The API 3 prototype accepts Rust
source with public SDK tool handlers, context hooks and durable events. Hosts
compile source on installation; catalog artifact URLs are not used. MCP, language
and UI contributions remain future work. Full production deployment is still in
progress; executable first-party releases are not listed yet.

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

The executable prototype lets authors implement behavior in Rust using a
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

The SDK lives in the Open WebIDE app's Rust workspace with versioned component
interface definitions; registry publication remains pending. Plugins consume
released interfaces without accessing app internals. A separate SDK repository
is not required to use it from a custom marketplace.

The prototype SDK is in `crates/plugin-sdk` in the app repository, with versioned
WIT interfaces and an isolated authoring checker. Until a registry release is
published, follow [the Rust example's checkout instructions](../examples/rust-plugin/README.md)
to use it. Host compilation supplies its embedded SDK; installed plugins do not
reach into the app's implementation crates.

Official defaults use that same source format and capability contract. App
**distribution builds**, rather than marketplace publishers, compile the selected
commit-pinned subset through the normal host compiler sandbox and embed validated
WASM components. First installation can then work offline. A custom marketplace
still ships Rust source and Cargo.lock; its users' hosts compile it on installation.
Precompilation grants no extra capabilities, app imports or built-in fallbacks.

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
An identical key/time/event/payload/source/scope request returns the original job,
including after its due time; changing those values under the same key is a
conflict. Account, project, plugin identity and execution host are attached by
the host and cannot be supplied in this request. Jobs keep the creating source
version and model context across plugin updates. `schedule` defaults to
`"scope": "origin"`, retaining its creating conversation for follow-up work.
Choose `"scope": "project"` for saved work that must survive deleting that chat;
it keeps the project/model context while dropping the conversation anchor. In
projectless contexts this is the account's global conversation realm. Chained
origin jobs retain their parent's origin; project jobs do not acquire one. A
scope change under an existing key is a conflict. Background callback grants
receive neither chat authority nor a manual-editing override.

Operations `list` (optional `after` cursor), `read` (`id`), `cancel` (`id`,
`revision`) and `delete` (`id`, `revision`) return `{jobs, next_after}`. States are
`waiting`, `pending`, `leased`, `completed`, `failed`, `cancelled` and `expired`.
`waiting` is reserved for run completion events. Cancel accepts waiting, pending
and leased jobs. Delete accepts only terminal jobs and also removes
their retained idempotency keys. Each namespace/project holds at most 1,000 jobs;
plugins own retention policy. Payloads are limited to 64 KiB of serialized JSON;
list pages hold at most 64 jobs and 1 MiB. The host leases bounded delivery
batches, renews active leases and rejects callbacks from cancelled or stale
attempts. Disabled or removed plugins receive no new delivery claims.

Plugins may declare one periodic background event for reconciliation:

```json
"background": {"event": "reconcile", "intervalSeconds": 60}
```

Place this under `contributions`, also list `reconcile` in `events`, and declare
the `jobs` capability. The interval must be 60–86,400 seconds. The host queues an
initial tick for enabled installations and new project bindings, then repeats
using persisted deadlines. Ticks use the same compiled component, leased worker
and capability grants as other events, with null payload, project scope and no
conversation, manual-edit override or inherited model pin. Source can read current
configuration and choose explicit models for raw runs.

Ticks use the current enabled binding. Updating or disabling a binding revokes
its previous background delivery; explicitly scheduled jobs keep their immutable
source snapshots. Only one tick per subscription can be pending or leased, and
host bookkeeping is cleaned between ticks. Recovery ticks have bounded capacity
separate from the 1,000 user-job quota, so source can clean a full queue. The
`background:` key prefix is reserved for host delivery. Delivery remains at least
once; reconcile using idempotency keys and revisions. The host does not interpret
plugin state, adopt tasks, calculate recurrence or invoke built-in features.

One-shot job deadlines may already be overdue when submitted; the host delivers
them when available. User-facing future-date rules and catch-up behavior belong
to plugin source.

The bridge prototype polls for due events and delivers them through the shared
plugin executor, renewing leases during compilation and execution. Losing a lease
cancels the active actor. Delivery is at least once, so plugin code must use stable
keys and revision checks to make effects repeatable. The Scheduling policy
migration, UI mutation integration and full deployment verification
remain in development. These foundations do not make the transitional Scheduling
catalog release an executable plugin.

The `tasks` collection offers scoped CRUD on existing task and monitor records,
preserving IDs and history. Values include `draft`, `next_run`, plugin-owned JSON
`state`, and optional `monitor` metadata (`session_id`, `interval_seconds`,
`remaining`, `expires_at`). Reads additionally expose read-only `owner` and
`editable` fields. Update/delete require a current revision; another plugin's
task and goal-worker records cannot be mutated. Unowned legacy tasks can be
adopted only after their prior deliveries finish, which stops legacy dispatch.
The host enforces schema and ownership, while source owns scheduling and monitor
policy. Lists hold at most 32 records and 1 MiB, using ascending ID cursors.

The `task_runs` collection binds a plugin-owned task to an SDK raw run:

```json
{
  "action": "create",
  "value": {
    "key": "run:42",
    "task_id": 12,
    "due_at": 1791600000,
    "run_id": 42
  }
}
```

The task and live run must belong to the same plugin, account and project scope.
Bindings are immutable; the host projects live raw-run status into the existing
app task history and retains a terminal snapshot before raw-run deletion.
Repeating the same key, task, due time and run ID returns the existing binding.
If the run has already been removed, creation requires a terminal `snapshot`
with `status`, optional `detail`, `session_id`, `message_id` and `permission_id`.
A snapshot can also record a preflight failure without a run ID. A run still
present in another scope cannot be replaced with a supplied snapshot.

Reads and ascending-cursor lists include retained legacy history as
`editable: false`; its IDs stay unchanged. Treat all history IDs as opaque
positive integers. New bindings return `editable: true`. Update is unavailable;
delete requires the current revision and a terminal status. Each scope holds at
most 1,000 new bindings, with the same 32-record/1 MiB paging contract as tasks.
Source decides recurrence, monitor outcomes and retention policy. These history
bindings do not dispatch work or call built-in Scheduling behavior.

To prevent a stale event from submitting work after its record changes, a raw run
`submit` may include `prerequisites`, for example:

```json
"prerequisites": [
  {"capability": "collections", "collection": "tasks", "id": 12, "revision": 3}
]
```

Up to eight prerequisites are checked atomically with a new submission. Each
requires the corresponding `records` or `collections` grant and uses the grant's
scope. Missing, changed or disabled records reject the submission without side
effects. Identical-key retries acknowledge existing work even if its record has
advanced. This does not cancel work already queued or running; source must use
the explicit cancellation operation for that.

The `collections` capability also offers read-only `conversations` metadata:

```rust
let page: serde_json::Value = openwebide_plugin_sdk::collections(
    "conversations", &serde_json::json!({"action": "list", "after": 0})
)?;
```

It returns `{enabled, records, next}` with at most 32 records/1 MiB, ordered by
ascending ID. Continue with `after: next` until `next` is null. `read` accepts a
conversation `id`; create/update/delete are rejected. Values include `name`,
`created_at`, `last_activity`, `archived`, `pinned`, `auto_title`, `origin`,
`connection_id`, raw `model_override` settings and connection `{id, model,
enabled}` metadata. Only your granted project's owned conversations appear;
projectless grants see the account's global realm. Message text and connection
credentials are omitted. Read-only record revisions are opaque positive content
tokens; `updated_at` is the activity timestamp. Rank/filter/select targets in
plugin code. Origin-bound jobs can discover their retained origin; project events
have none. These metadata reads do not grant chat or queue-consumption authority.

Read-only `configuration` metadata uses the same collection operations. Record 1
contains account preferences `primary`, `fast`, `default_connection` and shared
`servers: [{id, model, enabled}]`. It contains no server names, URLs, credentials
or unrelated settings. List from cursor 0 or read ID 1 obtains the current snapshot;
later list cursors are empty. Revisions are opaque positive content tokens and
`updated_at` is 0. The value is bounded to 64 KiB and 256 servers. Source owns
default-model precedence and can read it at delivery time instead of using a
creating conversation's older model pin. Conversation `model_override` metadata
uses the app's `{connection_id, model}` shape; raw run `model` uses
`{server_id, model}`, so source must translate those fields.

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
background authority. Released terminal callbacks retain their `{run, data}` payload
when delivery fails or raw runs are deleted. A plugin can replay its result handling
from that snapshot during reconciliation, then delete the terminal job after its
effects are durable. Leave pending and leased callbacks to their delivery worker;
check the reserved `$run:<id>` key against the retained run identity before replay.
The Scheduling reference uses this bounded recovery path. Executable Scheduling
remains unfinished; plugins supply
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
