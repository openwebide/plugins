# Language contributions

API 5 adds language declarations and a versioned host analysis protocol to the
existing package format. The reference components and public host implementation
are in development. The app's production editor migration is incomplete, and the
official catalog does not yet publish a language package. Do not use this guide
as evidence that an older deployed host supports language services.

## Start with a component

Copy [the Notes example](../examples/language-plugin/README.md). It implements a
custom language using only the public SDK, without registering a built-in parser
or changing the app. Its manifest uses `compatibility.pluginApi: 5`, an ordinary
Rust `executable`, and a `contributions.languages` array. Include
`contributions.skills: []` if the package has no skills. A package may combine
language declarations with tools, panels and skills.

Each language definition has a string `id` and display `name`. IDs do not come
from an app-owned language enumeration. Recognition supports extension names
without a leading dot, exact filenames, bounded filename patterns and an optional
first-line regular expression. Define only the features the package supplies.
The host selects among enabled installed providers; conflicting providers need a
user preference rather than an arbitrary installation order.

`configuration` declares line/block comments, brackets, automatic and surrounding
pairs, and indentation/Enter rules. Automatic pairs can exclude declared protected
regions and constrain surrounding text. Block comments use `nested: true` only
when the language permits nesting. The core editor interprets this data; a
configuration-only package needs no executable. Syntax, folding and structure
features require an executable component.

Set `configuration.uniformLineComments: true` only when the declared line marker
or block-comment fallback applies throughout the document. This allows explicit
line-comment commands before analysis, including files above the analysis limit.
The default is false: structural providers wait for source-bound context before
choosing comment rules. Embedded scopes must preserve a uniform declaration's
comment strategy; contradictory results are rejected. Keep the SDK pin current
when using this API 5 field; older SDK snapshots do not accept it.

Implement `Plugin::languages()` and `Plugin::language(input)`, then use
`openwebide_plugin_sdk::export!`. Exported definitions must exactly match the
manifest, including order. The SDK reserves `@openwebide/languages` for metadata
and `@openwebide/language` for requests; these are not agent tools.

The [Languages reference source](../plugins/languages/README.md) uses the optional
public syntax toolkit for its engines. A custom lexer or other engine can use
the SDK directly, as Notes does. Grammar and parser code belongs inside the host
component. Browser clients receive metadata and validated results and do not
execute plugin code.

## Document synchronization and results

Protocol version 1 supports Analyze and Close. Analyze identifies the document,
request, revision, selected language, path, tab width and complete source digest.
Source is either a full snapshot or one UTF-8 byte-range replacement with its
acknowledged base revision. Return NeedsSource if that base is missing; the shared
host facade retries once with a full snapshot. Drop retained source/parser state
on Close and bound caches even if a client never closes its document.

Echo request identity in the response. Ready includes source-bound analysis and
an honest `incremental` flag. TooLarge, Unsupported and NeedsSource contain no
analysis. Notes always scans complete source, so it returns `incremental: false`
even when the source arrived as a delta.

Results contain data, not source transformations or executable UI:

- Highlight rows use ordered UTF-8 byte endpoints relative to each logical row.
  Spans cover the row completely; preserve CRLF and the final empty row.
- Folds, embedded scopes, protected regions, selections and bracket partners use
  complete-source UTF-8 byte offsets. Do not split a scalar or invent offsets
  outside the supplied source. Bracket partners must be reciprocal.
- Embedded scopes identify their contributed language, allowing the core editor
  to choose its declared rules for comments, pairs and indentation.

The public SDK caps analyzed source at 2 MiB, result JSON at 32 MiB and analysis
records at 100,000. Package declarations have separate bounds, including at most
64 language definitions. The host validates declarations, result identity and
source coordinates before publication. A result from an old document revision,
provider or ownership scope cannot update the current editor.

Language components receive no ambient filesystem, network or process access.
Notes and Languages require no host capability grants. Unavailable hosts,
disabled providers, malformed results and component failures must preserve plain
editing and saving. Installation, enablement, updates and removal use the same
package lifecycle as other executable plugins in both project modes.

## Build and verify

Use a pinned host SDK checkout with API 5, the matching Rust toolchain and
`scripts/validate_rust.py`; commit the package's lockfile. Syntax engines containing
C grammars also need the host's pinned WASI SDK, documented in the app's
[plugin authoring guide](https://github.com/openwebide/openwebide/blob/main/docs/plugin-authoring.md#language-plugin-build-toolchain).
Do not use a browser-target C compiler or rely on the developer machine's system
headers for a WASI component.

Test full/delta source, lost bases, Unicode, LF/CRLF, invalid inputs and bounded
cache eviction. Verify the compiled component exports, not just native Rust
functions. The host's `check_language` example exercises the real component
protocol and parser-free result reconstruction. Complete paired/server lifecycle
and editor checks before publishing a catalog release.

Formatting, diagnostics, snippets and language-server metadata are reserved
extension points for subsequent editor/code-intelligence work. A declaration does
not launch a server, install a dependency or grant permission to run processes.
