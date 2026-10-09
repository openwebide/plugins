# Working on OpenWebIDE plugins

This repository contains the official marketplace and first-party reference
packages. The plugin API is still being designed; do not present draft examples
as working packages or add placeholder releases to the live catalog.

- Keep one package format with typed contributions. Packages may combine skills,
  MCP tools, language services, panels, annotations and editor hooks.
- Install and execute plugin code only on bridge hosts. Browser/phone clients
  use core OpenWebIDE renderers and shared bridge services, not plugin scripts,
  HTML/CSS, client-side expressions or plugin WASM execution.
- Support local and remote projects through the main app's shared feature facades
  and thin host adapters. Do not duplicate orchestration inside packages.
- First-party packages follow the same validation, capabilities and approval
  rules as community packages. Plugin instructions cannot expand permissions.
- Keep manifests portable across Git hosts. Pin package versions and runtime
  artifacts; keep credentials and user configuration out of package files.
- Existing language/editor features become plugins only after the ongoing editor
  work and relevant contribution contracts are ready.
- Keep working specifications in `ai_docs/<slug>.md`, excluded through
  `.git/info/exclude`. Never stage or commit them.
- Do not add agent attribution or session links to commits or PRs.
