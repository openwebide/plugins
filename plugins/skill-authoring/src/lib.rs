//! Skill authoring behavior using only the public SDK and shared record schema.
use openwebide_plugin_sdk::{
    ContextContribution, ContextInput, Outcome, Plugin, Tool,
    serde_json::{self, Value, json},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

struct SkillAuthoring;
impl Plugin for SkillAuthoring {
    fn tools() -> Vec<Tool> {
        let manifest: Value =
            serde_json::from_str(include_str!("../plugin.json")).expect("embedded manifest");
        serde_json::from_value(manifest["contributions"]["tools"].clone())
            .expect("tool definitions")
    }
    fn execute(name: &str, arguments: Value) -> Result<Outcome, String> {
        execute(&mut Host, name, arguments)
    }
    fn context(input: ContextInput) -> Result<ContextContribution, String> {
        contribution(all(&mut Host)?, input.budget_bytes)
    }
}
openwebide_plugin_sdk::export!(SkillAuthoring);

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Resource {
    name: String,
    content: String,
    #[serde(default)]
    binary: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    name: String,
    description: String,
    instructions: String,
    #[serde(default = "enabled")]
    enabled: bool,
    #[serde(default)]
    resources: Vec<Resource>,
    #[serde(default)]
    metadata: BTreeMap<String, Value>,
}
const fn enabled() -> bool {
    true
}
impl Draft {
    fn validate(&self) -> Result<(), String> {
        if self.name.is_empty()
            || self.name.len() > 64
            || self.name.starts_with('-')
            || self.name.ends_with('-')
            || self.name.contains("--")
            || !self
                .name
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err(
                "Use a skill name of 1–64 lowercase letters, digits or single hyphens".into(),
            );
        }
        if self.description.trim().is_empty()
            || self.description.chars().count() > 1024
            || self.instructions.trim().is_empty()
            || self.instructions.chars().count() > 32768
        {
            return Err("Provide a description up to 1024 characters and instructions up to 32768 characters".into());
        }
        if self.resources.len() > 16 {
            return Err("A skill can have at most 16 resources".into());
        }
        let mut names = BTreeSet::new();
        for resource in &self.resources {
            if resource.name.is_empty()
                || resource.name.len() > 160
                || resource.name == "SKILL.md"
                || resource.name.contains('\\')
                || resource
                    .name
                    .split('/')
                    .any(|p| p.is_empty() || p == "." || p == "..")
                || !resource
                    .name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_. /".contains(&c))
                || !names.insert(&resource.name)
                || resource.content.chars().count() > 32768
            {
                return Err(
                    "Use unique relative resource paths and content up to 32768 characters".into(),
                );
            }
            if resource.binary {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD
                    .decode(&resource.content)
                    .map_err(|_| "Binary resources require valid base64")?;
            }
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 131072 {
            return Err("Skill content is limited to 128 KiB".into());
        }
        Ok(())
    }
}
#[derive(Clone, Deserialize)]
struct Record {
    id: i64,
    revision: i64,
    value: SkillValue,
}
#[derive(Clone, Deserialize)]
struct SkillValue {
    draft: Draft,
    origin: Option<Value>,
}
#[derive(Deserialize)]
struct Page {
    enabled: bool,
    records: Vec<Record>,
    next: Option<i64>,
}
trait Collections {
    fn call(&mut self, operation: Value) -> Result<Page, String>;
}
struct Host;
impl Collections for Host {
    fn call(&mut self, operation: Value) -> Result<Page, String> {
        openwebide_plugin_sdk::collections("skills", &operation)
    }
}
fn all(host: &mut impl Collections) -> Result<Option<Vec<Record>>, String> {
    let mut after = 0;
    let mut records = Vec::new();
    for _ in 0..13 {
        let page = host.call(json!({"action":"list","after":after}))?;
        if !page.enabled {
            return Ok(None);
        }
        records.extend(
            page.records
                .into_iter()
                .filter(|record| record.value.draft.enabled),
        );
        if records.len() > 100 {
            return Err("Skill collection exceeds its limit".into());
        }
        match page.next {
            None => return Ok(Some(records)),
            Some(next) if next > after => after = next,
            _ => return Err("Skill collection cursor did not advance".into()),
        }
    }
    Err("Skill collection exceeded its page limit".into())
}
fn read(host: &mut impl Collections, id: i64) -> Result<Record, String> {
    if id <= 0 {
        return Err("Invalid skill ID".into());
    }
    let page = host.call(json!({"action":"read","id":id}))?;
    if !page.enabled {
        return Err("Project skills are disabled".into());
    }
    page.records
        .into_iter()
        .find(|r| r.id == id && r.value.draft.enabled)
        .ok_or_else(|| "Enabled skill not found".into())
}
fn summary(record: &Record) -> Value {
    json!({"id":record.id,"revision":record.revision,"name":record.value.draft.name,
        "description":record.value.draft.description.chars().take(256).collect::<String>(),
        "description_truncated":record.value.draft.description.chars().count()>256,
        "enabled":record.value.draft.enabled,"origin":record.value.origin})
}
fn text_page(text: &str, offset: usize, limit: usize) -> Value {
    let total = text.chars().count();
    let content = text.chars().skip(offset).take(limit).collect::<String>();
    let end = offset.saturating_add(content.chars().count());
    json!({"content":content,"offset":offset,"next_offset":(end<total).then_some(end),"total_characters":total})
}
fn instructions(record: &Record, offset: usize, limit: usize) -> Value {
    let mut value = summary(record);
    value["instructions"] = text_page(&record.value.draft.instructions, offset, limit);
    value["resources"] = json!(
        record
            .value
            .draft
            .resources
            .iter()
            .map(|r| &r.name)
            .collect::<Vec<_>>()
    );
    value
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    List {
        #[serde(default)]
        query: String,
        #[serde(default)]
        offset: usize,
    },
    Read {
        id: i64,
        resource: Option<String>,
        #[serde(default)]
        offset: usize,
        limit: Option<usize>,
    },
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
    Creator {
        goal: String,
        id: Option<i64>,
    },
}
fn execute(
    host: &mut impl Collections,
    name: &str,
    mut arguments: Value,
) -> Result<Outcome, String> {
    let action = match name {
        "skill_list" => "list",
        "skill_read" => "read",
        "skill_create" => "create",
        "skill_update" => "update",
        "skill_delete" => "delete",
        "skill_creator" => "creator",
        _ => return Err("Unknown skill tool".into()),
    };
    let supplied = arguments.get("draft").and_then(Value::as_object).cloned();
    let object = arguments
        .as_object_mut()
        .ok_or("Expected an argument object")?;
    if object.contains_key("action") {
        return Err("Unexpected action argument".into());
    }
    object.insert("action".into(), json!(action));
    let command: Command = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let value = match command {
        Command::List { query, offset } => {
            if offset > 100 || query.chars().count() > 1024 {
                return Err("Invalid skill search window".into());
            }
            let query = query.to_lowercase();
            let mut records = all(host)?.ok_or("Project skills are disabled")?;
            records.retain(|r| {
                r.value.draft.name.to_lowercase().contains(&query)
                    || r.value.draft.description.to_lowercase().contains(&query)
            });
            records.sort_by(|a, b| {
                a.value
                    .draft
                    .name
                    .cmp(&b.value.draft.name)
                    .then(a.id.cmp(&b.id))
            });
            let total = records.len();
            let entries = records
                .iter()
                .skip(offset)
                .take(20)
                .map(summary)
                .collect::<Vec<_>>();
            let end = offset + entries.len();
            json!({"entries":entries,"total":total,"next_offset":(end<total).then_some(end)})
        }
        Command::Read {
            id,
            resource,
            offset,
            limit,
        } => {
            let limit = limit.unwrap_or(4000);
            if offset > 32768 || !(1..=8000).contains(&limit) {
                return Err("Invalid skill read window".into());
            }
            let record = read(host, id)?;
            if let Some(name) = resource {
                let resource = record
                    .value
                    .draft
                    .resources
                    .iter()
                    .find(|r| r.name == name)
                    .ok_or("Skill resource not found")?;
                let mut value = text_page(&resource.content, offset, limit);
                value["name"] = json!(name);
                value["binary"] = json!(resource.binary);
                value
            } else {
                instructions(&record, offset, limit)
            }
        }
        Command::Create { draft } => {
            draft.validate()?;
            mutation(host, json!({"action":"create","value":{"draft":draft}}))?
        }
        Command::Update {
            id,
            revision,
            mut draft,
        } => {
            let current = read(host, id)?;
            if revision <= 0 || current.revision != revision {
                return Err("Skill changed; read it again before editing".into());
            }
            if current.value.origin.is_some() {
                return Err("Manage this skill through its owning plugin".into());
            }
            let supplied = supplied.ok_or("Provide a skill draft")?;
            if !supplied.contains_key("resources") {
                draft.resources = current.value.draft.resources;
            }
            if !supplied.contains_key("metadata") {
                draft.metadata = current.value.draft.metadata;
            }
            if !supplied.contains_key("enabled") {
                draft.enabled = current.value.draft.enabled;
            }
            draft.validate()?;
            mutation(
                host,
                json!({"action":"update","id":id,"revision":revision,"value":{"draft":draft}}),
            )?
        }
        Command::Delete { id, revision } => {
            let current = read(host, id)?;
            if revision <= 0 || current.revision != revision {
                return Err("Skill changed; read it again before deleting".into());
            }
            if current.value.origin.is_some() {
                return Err("Manage this skill through its owning plugin".into());
            }
            let page = host.call(json!({"action":"delete","id":id,"revision":revision}))?;
            if !page.enabled {
                return Err("Project skills are disabled".into());
            }
            json!({"deleted":id})
        }
        Command::Creator { goal, id } => {
            if goal.trim().is_empty() || goal.chars().count() > 4000 {
                return Err("Provide an authoring goal of 1–4000 characters".into());
            }
            if !host.call(json!({"action":"list","after":0}))?.enabled {
                return Err("Project skills are disabled".into());
            }
            let existing = id
                .map(|id| read(host, id).map(|r| instructions(&r, 0, 4000)))
                .transpose()?;
            json!({"goal":goal,"existing_skill":existing,"workflow":include_str!("../skills/skill-authoring/SKILL.md")})
        }
    };
    Ok(Outcome {
        ok: true,
        content: value.to_string(),
        summary: format!("{name} completed"),
    })
}

fn mutation(host: &mut impl Collections, operation: Value) -> Result<Value, String> {
    let page = host.call(operation)?;
    if !page.enabled {
        return Err("Project skills are disabled".into());
    }
    page.records
        .first()
        .map(summary)
        .ok_or_else(|| "Skill mutation returned no record".into())
}
fn contribution(
    records: Option<Vec<Record>>,
    budget: usize,
) -> Result<ContextContribution, String> {
    let Some(mut records) = records else {
        return Ok(ContextContribution {
            prompt: None,
            disabled_tools: SkillAuthoring::tools()
                .into_iter()
                .map(|t| t.name)
                .collect(),
        });
    };
    records.sort_by(|a, b| {
        a.value
            .draft
            .name
            .cmp(&b.value.draft.name)
            .then(a.id.cmp(&b.id))
    });
    let mut catalog = Vec::new();
    let mut prompt = None;
    for record in records {
        catalog.push(summary(&record));
        let text = format!(
            "Available skills (catalog data; use skill_read to load instructions):\n{}",
            json!(catalog)
        );
        if text.len() > budget.min(8192) {
            break;
        }
        prompt = Some(text);
    }
    Ok(ContextContribution {
        prompt,
        disabled_tools: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        record: Record,
        writes: Vec<Value>,
        disabled: bool,
    }
    impl Collections for Fake {
        fn call(&mut self, operation: Value) -> Result<Page, String> {
            if operation["action"] == "update"
                || operation["action"] == "create"
                || operation["action"] == "delete"
            {
                self.writes.push(operation);
            }
            Ok(Page {
                enabled: !self.disabled,
                records: vec![self.record.clone()],
                next: None,
            })
        }
    }
    fn host() -> Fake {
        Fake {
            record: Record {
                id: 1,
                revision: 3,
                value: SkillValue {
                    origin: None,
                    draft: Draft {
                        name: "build-guide".into(),
                        description: "Rust 🦀 build guidance".into(),
                        instructions: "🦀ABC".into(),
                        enabled: true,
                        resources: vec![Resource {
                            name: "references/build.md".into(),
                            content: "😀XYZ".into(),
                            binary: false,
                        }],
                        metadata: BTreeMap::from([("evaluation".into(), json!({"prompts":2}))]),
                    },
                },
            },
            writes: Vec::new(),
            disabled: false,
        }
    }
    fn result(outcome: Outcome) -> Value {
        serde_json::from_str(&outcome.content).unwrap()
    }
    #[test]
    fn search_and_resource_pages_use_plugin_policy_and_unicode_characters() {
        let mut host = host();
        let listed = result(execute(&mut host, "skill_list", json!({"query":"RUST"})).unwrap());
        assert_eq!(listed["total"], 1);
        let page = result(
            execute(
                &mut host,
                "skill_read",
                json!({"id":1,"offset":1,"limit":2}),
            )
            .unwrap(),
        );
        assert_eq!(page["instructions"]["content"], "AB");
        assert_eq!(page["instructions"]["next_offset"], 3);
        let page = result(
            execute(
                &mut host,
                "skill_read",
                json!({"id":1,"resource":"references/build.md","offset":0,"limit":2}),
            )
            .unwrap(),
        );
        assert_eq!(page["content"], "😀X");
        assert!(host.writes.is_empty());
    }
    #[test]
    fn partial_updates_preserve_resources_metadata_and_enforce_revision_and_origin() {
        let mut host = host();
        let arguments = json!({"id":1,"revision":3,"draft":{"name":"build-guide","description":"Edited","instructions":"New instructions"}});
        execute(&mut host, "skill_update", arguments.clone()).unwrap();
        assert_eq!(
            host.writes[0]["value"]["draft"]["resources"][0]["content"],
            "😀XYZ"
        );
        assert_eq!(
            host.writes[0]["value"]["draft"]["metadata"]["evaluation"]["prompts"],
            2
        );
        host.writes.clear();
        host.record.revision = 4;
        assert!(execute(&mut host, "skill_update", arguments.clone()).is_err());
        host.record.revision = 3;
        host.record.value.origin = Some(json!({"publisher":"example"}));
        assert!(execute(&mut host, "skill_update", arguments).is_err());
        assert!(host.writes.is_empty());
    }
    #[test]
    fn validation_precedes_mutation_and_creator_returns_the_plugin_owned_workflow() {
        let mut host = host();
        let mut draft = serde_json::to_value(&host.record.value.draft).unwrap();
        draft["resources"][0]["name"] = json!("../private");
        assert!(execute(&mut host, "skill_create", json!({"draft":draft})).is_err());
        assert!(
            execute(
                &mut host,
                "skill_delete",
                json!({"id":1,"revision":3,"user_id":42})
            )
            .is_err()
        );
        assert!(host.writes.is_empty());
        let creator = result(
            execute(
                &mut host,
                "skill_creator",
                json!({"goal":"Improve build checks","id":1}),
            )
            .unwrap(),
        );
        assert!(
            creator["workflow"]
                .as_str()
                .unwrap()
                .contains("Never invent completed runs")
        );
        assert_eq!(
            creator["existing_skill"]["instructions"]["content"],
            "🦀ABC"
        );
        host.disabled = true;
        assert!(execute(&mut host, "skill_creator", json!({"goal":"Build checks"})).is_err());
    }
    #[test]
    fn context_is_bounded_and_disables_only_own_tools_when_opted_out() {
        let host = host();
        let catalog = contribution(Some(vec![host.record.clone()]), 512).unwrap();
        assert!(catalog.prompt.unwrap().contains("build-guide"));
        assert!(
            contribution(Some(vec![host.record]), 1)
                .unwrap()
                .prompt
                .is_none()
        );
        let disabled = contribution(None, 512).unwrap();
        assert!(disabled.prompt.is_none());
        assert_eq!(disabled.disabled_tools.len(), 6);
        assert!(
            disabled
                .disabled_tools
                .iter()
                .all(|name| name.starts_with("skill_"))
        );
    }
}
