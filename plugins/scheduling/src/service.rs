//! Scheduling workflows over public SDK primitives. No application feature dispatch.
use crate::{
    configuration::Configuration,
    conversations::{self, Conversation},
    schedule::*,
};
use openwebide_plugin_sdk::{
    Outcome,
    serde_json::{self, Value, json},
};
use serde::{Deserialize, Serialize};

pub trait Host {
    fn call(&mut self, capability: &str, request: Value) -> Result<Value, String>;
}
pub struct SdkHost;
impl Host for SdkHost {
    fn call(&mut self, capability: &str, request: Value) -> Result<Value, String> {
        openwebide_plugin_sdk::request(capability, &request)
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub pending: Vec<Pending>,
    #[serde(default)]
    pub last_result: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pending {
    pub id: i64,
    pub key: String,
    pub due_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Monitor {
    pub session_id: i64,
    pub interval_seconds: i64,
    pub remaining: i64,
    pub expires_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskValue {
    pub draft: Draft,
    pub next_run: Option<i64>,
    #[serde(default)]
    pub state: State,
    pub monitor: Option<Monitor>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Task {
    pub id: i64,
    pub revision: i64,
    pub value: TaskValue,
}
#[derive(Deserialize)]
struct Page<T> {
    records: Vec<T>,
    next: Option<i64>,
}
fn clock(host: &mut impl Host) -> Result<i64, String> {
    host.call("clock", json!({}))?["unix_seconds"]
        .as_i64()
        .filter(|now| *now >= 0)
        .ok_or("Invalid host clock".into())
}
fn collection(host: &mut impl Host, name: &str, operation: Value) -> Result<Value, String> {
    host.call(
        "collections",
        json!({"collection":name,"operation":operation}),
    )
}
fn all<T: serde::de::DeserializeOwned>(host: &mut impl Host, name: &str) -> Result<Vec<T>, String> {
    let mut result = Vec::new();
    let mut after = 0;
    for _ in 0..64 {
        let page: Page<T> = serde_json::from_value(collection(
            host,
            name,
            json!({"action":"list","after":after}),
        )?)
        .map_err(|error| error.to_string())?;
        result.extend(page.records);
        match page.next {
            Some(next) if next > after => after = next,
            None => return Ok(result),
            _ => return Err("Invalid collection cursor".into()),
        }
    }
    Err("Collection traversal exceeded its bounded page budget".into())
}
fn tasks(host: &mut impl Host) -> Result<Vec<Task>, String> {
    all::<Value>(host, "tasks")?
        .into_iter()
        .filter(|row| row["value"]["editable"].as_bool() != Some(false))
        .map(|row| serde_json::from_value(row).map_err(|error| error.to_string()))
        .collect()
}
fn task(host: &mut impl Host, id: i64) -> Result<Task, String> {
    tasks(host)?
        .into_iter()
        .find(|task| task.id == id)
        .ok_or("Task not found or owned by another plugin".into())
}

fn save(host: &mut impl Host, previous: Option<&Task>, value: TaskValue) -> Result<Task, String> {
    let operation = previous.map_or_else(
        || json!({"action":"create","value":value}),
        |record| json!({"action":"update","id":record.id,"revision":record.revision,"value":value}),
    );
    let mut page: Page<Task> = serde_json::from_value(collection(host, "tasks", operation)?)
        .map_err(|error| error.to_string())?;
    if page.records.len() != 1 {
        return Err("Task mutation returned no record".into());
    }
    Ok(page.records.remove(0))
}
fn origin(rows: &[Conversation], session: i64) -> Result<i64, String> {
    rows.iter()
        .find(|row| {
            if session > 0 {
                row.id == session
            } else {
                row.value.origin
            }
        })
        .map(|row| row.id)
        .ok_or("Choose a conversation in this project".into())
}
fn named(host: &mut impl Host, mut draft: Draft, now: i64) -> Result<Draft, String> {
    if draft.session_target == SessionTarget::Existing {
        draft.session_id = origin(&all(host, "conversations")?, draft.session_id)?;
    }
    if draft.title.trim().is_empty() {
        draft.auto_title = true;
    }
    draft.validate(now)?;
    if draft.auto_title {
        let fallback = draft
            .prompt
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(80)
            .collect::<String>();
        let mut title = None;
        for profile in ["fast", "primary"] {
            if let Ok(value)=host.call("completion",json!({"system_prompt":"Name this scheduled task in at most eight words. Return only a plain title, without quotes or punctuation wrapping.","prompt":draft.prompt,"profile":profile,"max_output_tokens":64})) {
                let text=value["text"].as_str().unwrap_or("").trim().trim_matches('"').trim();
                if !text.is_empty() && text.chars().count()<=120 && !text.chars().any(char::is_control) {title=Some(text.to_owned());break;}
            }
        }
        draft.title = title.unwrap_or(fallback);
    }
    Ok(draft)
}
fn arm(host: &mut impl Host, task: &Task) -> Result<(), String> {
    let Some(due) = task.value.next_run.filter(|_| task.value.draft.enabled) else {
        return Ok(());
    };
    host.call("jobs",json!({"action":"schedule","key":format!("task:{}:{}:{due}",task.id,task.value.state.generation),
        "scope":"project","due_at":due,
        "expires_at":task.value.monitor.as_ref().map(|monitor|monitor.expires_at),
        "event":"task_due","payload":{"task_id":task.id,"generation":task.value.state.generation,"due_at":due}}))?;
    Ok(())
}
/// Invalidate the task generation before calling this; old actors cannot queue new work.
fn cancel_work(host: &mut impl Host, id: i64) -> Result<(), String> {
    let prefix = format!("task:{id}:");
    for (capability, field, cursor) in [
        ("jobs", "jobs", "next_after"),
        ("runs", "runs", "next_after"),
    ] {
        let mut after = 0;
        for page_index in 0..64 {
            let page = host.call(capability, json!({"action":"list","after":after}))?;
            for record in page[field].as_array().ok_or("Invalid work page")? {
                let key = record["key"].as_str().ok_or("Invalid work key")?;
                let state = record["state"].as_str().ok_or("Invalid work state")?;
                if key.starts_with(&prefix)
                    && matches!(
                        state,
                        "pending" | "leased" | "running" | "blocked" | "waiting"
                    )
                {
                    host.call(
                        capability,
                        json!({"action":"cancel","id":record["id"],"revision":record["revision"]}),
                    )?;
                }
            }
            match page[cursor].as_i64() {
                Some(next) if next > after => after = next,
                None => break,
                _ => return Err("Invalid work cursor".into()),
            }
            if page_index == 63 {
                return Err("Work traversal exceeded its bounded page budget".into());
            }
        }
    }
    Ok(())
}
fn excerpt(task: &Task) -> Value {
    json!({"id":task.id,"revision":task.revision,"title":task.value.draft.title,
        "prompt_excerpt":task.value.draft.prompt.chars().take(256).collect::<String>(),
        "session_id":task.value.draft.session_id,"session_target":task.value.draft.session_target,
        "schedule":task.value.draft.schedule,"enabled":task.value.draft.enabled,
        "next_run":task.value.next_run,"last_run":task.value.state.last_result})
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    List,
    Create {
        draft: Draft,
    },
    Update {
        id: i64,
        revision: i64,
        draft: Draft,
    },
    Delete {
        id: i64,
        revision: i64,
    },
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum MonitorCommand {
    Start {
        prompt: String,
        delay_seconds: i64,
        #[serde(default = "interval")]
        interval_seconds: i64,
        #[serde(default = "checks")]
        max_checks: i64,
        #[serde(default)]
        session_id: i64,
    },
    List {
        #[serde(default)]
        session_id: i64,
    },
    Cancel {
        id: i64,
        revision: i64,
    },
}
const fn interval() -> i64 {
    600
}
const fn checks() -> i64 {
    1
}
fn outcome(value: Value, summary: &str) -> Outcome {
    Outcome {
        ok: true,
        content: value.to_string(),
        summary: summary.into(),
    }
}
fn create(
    host: &mut impl Host,
    draft: Draft,
    monitor: Option<Monitor>,
    now: i64,
) -> Result<Task, String> {
    let count = tasks(host)?
        .iter()
        .filter(|task| task.value.monitor.is_some() == monitor.is_some())
        .count();
    if count >= if monitor.is_some() { 20 } else { 100 } {
        return Err(
            "This scope has reached its task limit (20 monitors or 100 saved tasks)".into(),
        );
    }
    let next_run = if draft.enabled {
        draft.schedule.next_after(now)?
    } else {
        None
    };
    let created = save(
        host,
        None,
        TaskValue {
            draft,
            next_run,
            monitor,
            state: State {
                generation: 1,
                ..State::default()
            },
        },
    )?;
    if let Err(error) = arm(host, &created) {
        collection(
            host,
            "tasks",
            json!({"action":"delete","id":created.id,"revision":created.revision}),
        )?;
        return Err(error);
    }
    Ok(created)
}
pub fn execute(host: &mut impl Host, name: &str, mut arguments: Value) -> Result<Outcome, String> {
    if name == "monitor" {
        let command: MonitorCommand =
            serde_json::from_value(arguments).map_err(|error| error.to_string())?;
        let now = clock(host)?;
        return match command {
            MonitorCommand::Start {
                prompt,
                delay_seconds,
                interval_seconds,
                max_checks,
                session_id,
            } => {
                let rows = all::<Conversation>(host, "conversations")?;
                let session = origin(&rows, session_id)?;
                let monitored = tasks(host)?
                    .into_iter()
                    .filter(|task| {
                        task.value
                            .monitor
                            .as_ref()
                            .is_some_and(|monitor| monitor.session_id == session)
                    })
                    .map(|task| task.id)
                    .collect::<Vec<_>>();
                let mut after = 0;
                for page_index in 0..64 {
                    let page = host.call("runs", json!({"action":"list","after":after}))?;
                    if page["runs"]
                        .as_array()
                        .ok_or("Invalid run page")?
                        .iter()
                        .any(|run| {
                            run["session_id"].as_i64() == Some(session)
                                && matches!(run["state"].as_str(), Some("running" | "blocked"))
                                && run["key"].as_str().is_some_and(|key| {
                                    monitored
                                        .iter()
                                        .any(|id| key.starts_with(&format!("task:{id}:")))
                                })
                        })
                    {
                        return Err("A monitor check cannot start another monitor to extend its deadline; configure bounded repeats when starting it".into());
                    }
                    match page["next_after"].as_i64() {
                        Some(next) if next > after => after = next,
                        None => break,
                        _ => return Err("Invalid run cursor".into()),
                    }
                    if page_index == 63 {
                        return Err("Run traversal exceeded its bounded page budget".into());
                    }
                }
                let (draft, expires) = MonitorStart {
                    prompt,
                    delay_seconds,
                    interval_seconds,
                    max_checks,
                }
                .draft(session, now)?;
                let monitor = Monitor {
                    session_id: session,
                    interval_seconds,
                    remaining: max_checks,
                    expires_at: expires,
                };
                let saved = create(host, draft, Some(monitor), now)?;
                Ok(outcome(json!([excerpt(&saved)]), "Monitor started"))
            }
            MonitorCommand::List { session_id } => {
                let session = origin(&all(host, "conversations")?, session_id)?;
                let records = tasks(host)?;
                Ok(outcome(
                    json!(
                        records
                            .iter()
                            .filter(|task| task.value.draft.enabled
                                && task
                                    .value
                                    .monitor
                                    .as_ref()
                                    .is_some_and(|monitor| monitor.session_id == session
                                        && monitor.expires_at > now))
                            .map(excerpt)
                            .collect::<Vec<_>>()
                    ),
                    "Monitors listed",
                ))
            }
            MonitorCommand::Cancel { id, revision } => {
                remove(host, id, revision, true)?;
                Ok(outcome(json!([]), "Monitor cancelled"))
            }
        };
    }
    let action = match name {
        "schedule_list" => "list",
        "schedule_create" => "create",
        "schedule_update" => "update",
        "schedule_delete" => "delete",
        _ => return Err("Unknown scheduling tool".into()),
    };
    let args = arguments
        .as_object_mut()
        .ok_or("Expected an argument object")?;
    if args.contains_key("action") {
        return Err("Unexpected action argument".into());
    }
    args.insert("action".into(), json!(action));
    let command: Command = serde_json::from_value(arguments).map_err(|error| error.to_string())?;
    let now = clock(host)?;
    match command {
        Command::List => Ok(outcome(
            json!(
                tasks(host)?
                    .iter()
                    .filter(|task| task.value.monitor.is_none())
                    .map(excerpt)
                    .collect::<Vec<_>>()
            ),
            "Tasks listed",
        )),
        Command::Create { draft } => {
            let draft = named(host, draft, now)?;
            let saved = create(host, draft, None, now)?;
            Ok(outcome(json!([excerpt(&saved)]), "Task created"))
        }
        Command::Update {
            id,
            revision,
            draft,
        } => {
            let old = task(host, id)?;
            if old.revision != revision || old.value.monitor.is_some() {
                return Err("Saved task revision changed".into());
            }
            let draft = named(host, draft, now)?;
            let next_run = if draft.enabled {
                draft.schedule.next_after(now)?
            } else {
                None
            };
            let generation = old
                .value
                .state
                .generation
                .checked_add(1)
                .ok_or("Task generation exhausted")?;
            let saved = save(
                host,
                Some(&old),
                TaskValue {
                    draft,
                    next_run,
                    monitor: None,
                    state: State {
                        generation,
                        last_result: old.value.state.last_result.clone(),
                        pending: vec![],
                    },
                },
            )?;
            cancel_work(host, id)?;
            arm(host, &saved)?;
            Ok(outcome(json!([excerpt(&saved)]), "Task updated"))
        }
        Command::Delete { id, revision } => {
            remove(host, id, revision, false)?;
            Ok(outcome(json!([]), "Task deleted"))
        }
    }
}
fn remove(host: &mut impl Host, id: i64, revision: i64, monitor: bool) -> Result<(), String> {
    let old = task(host, id)?;
    if old.revision != revision || old.value.monitor.is_some() != monitor {
        return Err("Task revision or type changed".into());
    }
    let mut value = old.value.clone();
    value.draft.enabled = false;
    value.next_run = None;
    value.state.generation = value
        .state
        .generation
        .checked_add(1)
        .ok_or("Task generation exhausted")?;
    value.state.pending.clear();
    let invalidated = save(host, Some(&old), value)?;
    cancel_work(host, id)?;
    collection(
        host,
        "tasks",
        json!({"action":"delete","id":id,"revision":invalidated.revision}),
    )?;
    Ok(())
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Due {
    pub task_id: i64,
    pub generation: u64,
    pub due_at: i64,
}
fn record_history(host: &mut impl Host, task: i64, due: i64, run: &Value) -> Result<(), String> {
    let id = run["id"].as_i64().ok_or("Invalid history run")?;
    let state = run["state"].as_str().ok_or("Invalid history state")?;
    let status = match state {
        "pending" => "queued",
        "leased" => "claimed",
        "completed" => "complete",
        other => other,
    };
    collection(
        host,
        "task_runs",
        json!({"action":"create","value":{
            "key":format!("run:{id}"),"task_id":task,"due_at":due,"run_id":id,
            "snapshot":{"status":status,"detail":run["detail"].as_str().unwrap_or(""),
                "session_id":run["session_id"],"message_id":run["message_id"],"permission_id":run["permission_id"]}
        }}),
    )?;
    Ok(())
}
/// Queue a raw prompt; record prerequisites close the read/edit/submit race.
pub fn due(host: &mut impl Host, input: Due) -> Result<Outcome, String> {
    let Some(old) = tasks(host)?
        .into_iter()
        .find(|task| task.id == input.task_id)
    else {
        return Ok(outcome(
            json!({"ignored":true}),
            "Deleted occurrence ignored",
        ));
    };
    if !old.value.draft.enabled
        || old.value.state.generation != input.generation
        || old.value.next_run != Some(input.due_at)
    {
        if old.value.draft.enabled && old.value.state.generation == input.generation {
            arm(host, &old)?;
        }
        return Ok(outcome(json!({"ignored":true}), "Stale occurrence ignored"));
    }
    let now = clock(host)?;
    if now < input.due_at {
        return Err("Task occurrence is not due yet".into());
    }
    if let Some(monitor) = &old.value.monitor
        && (monitor.remaining <= 0 || monitor.expires_at <= now)
    {
        let mut stopped = old.value.clone();
        stopped.draft.enabled = false;
        stopped.next_run = None;
        save(host, Some(&old), stopped)?;
        return Ok(outcome(json!({"expired":true}), "Monitor expired"));
    }
    let metadata = all::<Conversation>(host, "conversations")?;
    let configuration = collection(host, "configuration", json!({"action":"read","id":1}))?;
    let config: Configuration =
        serde_json::from_value(configuration["records"][0]["value"].clone())
            .map_err(|error| error.to_string())?;
    let default = config.default_model().ok();
    let selection = conversations::select(&old.value.draft, &metadata, default.as_ref())?;
    let key = format!("task:{}:{}:{}", old.id, input.generation, input.due_at);
    let response=host.call("runs",json!({"action":"submit","key":key,"prompt":old.value.draft.prompt,
        "target":selection.target,"model":selection.model,
        "prerequisites":[{"capability":"collections","collection":"tasks","id":old.id,"revision":old.revision}],
        "completion":{"event":"task_completed","payload":{"task_id":old.id,"generation":input.generation,"due_at":input.due_at}}}))?;
    let run = &response["runs"][0];
    let id = run["id"].as_i64().ok_or("No queued run returned")?;
    record_history(host, old.id, input.due_at, run)?;
    let mut value = old.value.clone();
    if !value.state.pending.iter().any(|run| run.id == id) {
        value.state.pending.push(Pending {
            id,
            key,
            due_at: input.due_at,
        });
    }
    value.next_run = if value.monitor.is_some() {
        None
    } else {
        value.draft.schedule.next_after(now)?
    };
    let advanced = save(host, Some(&old), value)?;
    arm(host, &advanced)?;
    Ok(outcome(response, "Scheduled prompt queued"))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Completed {
    pub run: Value,
    pub data: Due,
}
pub fn completed(host: &mut impl Host, input: Completed) -> Result<Outcome, String> {
    let Some(old) = tasks(host)?
        .into_iter()
        .find(|task| task.id == input.data.task_id)
    else {
        return Ok(outcome(json!({"ignored":true}), "Deleted result ignored"));
    };
    let expected_key = format!(
        "task:{}:{}:{}",
        old.id, input.data.generation, input.data.due_at
    );
    if input.run["key"].as_str() != Some(expected_key.as_str()) {
        return Err("Run completion key does not match occurrence".into());
    }
    let state = input.run["state"]
        .as_str()
        .ok_or("Invalid completion state")?;
    if !matches!(state, "completed" | "failed" | "cancelled" | "interrupted") {
        return Err("Completion run is not terminal".into());
    }
    record_history(host, old.id, input.data.due_at, &input.run)?;
    if !old.value.draft.enabled || old.value.state.generation != input.data.generation {
        return Ok(outcome(json!({"ignored":true}), "Stale result ignored"));
    }
    let run_id = input.run["id"].as_i64().ok_or("Invalid completion run")?;
    if old
        .value
        .state
        .last_result
        .as_ref()
        .is_some_and(|last| last["id"].as_i64() == Some(run_id))
    {
        arm(host, &old)?;
        return Ok(outcome(json!({"ignored":true}), "Result already recorded"));
    }
    if !old.value.state.pending.iter().any(|run| run.id == run_id)
        && old.value.next_run != Some(input.data.due_at)
    {
        arm(host, &old)?;
        return Ok(outcome(json!({"ignored":true}), "Prior result ignored"));
    }
    let now = clock(host)?;
    let mut value = old.value.clone();
    value.state.pending.retain(|run| run.id != run_id);
    if !value
        .state
        .last_result
        .as_ref()
        .is_some_and(|prior| prior["id"].as_i64().is_some_and(|id| id > run_id))
    {
        value.state.last_result = Some(input.run.clone());
    }
    if let Some(monitor) = &mut value.monitor {
        monitor.remaining = monitor.remaining.saturating_sub(1);
        let next = now
            .checked_add(monitor.interval_seconds)
            .ok_or("Invalid monitor timestamp")?;
        if state == "completed" && monitor.remaining > 0 && next < monitor.expires_at {
            value.next_run = Some(next);
        } else {
            value.next_run = None;
            value.draft.enabled = false;
        }
    } else if value.next_run == Some(input.data.due_at) {
        // Completion can repair a crash after submit but before occurrence advancement.
        value.next_run = value.draft.schedule.next_after(now)?;
    }
    let advanced = save(host, Some(&old), value)?;
    arm(host, &advanced)?;
    Ok(outcome(json!({"recorded":true}), "Task result recorded"))
}
