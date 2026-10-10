use openwebide_plugin_sdk::serde_json::{self, Value, json};
use scheduling_plugin::{
    schedule::{Draft, Schedule, SessionTarget},
    service::{self, Completed, Due, Host},
};
use std::collections::BTreeMap;
#[derive(Default)]
struct Fixture {
    now: i64,
    tasks: BTreeMap<i64, Value>,
    jobs: BTreeMap<i64, Value>,
    runs: BTreeMap<i64, Value>,
    job_id: i64,
    run_id: i64,
    calls: Vec<(String, Value)>,
    config: Value,
    conversations: Vec<Value>,
    fail_arm: bool,
    fail_update: Option<i64>,
    fail_history: bool,
    fail_history_task: Option<i64>,
    enforce_history_limit: bool,
    history: BTreeMap<String, Value>,
    history_id: i64,
    recovery: Option<Value>,
    fail_recovery_write: bool,
}
impl Fixture {
    fn new() -> Self {
        Self {
            now: 100,
            config: json!({"primary":{"server_id":1,"model":"current default"},"default_connection":null,"servers":[{"id":1,"model":"server model","enabled":true}]}),
            conversations: vec![
                json!({"id":1,"value":{"origin":true,"archived":false,"last_activity":90,"model_override":{"connection_id":1,"model":"session model"},"connection":null}}),
            ],
            ..Self::default()
        }
    }
    fn create(&mut self, draft: Draft) -> Value {
        let result = service::execute(self, "schedule_create", json!({"draft":draft})).unwrap();
        serde_json::from_str::<Value>(&result.content).unwrap()[0].clone()
    }
    fn run(&self) -> Value {
        self.runs.values().last().unwrap().clone()
    }
    fn next(&self) -> Option<i64> {
        self.tasks[&1]["value"]["next_run"].as_i64()
    }
    fn finish(&mut self, run: Value) -> service::Completed {
        Completed {
            data: serde_json::from_value(run["request"]["completion"]["payload"].clone()).unwrap(),
            run,
        }
    }
}
impl Host for Fixture {
    fn call(&mut self, capability: &str, input: Value) -> Result<Value, String> {
        self.calls.push((capability.into(), input.clone()));
        match capability {
            "clock" => Ok(json!({"unix_seconds":self.now})),
            "completion" => Err("Model is unavailable".into()),
            "records" => {
                assert_eq!(input["collection"], "recovery");
                let op = &input["operation"];
                if op["action"] != "list" && self.fail_recovery_write {
                    return Err("Recovery cursor write unavailable".into());
                }
                match op["action"].as_str().unwrap() {
                    "list" => (),
                    "create" => {
                        self.recovery = Some(json!({"id":1,"revision":1,"value":op["value"]}))
                    }
                    "update" => {
                        let old = self.recovery.as_ref().ok_or("Missing recovery record")?;
                        if old["revision"] != op["revision"] {
                            return Err("Recovery conflict".into());
                        }
                        self.recovery = Some(
                            json!({"id":1,"revision":old["revision"].as_i64().unwrap()+1,"value":op["value"]}),
                        );
                    }
                    _ => panic!("Unexpected recovery operation"),
                }
                Ok(json!({"records":self.recovery.iter().collect::<Vec<_>>(),"next":null}))
            }
            "collections" => {
                let operation = &input["operation"];
                let action = operation["action"].as_str().unwrap();
                match input["collection"].as_str().unwrap() {
                    "configuration" => Ok(
                        json!({"records":[{"id":1,"revision":1,"value":self.config}],"next":null}),
                    ),
                    "conversations" => Ok(json!({"records":self.conversations,"next":null})),
                    "task_runs" => {
                        if action == "list" {
                            let after = operation["after"].as_i64().unwrap_or(0);
                            let mut records: Vec<_> = self
                                .history
                                .values()
                                .filter(|row| row["id"].as_i64().unwrap() > after)
                                .cloned()
                                .collect();
                            records.sort_by_key(|row| row["id"].as_i64().unwrap());
                            let next = (records.len() > 32).then(|| records[31]["id"].clone());
                            records.truncate(32);
                            return Ok(json!({"records":records,"next":next}));
                        }
                        if action == "delete" {
                            let key = self
                                .history
                                .iter()
                                .find(|(_, row)| row["id"] == operation["id"])
                                .map(|(key, _)| key.clone())
                                .ok_or("Missing history")?;
                            let row = &self.history[&key];
                            if row["revision"] != operation["revision"]
                                || row["value"]["editable"] != true
                                || !matches!(
                                    row["value"]["snapshot"]["status"].as_str(),
                                    Some(
                                        "complete"
                                            | "failed"
                                            | "cancelled"
                                            | "expired"
                                            | "interrupted"
                                    )
                                )
                            {
                                return Err("History cannot be deleted".into());
                            }
                            self.history.remove(&key);
                            return Ok(json!({"records":[],"next":null}));
                        }
                        assert_eq!(action, "create");
                        if self.fail_history
                            || self.fail_history_task.is_some_and(|task| {
                                operation["value"]["task_id"].as_i64() == Some(task)
                            })
                        {
                            return Err("History write unavailable".into());
                        }
                        let mut value = operation["value"].clone();
                        value["editable"] = json!(true);
                        let key = value["key"].as_str().unwrap().to_owned();
                        if self.enforce_history_limit
                            && self.history.len() >= 1000
                            && !self.history.contains_key(&key)
                        {
                            return Err(
                                "Task history holds at most 1,000 entries in this scope".into()
                            );
                        }
                        if !self.history.contains_key(&key) {
                            self.history_id += 1;
                        }
                        let index = self.history_id;
                        let record = self
                            .history
                            .entry(key)
                            .or_insert_with(|| json!({"id":index,"revision":1,"value":value}));
                        Ok(json!({"records":[record],"next":null}))
                    }
                    "tasks" => {
                        let id = operation["id"].as_i64().unwrap_or(0);
                        let mut selected = None;
                        match action {
                            "create" => {
                                let id = self.tasks.keys().last().copied().unwrap_or(0) + 1;
                                self.tasks.insert(
                                    id,
                                    json!({"id":id,"revision":1,"value":operation["value"]}),
                                );
                                selected = Some(id);
                            }
                            "update" | "delete" => {
                                if action == "update" && self.fail_update == Some(id) {
                                    return Err("Revision conflict".into());
                                }
                                let current = self.tasks.get(&id).ok_or("Missing task")?;
                                if current["revision"] != operation["revision"] {
                                    return Err("Revision conflict".into());
                                }
                                let revision = current["revision"].as_i64().unwrap() + 1;
                                if action == "delete" {
                                    self.tasks.remove(&id);
                                } else {
                                    self.tasks.insert(id,json!({"id":id,"revision":revision,"value":operation["value"]}));
                                    selected = Some(id);
                                }
                            }
                            "read" => selected = Some(id),
                            "list" => (),
                            _ => panic!("Unsupported operation"),
                        }
                        let records = if let Some(id) = selected {
                            vec![self.tasks.get(&id).ok_or("Missing task")?.clone()]
                        } else if action == "delete" {
                            vec![]
                        } else {
                            self.tasks.values().cloned().collect()
                        };
                        Ok(json!({"records":records,"next":null}))
                    }
                    _ => panic!("Unexpected collection"),
                }
            }
            "jobs" | "runs" => {
                let action = input["action"].as_str().unwrap();
                let field = if capability == "jobs" { "jobs" } else { "runs" };
                if action == "submit" {
                    for guard in input["prerequisites"].as_array().unwrap() {
                        if self.tasks[&guard["id"].as_i64().unwrap()]["revision"]
                            != guard["revision"]
                        {
                            return Err("Stale prerequisite".into());
                        }
                    }
                }
                let records = if capability == "jobs" {
                    &mut self.jobs
                } else {
                    &mut self.runs
                };
                match action {
                    "schedule" | "submit" => {
                        if capability == "jobs" && self.fail_arm {
                            return Err("Job queue unavailable".into());
                        }
                        if let Some(existing) = records
                            .values()
                            .find(|record| record["key"] == input["key"])
                        {
                            return Ok(json!({field:[existing],"next_after":null}));
                        }
                        let counter = if capability == "jobs" {
                            &mut self.job_id
                        } else {
                            &mut self.run_id
                        };
                        *counter =
                            (*counter + 1).max(records.keys().last().copied().unwrap_or(0) + 1);
                        let id = *counter;
                        let record = json!({"id":id,"revision":1,"key":input["key"],"state":"pending",
                            "event":input["event"],"payload":input["payload"],"request":input});
                        records.insert(id, record.clone());
                        Ok(json!({field:[record],"next_after":null}))
                    }
                    "list" => {
                        Ok(json!({field:records.values().collect::<Vec<_>>(),"next_after":null}))
                    }
                    "delete" => {
                        let id = input["id"].as_i64().unwrap();
                        let record = records.get(&id).ok_or("Missing work")?;
                        if record["revision"] != input["revision"] {
                            return Err("Revision conflict".into());
                        }
                        records.remove(&id);
                        Ok(json!({field:[],"next_after":null}))
                    }
                    "cancel" => {
                        let record = records
                            .get_mut(&input["id"].as_i64().unwrap())
                            .ok_or("Missing work")?;
                        record["state"] = json!("cancelled");
                        record["revision"] = json!(record["revision"].as_i64().unwrap() + 1);
                        Ok(json!({field:[record],"next_after":null}))
                    }
                    _ => panic!("Unsupported work operation"),
                }
            }
            _ => panic!("Unexpected host capability: {capability}"),
        }
    }
}
fn draft(schedule: Schedule, target: SessionTarget) -> Draft {
    Draft {
        session_target: target,
        session_id: 0,
        title: String::new(),
        auto_title: true,
        prompt: "Check the latest build".into(),
        schedule,
        enabled: true,
        model: None,
    }
}
#[test]
fn enablement_preserves_titles_and_rejects_stale_and_elapsed_once_resumes() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    let title = host.tasks[&1]["value"]["draft"]["title"].clone();
    host.calls.clear();
    service::execute(
        &mut host,
        "schedule_set_enabled",
        json!({"id":1,"revision":1,"enabled":false}),
    )
    .unwrap();
    assert_eq!(host.tasks[&1]["value"]["draft"]["title"], title);
    assert_eq!(host.tasks[&1]["value"]["draft"]["auto_title"], true);
    assert_eq!(host.next(), None);
    assert!(
        !host
            .calls
            .iter()
            .any(|(capability, _)| capability == "completion")
    );
    assert!(
        service::execute(
            &mut host,
            "schedule_set_enabled",
            json!({"id":1,"revision":1,"enabled":true})
        )
        .is_err()
    );
    service::execute(
        &mut host,
        "schedule_set_enabled",
        json!({"id":1,"revision":2,"enabled":true}),
    )
    .unwrap();
    assert_eq!(host.next(), Some(105));
    host.now = 110;
    service::execute(
        &mut host,
        "schedule_set_enabled",
        json!({"id":1,"revision":3,"enabled":false}),
    )
    .unwrap();
    assert!(
        service::execute(
            &mut host,
            "schedule_set_enabled",
            json!({"id":1,"revision":4,"enabled":true})
        )
        .is_err()
    );
    assert_eq!(host.tasks[&1]["revision"], 4);
}

