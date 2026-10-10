# Changelog

## [Unreleased]

- Add an unlisted API 3 Rust Skill Authoring source implementation that owns its six tool handlers, validation, resource paging, update preservation, authoring workflow and read-only context catalog through the public SDK. Full deployment and executable default migration remain in progress.

- Document custom marketplace setup, immutable publishing and validation; distinguish
  the supported format from the planned Rust SDK/WASM artifact flow and require
  first-party executable plugins to own their behavior through the public contract.

- Publish the four API 2 plugins and PR Review 0.1.1 with immutable source pins in the Open WebIDE marketplace.

- Add plugin API 2 platform tool groups and first-party Web, Project Memory, Scheduling and Skill Authoring plugins.

- Inherit each release’s repository from its marketplace source; catalog entries
  declare only the pinned commit and package directory and reject repository overrides.

- Publish PR Review 0.1.0 in the official catalog with an immutable Git commit
  and document host preparation separately from skill activation.

- Add skills-only package and Git catalog schemas, offline validation and CI.
- Add a first-party PR review package with a reusable review skill and checklist.
  Its skill can be imported manually; package installation does not activate it.
