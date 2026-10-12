# Declarative plugin panels

API 4 plugins can declare panels alongside skills, tools and events. Their Rust
runs on the workspace's execution host. The browser uses Open WebIDE's shared
renderer, navigation, themes, forms and responsive layout; plugins cannot ship
browser JavaScript, HTML, CSS, UI WASM or client expressions.

The [panel schema](../schemas/panel.schema.json) defines UI version 1 separately
from the manifest schema and plugin API. The SDK's `panels` module is the canonical
Rust representation. Declare panel definitions under `contributions.panels` and
return the same definitions from `Plugin::panels()`. Installation checks the
compiled component against the manifest before activating it.

## Definitions and handlers

A definition contains a stable `id`, `title`, built-in `icon` and `view` handler.
Its `actions` declare stable IDs, handlers, titles and `requires_approval`.
Optional `refresh_seconds` is between 5 and 3,600 seconds. For example:

```json
{
  "id": "pull-requests",
  "title": "Pull requests",
  "icon": "git_pull_request",
  "view": "render",
  "refresh_seconds": 60,
  "actions": [
    {"id": "publish", "handler": "publish", "title": "Publish review", "requires_approval": true}
  ]
}
```

Implement `Plugin::panel(input)`. A view has no `input.action`; an interaction
identifies its declared action and handler. Return a `PanelDocument` with
`schema_version: 1` and `revision: input.revision + 1`. Use the typed SDK blocks
rather than generating markup. Text, headings, notices, groups, actions, forms,
lists, tables, code, diffs, links and dividers are supported.

View handlers can read scoped records and supported host data, and make HTTP
GET/HEAD requests without bodies. Mutations belong to actions. Every action uses
normal capability grants; mark publishing and other consequential actions as
requiring approval. First-party plugins use these same contracts and privileges.

## Forms, state and secrets

Fields support text, multiline text, checkboxes, selects and `secret` password
inputs. Secret fields must start empty; returning a saved token as their value is
invalid. Save configuration through scoped SDK records. Never place credentials
in UI text, links, button values or opaque view state. Approval previews mask
secret fields while retaining inspectable destinations and other inputs.

Button `values` identify immutable destinations, such as a pull-request number,
repository or reviewed commit. A form adds only declared, typed fields. The host
checks the submitted values against the authoritative view, including disabled
buttons, required fields, lengths and select options. Form fields cannot replace
button values. Required checkboxes must be checked.

`PanelDocument.state` is bounded opaque state carried between revisions. The app
stores canonical views and execution claims in the database, with account,
project, source/version and instance ownership. Repeated request identities do
not acquire another execution claim. Closing, navigation, disablement and updates
reject stale results. Interrupted actions are not automatically replayed. Keep
durable configuration in SDK records and re-read it before consequential actions.

Upstream data has its own revision: bind actions to the external state the user
reviewed and check it before publishing. The PR Review reference binds its form
to both the configuration revision and GitHub head commit, and sends that commit
explicitly when creating a review.

## Bounds and failure handling

Documents have a 256 KiB serialized limit, 256 blocks and six layout nesting
levels. Packages can declare 16 panels and 32 actions per panel. Forms contain up
to 16 fields; action rows contain eight buttons; lists contain 100 items; tables
contain eight columns and 100 rows. Submitted values and opaque state each have
a 32 KiB limit. The schema describes further per-component limits; host checks
also enforce aggregate bytes, identifiers, action references and revisions.

Return useful notices for recoverable upstream failures. Keep complete destinations
and routing values intact when shortening large display content. The app retains
core navigation on errors and pauses automatic refresh during form edits and
approval dialogs. A retry after an ambiguous action reads fresh state rather than
replaying the mutation.

## Reference implementation

[PR Review](../plugins/pr-review/src/lib.rs) owns its GitHub HTTP requests,
configuration records, pull-request listing, diff view and review publishing in
Rust. It imports only the public SDK and general Rust libraries. Its native tests
exercise readonly views, credential handling, configuration/commit changes and
bounded documents. The app runtime also checks the actual compiled WASI component
and its manifest exports against controlled capability responses.

PR Review 0.2.0 requires API 4 and is the executable reference release. Historical
0.1.x releases remain skill-only. Both authenticated bridge transports and the
shared desktop/phone UI exercise the same version-bound action contracts.
See [authoring](authoring.md) for the SDK setup and [custom marketplaces](custom-marketplaces.md)
for publishing your own repository. A component gallery and live builder are
planned on the app docs website, using this same renderer and schema.

The app coordinates requests using input fingerprints rather than persisting
submitted form values. It keeps receipts for eight recent view revisions; requests
from older revisions cannot execute again. Closing a panel clears its documents and
claims and retires its identity. Hour-old idle instances are retired on subsequent
panel requests; an expired view must reopen with a new instance. SDK records are
separate persistent plugin configuration and survive these view cleanups.
