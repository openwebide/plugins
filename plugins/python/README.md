# Python

Python syntax, folding, source structure and editing rules, with code intelligence
through `pyright-langserver --stdio` on the selected project execution host.
The language ID remains `python`; `.py` files retain their existing behavior and
`.pyi` type stubs use the same language service.

Install Node.js 22.22.2 or newer, Python 3, and Pyright (`npm install -g
pyright@1.1.414`) on paired hosts, with `pyright-langserver` on the bridge's
`PATH`. OpenWebIDE's Docker image includes these tools. Each project uses its own
`pyrightconfig.json` or `[tool.pyright]` in `pyproject.toml`; Pyright owns import,
interpreter and virtual-environment resolution. For example:

```toml
[tool.pyright]
venvPath = "."
venv = ".venv"
typeCheckingMode = "standard"
```

Create/install dependencies in that environment on the execution host. In local
mode this is the paired native folder; in remote mode it is the server's project
folder. Python configuration paths are relative to that project, not the browser.

The independent API 6 component uses the public SDK and the ordinary reviewed
native-execution grant, shared language-service lifecycle and result validation.
Both project modes support diagnostics, symbol information, completion,
definitions, references, document symbols and reviewed rename edits. Open buffers
are synchronized without saving; review stages edits into editor history and
recovery until Save. Pyright does not provide document formatting. Commands that
require server-side execution are not run automatically.

Missing tools, denied approval, disabled packages or server failures leave plain
editing available. Installation, updates, provider preferences, disablement and
removal use the normal plugin lifecycle. No Python-specific host privileges or
browser code are required.