#[test]
fn monitor_cancel_accepts_ui_session_context_and_rejects_another_conversation() {
    let mut host = Fixture::new();
    service::execute(
        &mut host,
        "monitor",
        json!({"action":"start","prompt":"Check","delay_seconds":5,"session_id":1}),
    )
    .unwrap();
    assert!(
        service::execute(
            &mut host,
            "monitor",
            json!({"action":"cancel","id":1,"revision":1,"session_id":2})
        )
        .is_err()
    );
    assert!(host.tasks.contains_key(&1));
    service::execute(
        &mut host,
        "monitor",
        json!({"action":"cancel","id":1,"revision":1,"session_id":1}),
    )
    .unwrap();
    assert!(host.tasks.is_empty());
}

#[test]
fn reconciliation_bounds_host_calls_and_rotates_past_permanent_failures() {
    let mut host = Fixture::new();
    for id in 1..=20 {
        host.tasks.insert(
            id,
            json!({"id":id,"revision":1,"value":{
                "draft":draft(Schedule::Once {at:105},SessionTarget::Existing),
                "next_run":105,"monitor":null,"state":null
            }}),
        );
    }
    host.fail_update = Some(1);
    for _ in 0..3 {
        let before = host.calls.len();
        service::reconcile(&mut host).unwrap();
        assert!(host.calls.len() - before <= 128);
    }
    assert_eq!(host.jobs.len(), 19);
    assert!(host.tasks[&1]["value"]["state"].is_null());
    host.fail_update = None;
    for _ in 0..3 {
        let before = host.calls.len();
        assert!(service::reconcile(&mut host).unwrap().ok);
        assert!(host.calls.len() - before <= 128);
    }
    assert_eq!(host.jobs.len(), 20);
}

