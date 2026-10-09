# OpenWebIDE plugins

Official OpenWebIDE plugin marketplace and reference implementations for agent
tools, language support, and editor extensions.

This repository is being prepared alongside the plugin system design in
[OpenWebIDE's roadmap](https://github.com/openwebide/openwebide/blob/main/docs/roadmap.md).
The catalog is empty; OpenWebIDE does not yet consume it. The skills-only
authoring format has schemas and validation, while runtime contracts remain
drafts rather than a supported plugin API.

## Repository layout

- `marketplace.json`: the first-party catalog, currently with no listings.
- `plugins/`: first-party packages, each in its own directory.
- `schemas/`: package and catalog schemas plus format documentation.
- `scripts/validate.py`: offline validation for packages and catalog entries.
- `tests/`: validator regression coverage.
- `.github/workflows/validate.yml`: package validation and tests in CI.
- `LICENSE`: MIT license for this repository. Packaged upstream dependencies
  retain their own licenses.

## Package model

A plugin is one versioned package containing any combination of skills, MCP
servers/tools, language support, declarative panels, editor annotations and
editor hooks. Contributions share installation, configuration and lifecycle
contracts; they are not separate package types.

Packages install and execute on bridge hosts: the paired native host for local
projects, or the server/container host for remote projects. Browser and phone
clients consume their metadata and service results through OpenWebIDE. Plugins
do not install browser code or native software on client devices.

The database records user-wide installations and configuration, with project
activation and host readiness tracked separately. Hosts maintain persistent Git
caches and commit-pinned package versions. Updates prepare a new version without
changing files used by active runs or language-service sessions.

## Marketplace model

The official marketplace uses the same format as custom sources. A source is a
Git repository URL, ref and `marketplace.json` path; it need not be hosted on
GitHub. Listings may point to package directories here or in other repositories.

Catalogs can track branches, while installed releases resolve to immutable
commits. File associations allow OpenWebIDE to suggest language packages without
sending source files or project paths to catalog hosts. Suggestions do not
implicitly install or enable packages.

## Reference packages

[PR Review](plugins/pr-review/README.md) is a skills-only reference package. Its
skill can be imported manually into OpenWebIDE today; package installation is
not implemented. See [plugins/README.md](plugins/README.md) for the remaining
candidates. No packages are listed as published releases yet.

## Validate locally

Requires Python 3.12 or newer:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-dev.txt
.venv/bin/python scripts/validate.py
.venv/bin/python -m unittest discover -s tests -v
```

Validate a standalone package with `scripts/validate.py --package /path/to/package`.
Validation uses local schemas, never fetches repositories and never executes
package code. See [schema documentation](schemas/README.md) for supported fields
and the checks that still belong to future installation/publishing tooling.

## Contributing

Add skills-only packages under `plugins/<name>/` using the reference manifest.
Run validation and tests before submitting changes. Do not publish placeholder
commit IDs or list unreleased packages in the catalog. Language, MCP and UI
contributions require their contracts in the main app before schema support is
added here. Automated release publishing and the app installer remain future work.
