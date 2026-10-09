# First-party reference packages

The [PR Review package](pr-review/README.md) is available for authoring validation
and manual skill import. Release 0.1.0 is commit-pinned in the catalog. Host
installation support prepares the package; agent activation still uses manual
skill import. Other entries below are planned packages.

| Package | Intended reference coverage |
| --- | --- |
| GitHub integration | Account connections, MCP tools, review/issue skills and a declarative pull-request panel |
| [PR review skills](pr-review/README.md) | Implemented reference package; manual skill import using existing tools |
| Browser testing | Host runtime requirements, browser automation and artifacts |
| Terraform | File recognition, syntax, formatting, an existing language server and optional agent workflows |
| Git annotations | Editor hooks, declarative inline/gutter annotations, hover details and existing Git navigation actions |

Terraform and the bundled Git annotations package depend on the ongoing editor
work and language/editor contribution APIs. Existing supported languages are
also intended to become bundled packages through the same contracts.