#[test]
fn exhausted_recovery_budget_retains_cursor_and_cancellation_makes_progress() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.tasks.get_mut(&1).unwrap()["value"]["draft"]["enabled"] = json!(false);
    host.tasks.get_mut(&1).unwrap()["value"]["state"]["generation"] = json!(2);
    for id in 2..=250 {
        host.jobs.insert(
            id,
            json!({"id":id,"revision":1,"state":"pending","key":format!("task:1:1:{id}")}),
        );
    }
    for _ in 0..3 {
        let before = host.calls.len();
        service::reconcile(&mut host).unwrap();
        assert!(host.calls.len() - before <= 128);
    }
    assert!(host.jobs.values().all(|job| job["state"] == "cancelled"));
    assert_eq!(host.recovery.as_ref().unwrap()["value"]["after"], 1);
}

#[test]
fn reconciliation_retries_each_record_independently_and_finishes_interrupted_deletion() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.create(draft(Schedule::Once { at: 110 }, SessionTarget::Existing));
    for row in host.tasks.values_mut() {
        row["value"]["state"] = Value::Null;
    }
    host.fail_update = Some(1);
    assert!(!service::reconcile(&mut host).unwrap().ok);
    assert!(host.tasks[&1]["value"]["state"].is_null());
    assert_eq!(
        host.tasks[&2]["value"]["state"]["source_version"],
        env!("CARGO_PKG_VERSION")
    );
    host.fail_update = None;
    assert!(service::reconcile(&mut host).unwrap().ok);
    // Actor dropped after disabling the task and persisting deletion intent.
    host.tasks.get_mut(&1).unwrap()["value"]["draft"]["enabled"] = json!(false);
    host.tasks.get_mut(&1).unwrap()["value"]["state"]["deleting"] = json!(true);
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert!(!host.tasks.contains_key(&1));
    assert!(host.tasks.contains_key(&2));
}

#[test]
fn reconciliation_expires_monitors_and_updated_checks_keep_the_original_budget() {
    let mut host = Fixture::new();
    service::execute(&mut host,"monitor",json!({"action":"start","prompt":"Check","delay_seconds":5,"interval_seconds":10,"max_checks":2})).unwrap();
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    let expiry = host.tasks[&1]["value"]["monitor"]["expires_at"].clone();
    host.tasks.get_mut(&1).unwrap()["value"]["state"]["source_version"] = json!("previous");
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.tasks[&1]["value"]["monitor"]["remaining"], 1);
    assert_eq!(host.tasks[&1]["value"]["monitor"]["expires_at"], expiry);
    assert_eq!(host.next(), Some(115));
    host.now = expiry.as_i64().unwrap();
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.tasks[&1]["value"]["draft"]["enabled"], false);
    assert_eq!(host.next(), None);
    assert!(host.runs.is_empty());
    assert!(host.history.contains_key("run:1"));
    assert_eq!(
        host.calls
            .iter()
            .filter(|(capability, request)| capability == "runs" && request["action"] == "submit")
            .count(),
        1
    );
}

#[test]
fn reconciliation_rearms_orphaned_overdue_and_failed_occurrences_without_duplicate_runs() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    // Actor dropped after saving the task but before arming its timer.
    host.jobs.clear();
    host.now = 110;
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.jobs.len(), 1);
    let job = host.jobs.values().next().unwrap();
    let job_id = job["id"].as_i64().unwrap();
    assert_eq!(job["request"]["due_at"], 105);
    // A terminal failed due handler is retried by source rather than stuck under its key.
    host.jobs.get_mut(&job_id).unwrap()["state"] = json!("failed");
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.jobs.len(), 1);
    assert_eq!(host.jobs.values().next().unwrap()["state"], "pending");
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    // Actor dropped after submitting its run, before advancing the task or recording history.
    host.tasks.get_mut(&1).unwrap()["value"]["next_run"] = json!(105);
    host.tasks.get_mut(&1).unwrap()["value"]["state"]["pending"] = json!([]);
    host.history.clear();
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.next(), None);
    assert_eq!(host.history.len(), 1);
    assert_eq!(
        host.tasks[&1]["value"]["state"]["pending"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.runs.len(), 1);
}

