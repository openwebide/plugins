# Changelog

## [Unreleased]

- Publish executable API 3 releases of Web, Project Memory, Scheduling and Skill Authoring at 0.2.1, plus PR Review 0.1.2, with immutable source pins and individual structured JSON icons.
- Move first-party behavior into Rust using the public SDK, with the same capabilities and approval rules as community plugins. Distribution builds compile and embed the selected official defaults.
- Document custom marketplaces, Rust source compilation on install, immutable publishing and validation against the pinned public Git SDK.
- Add capability-gated HTTP, scoped records/shared collections, clocks, model completion, durable jobs and raw prompt runs. Preserve source pins, authenticated scope, bounded execution and recoverable delivery.
- Implement Memory naming/context, Skill Authoring tools/resources/context and Scheduling recurrence, monitors, failure journals, history retention and reconciliation in plugin source. App UI actions share the public SDK path in both project modes.

- Publish the four API 2 plugins and PR Review 0.1.1 with immutable source pins in the Open WebIDE marketplace.

- Add plugin API 2 platform tool groups and first-party Web, Project Memory, Scheduling and Skill Authoring plugins.

- Inherit each release’s repository from its marketplace source; catalog entries
  declare only the pinned commit and package directory and reject repository overrides.

- Publish PR Review 0.1.0 in the official catalog with an immutable Git commit
  and document host preparation separately from skill activation.

- Add skills-only package and Git catalog schemas, offline validation and CI.
- Add a first-party PR review package with a reusable review skill and checklist.
  Its skill can be imported manually; package installation does not activate it.
