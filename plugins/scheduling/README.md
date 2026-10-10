# Scheduling

Schedule prompts and monitor follow-up checks on the host.

Requires Open WebIDE plugin API 2. Install from Plugins to enable it across your
projects. Disable it in an individual project when you do not need it.

This reference plugin contributes the `scheduling` platform tool group. The host
uses shared tool implementations, validation and approval rules; the manifest
does not run scripts on browser clients or expand permissions.

The catalog release above is a transitional switch, not a completed behavior
migration. An unlisted Rust migration lives alongside it in `Cargo.toml` and `src/`.
It owns five-field cron/timezone calculations, daylight-saving behavior, task
validation, bounded monitor configuration, and conversation/model selection using the public SDK dependency;
it does not import application internals. Its policy library has native tests and
compiles to WASI, but SDK tool/event handlers, lifecycle recovery, task history and
UI activation are still in progress. Do not install this source as an executable
release yet; `plugin.json` and the live marketplace still describe API 2.
