# Scheduling

The API 3 Rust source owns scheduled prompts and bounded follow-up checks.
It uses only the public Open WebIDE SDK: scoped collections and records, clocks, one-shot
jobs, raw prompt runs and bounded model completion. It does not call built-in
Scheduling behavior or import application crates.

The six tools are `schedule_list`, `schedule_create`, `schedule_update`,
`schedule_set_enabled`, `schedule_delete` and `monitor`. Source validates five-field cron expressions and
IANA timezones, calculates recurrence across daylight-saving changes, names tasks,
selects conversations/models at delivery time, cancels prior work after edits and
interprets monitor results. Pause/resume preserves the existing title and rejects
stale revisions or elapsed one-time schedules; UI monitor cancellation verifies
the owning conversation. Declared `task_due` and `task_completed` events use
revision-guarded raw submissions. Saved work uses project-scoped events; monitor
records are anchored to their owned conversation and expire within 24 hours.
A declared one-minute `reconcile` background event adopts idle legacy records,
rebuilds missing/failed timers, repairs run/history bookkeeping and terminal
results, completes interrupted deletions and invalidates old work after source
version changes. This cancels prior-generation jobs and active runs; their
terminal history retains the original message, and accepted one-time prompts
are not requeued. Interrupted monitor checks still consume their original finite
budget and retain their original expiry. Repairs use current revisions; a racing
edit or unavailable host operation is retried on a later tick. Reconciliation
visits at most eight records per tick with a bounded host-call budget and a
private persistent cursor; failing records do not starve later ones. A work-budget
interruption preserves partial progress and resumes that record on the next tick.

Version 0.2.1 is listed in the candidate catalog and included in the compiled
default bundle. Official main integration remains pending. The source has native policy/workflow tests
and an actual compiled-component contract against shared host storage in both
project modes. Source creates SDK history bindings; the app projects live run
progress and retains terminal snapshots after raw-run deletion. Production
HTTP deployment checks now cover CRUD, configured naming, delivered agent work,
bounded monitors, cancellation, pending-task recovery after deployment restarts
and pending-occurrence handoff between two compiled source versions on both hosts.
An executing component keeps its original code and grant across installation
updates; later invocations use the updated version. Deployment checks also verify
recovery from validation failures, admission limits and cancellation freeing
invocation capacity.
Restarting during a streamed agent response records one interrupted result with
the original message and does not replay the accepted prompt. With task history
at quota, source journals a real preflight failure; bounded Rust reconciliation
frees space and flushes that journal once through the public SDK.
Current production checks also verify fresh defaults, catalog updates and
configured naming followed by completed agent runs on both host transports.
Task and monitor UI mutations use declared SDK tools. Manual browser checks for
retention, quota recovery and legacy task adoption remain pending.
