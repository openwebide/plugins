# First-party plugins

The first-party catalog includes PR Review, Web, Project Memory, Scheduling,
Skill Authoring and independent language families. These use the same validation, installation and project opt-out
contracts as community plugins. See the repository README for current plugins.
Other planned contributions are listed below.

| Package | Intended reference coverage |
| --- | --- |
| GitHub integration | Account connections, MCP tools, review/issue skills and a declarative pull-request panel |
| [PR review skills](pr-review/README.md) | Review skill using core workspace tools |
| Browser testing | Host runtime requirements, browser automation and artifacts |
| Git annotations | Editor hooks, declarative inline/gutter annotations, hover details and existing Git navigation actions |

The [Terraform source](terraform/README.md) now contains an independent grammar
and declarations for the seven related dialects served by terraform-ls. Its
installed lifecycle, public toolkit pin and catalog release remain under verification.
The bundled Git annotations package depends on ongoing editor work and the
language/editor contribution APIs. Supported language families already
use independent packages through the public language contract; Rust,
JavaScript/TypeScript, C#, Python, Go, C/C++, HTML, CSS and JSON declare native
language servers.
