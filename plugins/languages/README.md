# Languages

A host-only language component for Rust, Python, JavaScript/JSX, TypeScript/TSX,
Java, C#, PHP, JSON/JSONC, HTML, CSS/SCSS, Markdown, INI/EditorConfig, XML,
Shell, TOML, YAML, SQL, C, C++ and Go. Embedded languages use the same public
source-bound result contract. Comment, pairing and indentation behavior is
manifest data interpreted by the core editor.

The component uses the public syntax toolkit, retains bounded incremental parser
state, and has no host grants, filesystem access or browser code. Installation,
disablement, updates and failures follow the same lifecycle as other packages.

Development is in progress. Cargo pins the syntax toolkit to an immutable app
source commit. Build with the matching API 5 host SDK patch and pinned WASI SDK;
the syntax toolkit is not a crates.io dependency. The app's editor integration,
default activation and lifecycle verification must finish before this package is
listed or bundled. This source is not a catalog release.