#[test]
fn reconciliation_adopts_legacy_records_and_updates_generation_before_old_work_is_cancelled() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.tasks.get_mut(&1).unwrap()["value"]["state"] = Value::Null;
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.tasks[&1]["value"]["state"]["generation"], 1);
    assert_eq!(
        host.tasks[&1]["value"]["state"]["source_version"],
        env!("CARGO_PKG_VERSION")
    );
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    host.tasks.get_mut(&1).unwrap()["value"]["next_run"] = json!(120);
    host.tasks.get_mut(&1).unwrap()["value"]["state"]["source_version"] = json!("previous source");
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.tasks[&1]["value"]["state"]["generation"], 2);
    assert_eq!(host.runs[&1]["state"], "cancelled");
    assert!(
        host.jobs
            .values()
            .any(|job| job["key"] == "task:1:2:120" && job["state"] == "pending")
    );
    let generations = host
        .calls
        .iter()
        .filter(|(capability, request)| {
            capability == "collections" && request["operation"]["action"] == "update"
        })
        .count();
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.tasks[&1]["value"]["state"]["generation"], 2);
    assert_eq!(
        host.calls
            .iter()
            .filter(|(capability, request)| capability == "collections"
                && request["operation"]["action"] == "update")
            .count(),
        generations
    );
}

#[test]
fn reconciliation_recovers_terminal_callbacks_idempotently() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    host.runs.get_mut(&1).unwrap()["state"] = json!("completed");
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.tasks[&1]["value"]["state"]["last_result"]["id"], 1);
    let revision = host.tasks[&1]["revision"].clone();
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.tasks[&1]["revision"], revision);
}

#[test]
fn reconciliation_recovers_failed_retained_callbacks_after_raw_cleanup() {
    for state in ["completed", "failed", "cancelled", "expired"] {
        let mut host = Fixture::new();
        host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
        host.now = 105;
        service::due(
            &mut host,
            Due {
                task_id: 1,
                generation: 1,
                due_at: 105,
            },
        )
        .unwrap();
        let mut run = host.run();
        run["state"] = json!("completed");
        let data = run["request"]["completion"]["payload"].clone();
        host.jobs.insert(
            2,
            json!({"id":2,"revision":3,"key":"$run:1","event":"task_completed",
            "state":state,"payload":{"run":run,"data":data}}),
        );
        host.runs.clear();
        host.history.clear(); // A drop before recording history must also be repairable.
        host.fail_update = Some(1);
        assert!(!service::reconcile(&mut host).unwrap().ok);
        assert!(
            host.jobs.contains_key(&2),
            "Keep a callback until its effects are durable"
        );
        assert_eq!(
            host.tasks[&1]["value"]["state"]["pending"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        host.fail_update = None;
        assert!(service::reconcile(&mut host).unwrap().ok);
        assert!(!host.jobs.contains_key(&2));
        assert_eq!(
            host.tasks[&1]["value"]["state"]["last_result"]["state"],
            "completed"
        );
        assert!(
            host.tasks[&1]["value"]["state"]["pending"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            host.history["run:1"]["value"]["snapshot"]["status"],
            "complete"
        );
        let revision = host.tasks[&1]["revision"].clone();
        assert!(service::reconcile(&mut host).unwrap().ok);
        assert_eq!(host.tasks[&1]["revision"], revision);
    }
}

#[test]
fn retained_callback_recovery_ignores_live_unreleased_and_other_task_jobs() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    for (id, state, key, task) in [
        (2, "pending", "$run:1", 1),
        (3, "leased", "$run:1", 1),
        (4, "waiting", "$run:1", 1),
        (5, "failed", "ordinary-job", 1),
        (6, "failed", "$run:1", 99),
    ] {
        host.jobs.insert(
            id,
            json!({"id":id,"revision":1,"key":key,"event":"task_completed",
            "state":state,"payload":{"data":{"task_id":task}}}),
        );
    }
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert!((2..=6).all(|id| host.jobs.contains_key(&id)));
}

#[test]
fn full_history_recovers_capacity_before_terminal_run_deletion() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 200 }, SessionTarget::Existing));
    for id in 1..=1000 {
        host.call(
            "collections",
            json!({"collection":"task_runs","operation":{"action":"create",
            "value":{"key":format!("old:{id}"),"task_id":1,"due_at":90,
                "snapshot":{"status":"complete"}}}}),
        )
        .unwrap();
    }
    host.enforce_history_limit = true;
    host.runs.insert(
        1,
        json!({"id":1,"revision":1,"key":"task:1:1:100","state":"completed"}),
    );
    for _ in 0..4 {
        host.calls.clear();
        assert!(!service::reconcile(&mut host).unwrap().ok);
        assert!(host.runs.contains_key(&1));
        assert_eq!(host.history.len(), 1000);
        assert!(host.calls.len() <= 114);
    }
    host.calls.clear();
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert!(!host.runs.contains_key(&1));
    assert!(host.history.contains_key("run:1"));
    assert!(host.history.len() < 1000);
    assert!(host.calls.len() <= 114);
}

