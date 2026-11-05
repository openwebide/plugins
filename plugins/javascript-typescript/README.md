# JavaScript and TypeScript

Syntax, folding, source structure, editing rules and native code intelligence for
JavaScript, JSX, TypeScript and TSX through `typescript-language-server`.

Version 0.2.0 requires Open WebIDE plugin API 7. The syntax component uses the
public Rust SDK and syntax toolkit; the language server runs on the project's
execution host. Browser clients render results and apply reviewed text edits.

The server image includes Node.js and locked language-server tools. On a paired
host, install Node.js 22.22.2 or newer, then install the verified tools:

```sh
npm install -g typescript-language-server@6.0.3 typescript@6.0.3
```

The manifest runs `typescript-language-server --stdio` from the host's PATH. The
server can use the project's local TypeScript installation, with the installed
TypeScript package as its fallback. It discovers `tsconfig.json`, `jsconfig.json`
and nested configured projects inside the selected project folder. Automatic
typing acquisition is disabled; install project dependencies through the normal
project workflow.

Related dialects share one native service. Published language IDs remain
`javascript`, `jsx`, `typescript` and `tsx`; API 7 maps the latter JSX dialects to
the server's `javascriptreact` and `typescriptreact` document IDs. Unsaved open
files participate together in navigation, references, completion imports and
rename. Independent editing clients retain separate unsaved buffers.

The server supplies diagnostics, symbol information, completion, definitions,
references, document symbols, rename, formatting and text-edit code actions.
Edits are reviewed and staged as unsaved editor changes with undo and recovery;
ordinary Save writes them. Server commands and workspace file operations remain
outside the editor's current text-edit contract.

Installation, preferences, disablement, removal and updates use the same public
lifecycle as community packages. Updating a syntax-only release requires the
ordinary review for added `language_server` access. Syntax and plain editing
remain available when the server or its toolchain is unavailable.
