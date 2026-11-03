# Languages

A host-only language component for Rust, Python, JavaScript/JSX, TypeScript/TSX,
Java, C#, PHP, JSON/JSONC, HTML, CSS/SCSS, Markdown, INI/EditorConfig, XML,
Shell, TOML, YAML, SQL, C, C++ and Go. Embedded languages use the same public
source-bound result contract. Comment, pairing and indentation behavior is
manifest data interpreted by the core editor. A plain-text contribution supplies
generic region folding for otherwise unrecognized files; specific language
matches take precedence over its catch-all pattern.

The component uses the public syntax toolkit, retains bounded incremental parser
state, and has no host grants, filesystem access or browser code. Installation,
disablement, updates and failures follow the same lifecycle as other packages.

This aggregate prototype is unlisted and is being replaced by independent
language-family packages, grouped by the intended language server's coverage.
Related dialects share a package; unrelated families have independent installation,
provider selection, enablement and versioning. Embedded regions must resolve
through installed providers. LSP execution belongs to the following app goal.

Cargo pins the syntax toolkit to an immutable app source commit. This source is
retained for protocol/reference history; do not publish it as the default language
package. See the language guide for the current packaging contract.
