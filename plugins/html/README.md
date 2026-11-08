# HTML

HTML syntax, folding, source structure and editing rules, with native code
intelligence from `vscode-html-language-server` on the project's execution host.
This package uses the public API 7 language contract. Its syntax component and
server declaration have the same installation, approval, update and disablement
rules as community language packages. No plugin code runs in the browser.

Install Node.js 22.22.2 or newer and the pinned upstream tools on each execution host:

```sh
npm install --global --ignore-scripts vscode-langservers-extracted@4.10.0
```

The host starts `vscode-html-language-server --stdio`. Completion (including
snippets), hover, document symbols and reviewed document/selection formatting use
the shared language-service facade in local and remote projects. Features remain
subject to the server's advertised capabilities. Missing tools leave ordinary
editing available and report an actionable language-service error.

Embedded syntax resolves through installed language providers and preferences;
missing or disabled providers leave those bodies plain.

Open WebIDE maintains this package; the native language server is extracted from
[Microsoft VS Code](https://github.com/microsoft/vscode) by the independent
[vscode-langservers-extracted project](https://github.com/hrsh7th/vscode-langservers-extracted).
