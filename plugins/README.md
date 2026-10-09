# First-party plugins

The first-party catalog includes PR Review, Web, Project Memory, Scheduling and
Skill Authoring. These use the same validation, installation and project opt-out
contracts as community plugins. See the repository README for current plugins.
Other planned contributions are listed below.

| Package | Intended reference coverage |
| --- | --- |
| GitHub integration | Account connections, MCP tools, review/issue skills and a declarative pull-request panel |
| [PR review skills](pr-review/README.md) | Review skill using core workspace tools |
| Browser testing | Host runtime requirements, browser automation and artifacts |
| Terraform | File recognition, syntax, formatting, an existing language server and optional agent workflows |
| Git annotations | Editor hooks, declarative inline/gutter annotations, hover details and existing Git navigation actions |

Terraform and the bundled Git annotations package depend on the ongoing editor
work and language/editor contribution APIs. Existing supported languages are
also intended to become bundled packages through the same contracts.
