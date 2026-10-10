# Scheduling

The unlisted API 3 Rust source owns scheduled prompts and bounded follow-up checks.
It uses only the public Open WebIDE SDK: scoped collections, clocks, one-shot
jobs, raw prompt runs and bounded model completion. It does not call built-in
Scheduling behavior or import application crates.

The five tools are `schedule_list`, `schedule_create`, `schedule_update`,
`schedule_delete` and `monitor`. Source validates five-field cron expressions and
IANA timezones, calculates recurrence across daylight-saving changes, names tasks,
selects conversations/models at delivery time, cancels prior work after edits and
interprets monitor results. Declared `task_due` and `task_completed` events use
revision-guarded raw submissions. Saved work uses project-scoped events; monitor
records are anchored to their owned conversation and expire within 24 hours.

This migration is **not released in the live marketplace yet**. The catalog still
pins the earlier API 2 platform switch. The source has native policy/workflow tests
and component exports; production installation, crash recovery, task-history/UI
integration, retention and default deployment remain in progress. Do not treat
this draft as a completed install/run/update lifecycle.
