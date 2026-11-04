# Language contributions

API 5 adds language declarations and a versioned host analysis protocol to the
existing package format. The Notes example has its own catalog. The
official catalog lists 19 independent language-family packages; the
aggregate Languages prototype remains unlisted.

Use one package per intended LSP's language coverage. Related dialects served by
one server belong together, such as JavaScript/TypeScript/JSX/TSX or C/C++.
Packages own recognition, syntax, folding and editing rules now; LSP execution
is a following app goal. Shared parsers belong in public libraries, while
embedded regions resolve through installed providers. API 5 editor integration
is required; earlier SDK-only builds need an update.

## Start with a component

Copy [the Notes example](../examples/language-plugin/README.md). It implements a
custom language using only the public SDK, without registering a built-in parser
or changing the app. Its manifest uses `compatibility.pluginApi: 5`, an ordinary
Rust `executable`, and a `contributions.languages` array. Include
`contributions.skills: []` if the package has no skills. A package may combine
language declarations with tools, panels and skills.

The [example marketplace](../examples/language-marketplace.json) lists immutable
Notes releases separately from the official catalog. On a host with API 5 editor
integration, add this repository with catalog path
`examples/language-marketplace.json` in Settings → Plugins, then install Notes
language from Plugins. Keep the default `marketplace.json` source as well. This
example does not make API 5 available on an older deployed app.

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

`surroundingPairs` describe explicit selection wrapping and do not require parsed
regions. Context exclusions in `autoClosingPairs.notIn` apply to automatic
insertion at an empty caret, where unknown protected regions remain unavailable.

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

The independent [Rust](../plugins/rust/README.md),
[JavaScript/TypeScript](../plugins/javascript-typescript/README.md),
[C/C++](../plugins/c-cpp/README.md) and [Markdown](../plugins/markdown/README.md)
sources use the optional public syntax toolkit. Set `default-features = false`
and select the package's family feature, then use `LanguageSyntax::new_delegated`.
The compatibility `all` default is for legacy consumers. A custom lexer or other
engine can use the SDK directly, as Notes does. Grammar and parser code belongs
inside the host component. Browser clients receive metadata and validated results
and do not execute plugin code.

First-party packages preserve the existing 24 language IDs in 19 independently
installable families. `javascript-typescript` owns JS/JSX/TS/TSX; `c-cpp` owns C/C++;
`markdown` owns block/inline Markdown. JSON/JSONC, CSS/SCSS and INI/EditorConfig
keep their related formats together. Plain text's optional component supplies
region folding; core text editing remains available without it. The old aggregate
source is retained unlisted for historical compatibility.

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
- Embedded scopes identify a language ID or alias. The shared document service
  resolves the selected installed provider, including account/project preferences.
  Missing, disabled, ambiguous or failed providers leave the body plain. Child
  tokens, folds and structural positions are translated into the outer source;
  nested delegation uses a shared source/work budget and depth limit. Removed
  bodies release their host document sessions. HTML and Markdown do not bundle
  other families' parsers; Markdown fences can name custom language providers.

The public SDK caps analyzed source at 2 MiB, result JSON at 32 MiB and analysis
records at 100,000. Package declarations have separate bounds, including at most
64 language definitions. The host validates declarations, result identity and
source coordinates before publication. A result from an old document revision,
provider or ownership scope cannot update the current editor.

Language components receive no ambient filesystem, network or process access.
Notes and the first-party language families require no host capability grants.
Unavailable hosts,
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
protocol and parser-free result reconstruction. Pass `--family-matrix` after the
component and manifest paths to cover every declared family ID, including
dialects, embedded delegation and Unicode/CRLF. Complete paired/server lifecycle
and editor checks before publishing a catalog release.
For custom IDs outside those first-party fixtures, pass a sample source file
instead of `--family-matrix`.

Formatting, diagnostics, snippets and language-server metadata are reserved
extension points for subsequent editor/code-intelligence work. A declaration does
not launch a server, install a dependency or grant permission to run processes.