#[test]
fn terminal_work_cleanup_preserves_history_and_ignores_active_and_unrelated_work() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    host.runs.get_mut(&1).unwrap()["state"] = json!("completed");
    let run = host.run();
    let data = run["request"]["completion"]["payload"].clone();
    let finished = host.finish(run.clone());
    service::completed(&mut host, finished).unwrap();
    host.history.clear(); // Cleanup must recreate an explicitly removed terminal binding.
    host.jobs.get_mut(&1).unwrap()["state"] = json!("completed");
    host.jobs.insert(
        2,
        json!({"id":2,"revision":1,"key":"$run:1","event":"task_completed",
        "state":"completed","payload":{"run":run,"data":data}}),
    );
    host.runs.insert(
        2,
        json!({"id":2,"revision":1,"key":"task:1:1:200","state":"running"}),
    );
    host.runs.insert(
        3,
        json!({"id":3,"revision":1,"key":"manual","state":"completed"}),
    );
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert!(!host.runs.contains_key(&1));
    assert!(host.runs.contains_key(&2));
    assert!(host.runs.contains_key(&3));
    assert!(!host.jobs.contains_key(&1));
    assert!(!host.jobs.contains_key(&2));
    assert_eq!(
        host.history["run:1"]["value"]["snapshot"]["status"],
        "complete"
    );
    assert_eq!(host.tasks[&1]["value"]["state"]["last_result"]["id"], 1);
}

#[test]
fn terminal_cleanup_keeps_runs_when_history_fails_and_rotates_past_blocked_tasks() {
    let mut host = Fixture::new();
    for _ in 0..2 {
        host.create(draft(Schedule::Once { at: 200 }, SessionTarget::Existing));
    }
    for (id, task) in [(1, 1), (2, 1), (3, 2)] {
        host.runs.insert(
            id,
            json!({"id":id,"revision":1,"key":format!("task:{task}:1:100"),"state":"completed"}),
        );
    }
    host.fail_history_task = Some(1);
    assert!(!service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.runs.len(), 3);
    assert_eq!(
        host.recovery.as_ref().unwrap()["value"]["cleanup"]["runs"],
        2
    );
    assert!(!service::reconcile(&mut host).unwrap().ok);
    assert!(!host.runs.contains_key(&3));
    assert!(host.runs.contains_key(&1));
    assert!(host.runs.contains_key(&2));
    assert!(host.history.contains_key("run:3"));
    host.fail_history_task = None;
    for _ in 0..3 {
        assert!(service::reconcile(&mut host).unwrap().ok);
    }
    assert!(host.runs.is_empty());
    assert_eq!(host.history.len(), 3);
}

#[test]
fn orphaned_terminal_jobs_are_removed_without_touching_unknown_or_malformed_events() {
    let mut host = Fixture::new();
    host.jobs.insert(
        1,
        json!({"id":1,"revision":1,"key":"task:99:1:100","event":"task_due","state":"completed"}),
    );
    host.jobs.insert(2, json!({"id":2,"revision":1,"key":"$run:77","event":"task_completed","state":"failed",
        "payload":{"run":{"id":77,"key":"task:99:1:100","state":"cancelled"},"data":{"task_id":99,"generation":1,"due_at":100}}}));
    host.jobs.insert(3, json!({"id":3,"revision":1,"key":"task:99:1:100","event":"other_event","state":"completed"}));
    host.jobs.insert(4, json!({"id":4,"revision":1,"key":"$run:78","event":"task_completed","state":"completed",
        "payload":{"run":{"id":999,"key":"task:99:1:100","state":"completed"},"data":{"task_id":99,"generation":1,"due_at":100}}}));
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert!(!host.jobs.contains_key(&1));
    assert!(!host.jobs.contains_key(&2));
    assert!(host.jobs.contains_key(&3));
    assert!(host.jobs.contains_key(&4));
}

#[test]
fn cancelled_waiting_callbacks_survive_until_terminal_run_bookkeeping_is_durable() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    host.jobs.insert(
        2,
        json!({"id":2,"revision":1,"key":"$run:1","event":"task_completed","state":"cancelled",
        "payload":{"task_id":1,"generation":1,"due_at":105}}),
    );
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert!(host.jobs.contains_key(&2));
    assert!(host.runs.contains_key(&1));
    host.runs.get_mut(&1).unwrap()["state"] = json!("cancelled");
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert!(host.jobs.contains_key(&2));
    assert!(host.runs.contains_key(&1));
    assert!(
        host.tasks[&1]["value"]["state"]["pending"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert!(!host.jobs.contains_key(&2));
    assert!(!host.runs.contains_key(&1));
    assert!(host.history.contains_key("run:1"));
    assert_eq!(
        host.tasks[&1]["value"]["state"]["last_result"]["state"],
        "cancelled"
    );
}

#[test]
fn history_retention_keeps_latest_active_and_legacy_entries_and_survives_external_deletion() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 200 }, SessionTarget::Existing));
    host.create(draft(Schedule::Once { at: 200 }, SessionTarget::Existing));
    host.tasks.get_mut(&1).unwrap()["value"]["state"]["last_result"] = json!({"id":179});
    host.tasks.get_mut(&2).unwrap()["value"]["state"]["last_result"] =
        json!({"key":"preflight:2:1:100"});
    for id in 1..=180 {
        let key = if id == 180 {
            "preflight:2:1:100".to_owned()
        } else {
            format!("run:{id}")
        };
        let status = match id {
            1 => "queued",
            2 => "running",
            _ => "complete",
        };
        host.call(
            "collections",
            json!({"collection":"task_runs","operation":{"action":"create",
            "value":{"key":key,"task_id":if id == 180 {2} else {1},"due_at":100,
                "run_id":if id == 180 {None} else {Some(id)},"snapshot":{"status":status}}}}),
        )
        .unwrap();
    }
    host.history.get_mut("run:3").unwrap()["value"]["editable"] = json!(false);
    for _ in 0..100 {
        host.calls.clear();
        assert!(service::reconcile(&mut host).unwrap().ok);
        assert!(host.calls.len() <= 114);
    }
    assert_eq!(host.history.len(), 133);
    assert!(host.history.contains_key("run:1"));
    assert!(host.history.contains_key("run:2"));
    assert!(host.history.contains_key("run:3"));
    assert!(host.history.contains_key("run:179"));
    assert!(host.history.contains_key("preflight:2:1:100"));
    assert!(!host.history.contains_key("run:50"));
    assert!(host.history.contains_key("run:51"));
    // A deleted buffered row must not permanently stop retention on its stale ID.
    host.history.remove("run:51");
    host.call("collections", json!({"collection":"task_runs","operation":{"action":"create",
        "value":{"key":"run:181","task_id":1,"due_at":100,"run_id":181,"snapshot":{"status":"failed"}}}})).unwrap();
    for _ in 0..20 {
        assert!(service::reconcile(&mut host).unwrap().ok);
    }
    assert_eq!(host.history.len(), 133);
    assert!(host.history.contains_key("run:181"));
    assert!(host.history.contains_key("run:52"));
}

