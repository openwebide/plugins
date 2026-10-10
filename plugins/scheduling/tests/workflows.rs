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
    calls: Vec<(String, Value)>,
    config: Value,
    conversations: Vec<Value>,
    fail_arm: bool,
    history: BTreeMap<String, Value>,
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
            "collections" => {
                let operation = &input["operation"];
                let action = operation["action"].as_str().unwrap();
                match input["collection"].as_str().unwrap() {
                    "configuration" => Ok(
                        json!({"records":[{"id":1,"revision":1,"value":self.config}],"next":null}),
                    ),
                    "conversations" => Ok(json!({"records":self.conversations,"next":null})),
                    "task_runs" => {
                        assert_eq!(action, "create");
                        let value = operation["value"].clone();
                        let key = value["key"].as_str().unwrap().to_owned();
                        let index = self.history.len() + 1;
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
                        let id = records.keys().last().copied().unwrap_or(0) + 1;
                        let record = json!({"id":id,"revision":1,"key":input["key"],"state":"pending","request":input});
                        records.insert(id, record.clone());
                        Ok(json!({field:[record],"next_after":null}))
                    }
                    "list" => {
                        Ok(json!({field:records.values().collect::<Vec<_>>(),"next_after":null}))
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
