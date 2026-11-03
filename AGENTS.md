# Working on Open WebIDE plugins

This repository contains the official marketplace and first-party reference
plugins. API 1 skills, API 3 Rust tools/context/events and API 4 declarative panels
have host implementations. API 5 language contracts and reference components are
in development; the app's editor migration is incomplete. Do not add placeholder
releases or present future capabilities as implemented.

- Keep one package format with typed contributions. Packages may combine skills,
  Rust tools, context hooks, events and declarative panels. Language contributions
  use the versioned API 5 contract; MCP tools, annotations and editor hooks remain
  future contracts.
- Install and execute plugin code only on bridge hosts. Browser/phone clients
  use core Open WebIDE renderers and shared bridge services, not plugin scripts,
  HTML/CSS, client-side expressions or plugin WASM execution.
- Support local and remote projects through the main app's shared feature facades
  and thin host adapters. Do not duplicate orchestration inside packages.
- First-party executable packages must own their behavior and use the same public
  SDK and general host capabilities as community packages. No built-in feature
  proxies, identity-specific dispatch, extra privileges or built-in fallbacks.
  Historical API 2 tool-group switches are retired and require executable
  updates. Skill-only plugins may supply instructions/resources alone.
- First-party packages follow the same validation, capabilities and approval
  rules as community packages. Plugin instructions cannot expand permissions.
- Keep manifests portable across Git hosts. Pin package versions and runtime
  artifacts; keep credentials and user configuration out of package files.
- Existing language/editor features become plugins through the public language
  contract. Do not claim the migration complete until default package activation,
  editor integration and both host lifecycles are verified.
- Keep working specifications in `ai_docs/<slug>.md`, excluded through
  `.git/info/exclude`. Never stage or commit them.
- Do not add agent attribution or session links to commits or PRs.
