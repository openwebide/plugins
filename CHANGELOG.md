# Changelog

## [Unreleased]

- Add plugin API 2 platform tool groups and first-party Web, Project Memory, Scheduling and Skill Authoring plugins.

- Inherit each release’s repository from its marketplace source; catalog entries
  declare only the pinned commit and package directory and reject repository overrides.

- Publish PR Review 0.1.0 in the official catalog with an immutable Git commit
  and document host preparation separately from skill activation.

- Add skills-only package and Git catalog schemas, offline validation and CI.
- Add a first-party PR review package with a reusable review skill and checklist.
  Its skill can be imported manually; package installation does not activate it.
