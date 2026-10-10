# Rust source plugin example

Development example for the source-compilation runtime being implemented alongside
this package. It is not listed in the marketplace or usable with released API 1/2
hosts. The Rust crate owns the tool behavior; it contains no built-in feature proxy.

The host supplies the pinned SDK, builds with Cargo.lock in isolation, verifies
that exported tool definitions match plugin.json, and executes the resulting WASM
component on the bridge. No author-side artifact publishing is required.