#[test]
fn history_retention_recovers_a_drop_after_deletion_before_cursor_save() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 200 }, SessionTarget::Existing));
    for id in 1..=130 {
        host.call(
            "collections",
            json!({"collection":"task_runs","operation":{"action":"create",
            "value":{"key":format!("run:{id}"),"task_id":1,"due_at":100,"run_id":id,
                "snapshot":{"status":"complete"}}}}),
        )
        .unwrap();
    }
    host.recovery = Some(json!({"id":1,"revision":1,"value":{"after":0,
        "history":{"after":128,"recent":(1..=128).collect::<Vec<_>>()}}}));
    host.fail_recovery_write = true;
    assert!(service::reconcile(&mut host).is_err());
    assert_eq!(host.history.len(), 128);
    assert_eq!(
        host.recovery.as_ref().unwrap()["value"]["history"]["after"],
        128
    );
    host.fail_recovery_write = false;
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.history.len(), 128);
    assert!(host.history.contains_key("run:3"));
    assert!(host.history.contains_key("run:130"));
    assert_eq!(
        host.recovery.as_ref().unwrap()["value"]["history"]["recent"][0],
        3
    );
}

#[test]
fn preflight_failure_advances_once_and_records_visible_history() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.conversations.clear();
    host.now = 105;
    let input = || Due {
        task_id: 1,
        generation: 1,
        due_at: 105,
    };
    assert!(service::due(&mut host, input()).unwrap().ok);
    assert!(host.runs.is_empty());
    assert_eq!(host.next(), None);
    let failure = &host.history["preflight:1:1:105"]["value"];
    assert_eq!(failure["snapshot"]["status"], "failed");
    assert!(
        failure["snapshot"]["detail"]
            .as_str()
            .unwrap()
            .contains("no longer exists")
    );
    assert!(failure["run_id"].is_null());
    let revision = host.tasks[&1]["revision"].clone();
    service::due(&mut host, input()).unwrap();
    assert_eq!(host.tasks[&1]["revision"], revision);
    assert_eq!(host.history.len(), 1);
}

#[test]
fn preflight_failure_repairs_history_after_drop_and_preserves_it_across_edits() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.conversations.clear();
    host.now = 105;
    host.fail_history = true;
    assert!(
        service::due(
            &mut host,
            Due {
                task_id: 1,
                generation: 1,
                due_at: 105
            }
        )
        .is_err()
    );
    assert!(host.history.is_empty());
    assert_eq!(host.next(), None);
    assert!(host.tasks[&1]["value"]["state"]["preflight"].is_object());
    host.conversations = Fixture::new().conversations;
    let revision = host.tasks[&1]["revision"].as_i64().unwrap();
    service::execute(
        &mut host,
        "schedule_update",
        json!({"id":1,"revision":revision,
        "draft":draft(Schedule::Once { at: 200 }, SessionTarget::Existing)}),
    )
    .unwrap();
    host.fail_history = false;
    assert!(service::reconcile(&mut host).unwrap().ok);
    assert_eq!(host.history["preflight:1:1:105"]["value"]["due_at"], 105);
    assert!(host.tasks[&1]["value"]["state"]["preflight"].is_null());
    assert_eq!(host.next(), Some(200));
    assert!(host.runs.is_empty());
}

#[test]
fn missing_model_advances_recurrence_and_stops_monitors() {
    let mut host = Fixture::new();
    host.create(draft(
        Schedule::Cron {
            expression: "* * * * *".into(),
            timezone: "UTC".into(),
        },
        SessionTarget::New,
    ));
    host.config = json!({"primary":null,"default_connection":null,"servers":[]});
    host.now = 120;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 120,
        },
    )
    .unwrap();
    assert_eq!(host.next(), Some(180));
    assert!(
        host.history["preflight:1:1:120"]["value"]["snapshot"]["detail"]
            .as_str()
            .unwrap()
            .contains("Configure a model")
    );

    let mut monitor = Fixture::new();
    service::execute(
        &mut monitor,
        "monitor",
        json!({"action":"start","session_id":1,
        "prompt":"Check build", "delay_seconds":5,"interval_seconds":30,"max_checks":3}),
    )
    .unwrap();
    monitor.conversations.clear();
    monitor.now = 105;
    service::due(
        &mut monitor,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    assert_eq!(monitor.next(), None);
    assert_eq!(monitor.tasks[&1]["value"]["draft"]["enabled"], false);
    assert_eq!(monitor.history.len(), 1);
}

