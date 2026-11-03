# Notes language example

This API 5 Rust component implements a language outside the app's existing
language set. It recognizes `.notes` files, colors `note` and `task` keywords,
quoted strings, numbers and `#` comments, and folds brace-delimited blocks.
Comment, pairing and indentation rules are ordinary declarative contributions.

```notes
note {
  task "Keep Unicode 文😀 and CRLF source positions intact" # comment
}
```

The component runs on the selected project's bridge host using the public SDK.
It has no host capability grants, filesystem access or model tools. It supports
full source snapshots, revision-checked source replacements and document close;
its bounded in-component cache is disposable. A lost source base requests a full
snapshot. Analysis itself is a complete lexical pass, so it does not claim
incremental parsing.

Build and test against the public SDK checkout with `scripts/validate_rust.py`.
Install it through a custom marketplace or a pinned Git source, using the same
language lifecycle as bundled packages. No browser scripts or app-specific
provider identity is involved. This example is not a catalog release.
