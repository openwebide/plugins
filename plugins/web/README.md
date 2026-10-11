# Web

Rust source owns provider selection, search result extraction and deduplication,
redirect handling, Stack Overflow question retrieval, HTML sanitization and
Markdown formatting. The SDK supplies general HTTP transport; no built-in Web
handler is called. Requests and execution happen on the bridge host.

Version 0.2.1 is an executable API 3 release in the official catalog.
Installation requires the host source compiler and SDK 0.1.0; packaged hosts
embed its validated component as a default. Production checks exercise source
execution and sanitized page retrieval through both host transports. Official
main integration and the remaining manual PWA checks are pending.

Exports `search_web` and `fetch_web_page`; declares only `http` capability.
Search queries use DuckDuckGo, Stack Exchange, crates.io, GitHub and Wikipedia
without paid API keys. Fetch requires tool approval. The host enforces destination
and response-size limits; the plugin bounds redirects and Markdown output.

Development checks use a checkout of the app's SDK/runtime implementation:

```sh
cargo test --manifest-path plugins/web/Cargo.toml --locked \
  --config 'patch.crates-io.openwebide-plugin-sdk.path="../open-webide/crates/plugin-sdk"'
```

The host runtime's `check_source` example compiles source using the normal
isolated compiler, checks exported schemas against the manifest, and can run a
tool against canned capability responses. From the app checkout:

```sh
cargo run -p openwebide-plugin-runtime --example check_source -- \
  ../plugins/plugins/web fetch_web_page '{"url":"https://example.com/docs"}' \
  ../plugins/plugins/web/fixtures/page.json
cargo run -p openwebide-plugin-runtime --example check_source -- \
  ../plugins/plugins/web search_web '{"query":"Rust reference","limit":1}' \
  ../plugins/plugins/web/fixtures/search.json
```

These checks use no real HTTP requests. The compiler still retrieves locked
public dependencies before entering its offline sandbox. The authoring checker
needs the host compiler and sandbox prerequisites; it does not install a plugin.