#[test]
fn unavailable_explicit_model_records_failure_without_submitting_a_run() {
    let mut host = Fixture::new();
    let mut scheduled = draft(Schedule::Once { at: 105 }, SessionTarget::Existing);
    scheduled.model = Some(scheduling_plugin::schedule::ModelSelection {
        server_id: 1,
        model: "Pinned model".into(),
    });
    host.create(scheduled);
    host.config["servers"][0]["enabled"] = json!(false);
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    assert!(host.runs.is_empty());
    assert_eq!(host.next(), None);
    assert!(
        host.history["preflight:1:1:105"]["value"]["snapshot"]["detail"]
            .as_str()
            .unwrap()
            .contains("server is unavailable")
    );
    assert_eq!(
        host.tasks[&1]["value"]["draft"]["model"]["model"],
        "Pinned model"
    );
}

#[test]
fn preflight_failure_preserves_a_run_accepted_before_configuration_changed() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.now = 105;
    host.fail_update = Some(1);
    assert!(
        service::due(
            &mut host,
            Due {
                task_id: 1,
                generation: 1,
                due_at: 105
            }
        )
        .is_err()
    );
    assert_eq!(host.runs.len(), 1);
    host.fail_update = None;
    host.conversations.clear();
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    assert_eq!(host.runs.len(), 1);
    assert_eq!(host.history.len(), 1);
    assert!(host.history.contains_key("run:1"));
    assert_eq!(host.next(), None);
}

#[test]
fn create_owns_origin_title_due_job_and_rolls_back_if_arm_fails() {
    let mut host = Fixture::new();
    let created = host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    assert_eq!(created["title"], "Check the latest build");
    assert_eq!(created["session_id"], 1);
    assert_eq!(host.jobs[&1]["request"]["scope"], "project");
    assert_eq!(host.jobs[&1]["request"]["due_at"], 105);
    assert_eq!(host.jobs[&1]["request"]["event"], "task_due");
    let mut failed = Fixture::new();
    failed.fail_arm = true;
    assert!(
        service::execute(
            &mut failed,
            "schedule_create",
            json!({"draft":draft(Schedule::Once {at:105},SessionTarget::Existing)})
        )
        .is_err()
    );
    assert!(failed.tasks.is_empty());
}
#[test]
fn once_queues_with_revision_guard_and_current_session_model_then_ignores_replay() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    assert!(
        service::due(
            &mut host,
            Due {
                task_id: 1,
                generation: 1,
                due_at: 105
            }
        )
        .is_err()
    );
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    let run = host.run();
    assert_eq!(run["request"]["target"], json!({"kind":"session","id":1}));
    assert_eq!(
        run["request"]["model"],
        json!({"server_id":1,"model":"session model"})
    );
    assert_eq!(run["request"]["prerequisites"][0]["revision"], 1);
    assert_eq!(host.next(), None);
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    assert_eq!(host.runs.len(), 1);
    let mut done = run;
    done["state"] = json!("completed");
    let input = host.finish(done.clone());
    service::completed(&mut host, input).unwrap();
    let revision = host.tasks[&1]["revision"].clone();
    let input = host.finish(done);
    service::completed(&mut host, input).unwrap();
    assert_eq!(host.tasks[&1]["revision"], revision);
}
#[test]
fn cron_uses_latest_at_delivery_and_current_configuration_for_new_conversations() {
    let mut host = Fixture::new();
    host.create(draft(
        Schedule::Cron {
            expression: "* * * * *".into(),
            timezone: "UTC".into(),
        },
        SessionTarget::Latest,
    ));
    host.conversations.push(json!({"id":2,"value":{"origin":false,"archived":false,"last_activity":110,"model_override":null,"connection":{"id":1,"model":"new conversation model","enabled":true}}}));
    host.now = 120;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 120,
        },
    )
    .unwrap();
    assert_eq!(
        host.run()["request"]["target"],
        json!({"kind":"session","id":2})
    );
    assert_eq!(host.next(), Some(180));
    host.config["primary"]["model"] = json!("updated default");
    host.conversations
        .iter_mut()
        .for_each(|row| row["value"]["archived"] = json!(true));
    host.now = 180;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 180,
        },
    )
    .unwrap();
    assert_eq!(host.run()["request"]["target"]["kind"], "new");
    assert_eq!(host.run()["request"]["model"]["model"], "updated default");
    assert_eq!(host.next(), Some(240));
}
#[test]
fn update_invalidates_generation_before_cancelling_old_work_and_delete_removes_task() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    let revision = host.tasks[&1]["revision"].clone();
    service::execute(&mut host,"schedule_update",json!({"id":1,"revision":revision,"draft":draft(Schedule::Once {at:110},SessionTarget::Existing)})).unwrap();
    assert_eq!(host.tasks[&1]["value"]["state"]["generation"], 2);
    assert_eq!(host.jobs[&1]["state"], "cancelled");
    assert_eq!(host.runs[&1]["state"], "cancelled");
    let last_update = host
        .calls
        .iter()
        .position(|(cap, input)| {
            cap == "collections"
                && input["operation"]["action"] == "update"
                && input["operation"]["value"]["state"]["generation"] == 2
        })
        .unwrap();
    let first_cancel = host
        .calls
        .iter()
        .position(|(_, input)| input["action"] == "cancel")
        .unwrap();
    assert!(last_update < first_cancel);
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    assert_eq!(host.runs.len(), 1);
    let revision = host.tasks[&1]["revision"].clone();
    service::execute(
        &mut host,
        "schedule_delete",
        json!({"id":1,"revision":revision}),
    )
    .unwrap();
    assert!(host.tasks.is_empty());
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 2,
            due_at: 110,
        },
    )
    .unwrap();
    assert_eq!(host.runs.len(), 1);
}
#[test]
fn monitor_repeats_are_bounded_and_duplicate_completion_does_not_decrement_again() {
    let mut host = Fixture::new();
    service::execute(&mut host,"monitor",json!({"action":"start","prompt":"Check build","delay_seconds":5,"interval_seconds":10,"max_checks":2})).unwrap();
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    assert_eq!(host.next(), None);
    let mut first = host.run();
    first["state"] = json!("completed");
    let input = host.finish(first.clone());
    service::completed(&mut host, input).unwrap();
    assert_eq!(host.next(), Some(115));
    assert_eq!(host.tasks[&1]["value"]["monitor"]["remaining"], 1);
    let input = host.finish(first.clone());
    service::completed(&mut host, input).unwrap();
    assert_eq!(host.tasks[&1]["value"]["monitor"]["remaining"], 1);
    host.now = 115;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 115,
        },
    )
    .unwrap();
    let mut second = host.run();
    second["state"] = json!("completed");
    let input = host.finish(second);
    service::completed(&mut host, input).unwrap();
    assert_eq!(host.next(), None);
    assert_eq!(host.tasks[&1]["value"]["draft"]["enabled"], false);
    let input = host.finish(first);
    service::completed(&mut host, input).unwrap();
    assert_eq!(host.tasks[&1]["value"]["monitor"]["remaining"], 0);
}
#[test]
fn monitor_failure_stops_checks_and_expiry_never_submits_a_run() {
    for state in ["failed", "cancelled", "interrupted"] {
        let mut host = Fixture::new();
        service::execute(
            &mut host,
            "monitor",
            json!({"action":"start","prompt":"Check","delay_seconds":5,"max_checks":2}),
        )
        .unwrap();
        host.now = 105;
        service::due(
            &mut host,
            Due {
                task_id: 1,
                generation: 1,
                due_at: 105,
            },
        )
        .unwrap();
        let mut run = host.run();
        run["state"] = json!(state);
        let input = host.finish(run);
        service::completed(&mut host, input).unwrap();
        assert_eq!(host.next(), None);
        assert_eq!(host.tasks[&1]["value"]["draft"]["enabled"], false);
    }
    let mut host = Fixture::new();
    service::execute(
        &mut host,
        "monitor",
        json!({"action":"start","prompt":"Check","delay_seconds":5}),
    )
    .unwrap();
    host.now = 86500;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    assert!(host.runs.is_empty());
}
#[test]
fn completion_repairs_submission_before_advance_and_foreign_plugin_state_is_not_interpreted() {
    let mut host = Fixture::new();
    host.create(draft(Schedule::Once { at: 105 }, SessionTarget::Existing));
    host.tasks.insert(
        2,
        json!({"id":2,"revision":1,"value":{"editable":false,"state":{"pending":"foreign state"}}}),
    );
    let listed = service::execute(&mut host, "schedule_list", json!({})).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&listed.content)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    host.now = 105;
    let completed = Completed {
        run: json!({"id":3,"key":"task:1:1:105","state":"completed"}),
        data: Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    };
    service::completed(&mut host, completed).unwrap();
    assert_eq!(host.next(), None);
    assert_eq!(host.tasks[&1]["value"]["state"]["last_result"]["id"], 3);
}

