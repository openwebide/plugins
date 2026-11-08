# CSS and SCSS

CSS and SCSS retain separate language IDs within one independently installable
package, sharing `vscode-css-language-server` on the execution host. Syntax,
folding, source structure and editing rules remain available without native tools.
The package uses the public API 7 language contract and ordinary process-access
approval; its component never runs in browser clients.

Install Node.js 22.22.2 or newer and the pinned upstream tools on each execution host:

```sh
npm install --global --ignore-scripts vscode-langservers-extracted@4.10.0
```

The host starts `vscode-css-language-server --stdio`, with per-document `css` or
`scss` identity. Completion snippets, hover, diagnostics, symbols and reviewed
document/selection formatting use the same facade in local and remote projects.
Supported navigation and actions follow the server's advertised capabilities.
The completion's client-side request for value suggestions does not execute a
native server command. Missing tools preserve ordinary editing.

Open WebIDE maintains the packaging, not the upstream server. See
[Microsoft VS Code](https://github.com/microsoft/vscode) and the independent
[vscode-langservers-extracted project](https://github.com/hrsh7th/vscode-langservers-extracted).
