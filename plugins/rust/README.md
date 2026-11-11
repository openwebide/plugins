# Rust

Rust syntax, folding, source structure, editing rules and a `rust-analyzer`
language-server declaration.

Version 0.3.0 requires an Open WebIDE host with plugin API 9. The syntax component
uses the public Rust SDK; the native language server runs separately on the same
project execution host. Browser clients render results and edit text.

Install `rust-analyzer`, `rust-src` and `rustfmt` for the project's Rust toolchain
on that host. For a rustup toolchain, run:

```sh
rustup component add rust-analyzer rust-src rustfmt
```

The manifest selects `rust-analyzer` from the host's PATH, recognizes Cargo and
`rust-project.json` project roots, and enables the server's own file watcher.
The host manages its lifetime and document synchronization. The server supplies
diagnostics, completion, hover, navigation, symbols, rename, formatting and code
actions according to its advertised capabilities.

The public cache declaration redirects Cargo build output to the execution host's
derived-cache directory outside the project. Native namespaces include the actual
rust-analyzer version and declared Cargo/toolchain inputs. The server retains
responsibility for build freshness; user settings and installation choices stay
in the database. Build scripts still require the normal native-process review.

Installation, provider preferences, disablement, removal and updates use the same
public lifecycle as community packages. Updating an older syntax-only release
requires the ordinary review for added `language_server` access. Syntax and plain
editing remain available when the native server is unavailable.