#[test]
fn an_active_monitor_cannot_start_a_nested_monitor_and_disabled_past_tasks_can_be_edited() {
    let mut host = Fixture::new();
    service::execute(
        &mut host,
        "monitor",
        json!({"action":"start","prompt":"Check","delay_seconds":5}),
    )
    .unwrap();
    host.now = 105;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 105,
        },
    )
    .unwrap();
    host.runs.get_mut(&1).unwrap()["session_id"] = json!(1);
    host.runs.get_mut(&1).unwrap()["state"] = json!("running");
    assert!(
        service::execute(
            &mut host,
            "monitor",
            json!({"action":"start","prompt":"Extend indefinitely","delay_seconds":5})
        )
        .is_err()
    );
    assert_eq!(host.tasks.len(), 1);
    let mut disabled = draft(Schedule::Once { at: 1 }, SessionTarget::Existing);
    disabled.enabled = false;
    let created = host.create(disabled.clone());
    let id = created["id"].as_i64().unwrap();
    service::execute(
        &mut host,
        "schedule_update",
        json!({"id":id,"revision":1,"draft":disabled}),
    )
    .unwrap();
    assert_eq!(host.tasks[&id]["value"]["next_run"], json!(null));
}
#[test]
fn older_completed_occurrences_do_not_replace_a_newer_visible_result() {
    let mut host = Fixture::new();
    host.create(draft(
        Schedule::Cron {
            expression: "* * * * *".into(),
            timezone: "UTC".into(),
        },
        SessionTarget::Existing,
    ));
    host.now = 120;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 120,
        },
    )
    .unwrap();
    let mut first = host.run();
    first["state"] = json!("completed");
    host.now = 180;
    service::due(
        &mut host,
        Due {
            task_id: 1,
            generation: 1,
            due_at: 180,
        },
    )
    .unwrap();
    let mut second = host.run();
    second["state"] = json!("completed");
    let input = host.finish(second);
    service::completed(&mut host, input).unwrap();
    let input = host.finish(first);
    service::completed(&mut host, input).unwrap();
    assert_eq!(host.tasks[&1]["value"]["state"]["last_result"]["id"], 2);
    assert!(
        host.tasks[&1]["value"]["state"]["pending"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
