# JSON and JSONC

JSON and JSON with Comments retain separate language IDs within one independently
installable package. Both share `vscode-json-language-server` on the project's
execution host. Syntax, folding, structure and editing rules use the public
component contract; the native declaration uses API 7 and normal process approval.

Install Node.js 22.22.2 or newer and the pinned upstream tools on each execution host:

```sh
npm install --global --ignore-scripts vscode-langservers-extracted@4.10.0
```

The host starts `vscode-json-language-server --stdio`, enables validation and
advertises the formatter through initialization options. Schema-aware diagnostics,
completion snippets, hover, symbols and reviewed document/selection formatting
use the shared service in both project modes. JSONC keeps comment-aware editing
rules and its own server language ID. Editing stays usable when tools are absent.

Open WebIDE maintains this package. The native server comes from
[Microsoft VS Code](https://github.com/microsoft/vscode), distributed by the
independent [vscode-langservers-extracted project](https://github.com/hrsh7th/vscode-langservers-extracted).
