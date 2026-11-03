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

The official marketplace lists immutable releases of this package. Hosts need
API 5 editor integration; earlier SDK-only app builds need an update before the
component can supply editor features. The app bundles a pinned release on those
hosts, using the same installation and project enablement rules as community
packages.

Cargo pins the syntax toolkit to an immutable app source commit. Build with the
matching API 5 host SDK patch and pinned WASI SDK; the syntax toolkit is not a
crates.io dependency.
