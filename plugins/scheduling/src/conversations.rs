//! Select conversations from scoped SDK metadata; the host does not choose “latest”.
use crate::schedule::{Draft, ModelSelection, SessionTarget};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct Conversation {
    pub id: i64,
    pub value: Metadata,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Metadata {
    pub archived: bool,
    pub last_activity: i64,
    pub model_override: Option<ModelSelection>,
    pub connection: Option<Connection>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Connection {
    pub id: i64,
    pub model: Option<String>,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    Session { id: i64 },
    New { title: String },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub target: Target,
    pub model: ModelSelection,
}
/// Call with all pages collected at delivery time, not a snapshot taken at creation.
/// `default_model` must come from current general host configuration metadata.
pub fn select(
    draft: &Draft,
    conversations: &[Conversation],
    default_model: Option<&ModelSelection>,
) -> Result<Selection, String> {
    let conversation = match draft.session_target {
        SessionTarget::Existing => Some(
            conversations
                .iter()
                .find(|row| row.id == draft.session_id)
                .ok_or("The task conversation no longer exists in this project")?,
        ),
        SessionTarget::New => None,
        SessionTarget::Latest => conversations
            .iter()
            .filter(|row| !row.value.archived)
            .max_by_key(|row| (row.value.last_activity, row.id)),
    };
    let target = conversation.map_or_else(
        || Target::New {
            title: draft.title.clone(),
        },
        |row| Target::Session { id: row.id },
    );
    let configured = conversation.and_then(|row| {
        row.value.model_override.clone().or_else(|| {
            row.value
                .connection
                .as_ref()
                .filter(|connection| connection.enabled)
                .and_then(|connection| {
                    connection.model.as_ref().map(|model| ModelSelection {
                        server_id: connection.id,
                        model: model.clone(),
                    })
                })
        })
    });
    let model = draft
        .model
        .clone()
        .or(configured)
        .or_else(|| default_model.cloned())
        .ok_or("Configure a model before scheduling this conversation")?;
    if model.server_id <= 0
        || model.model.trim().is_empty()
        || model.model.len() > 256
        || model.model.chars().any(char::is_control)
    {
        return Err("Choose a valid server and model for this task".into());
    }
    Ok(Selection { target, model })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;
    fn draft(target: SessionTarget) -> Draft {
        Draft {
            session_target: target,
            session_id: 1,
            title: "Task".into(),
            auto_title: false,
            prompt: "Work".into(),
            enabled: true,
            model: None,
            schedule: Schedule::Once { at: 100 },
        }
    }
    fn model(id: i64, name: &str) -> ModelSelection {
        ModelSelection {
            server_id: id,
            model: name.into(),
        }
    }
    fn row(id: i64, activity: i64, archived: bool) -> Conversation {
        Conversation {
            id,
            value: Metadata {
                archived,
                last_activity: activity,
                model_override: None,
                connection: Some(Connection {
                    id: 9,
                    enabled: true,
                    model: Some("configured".into()),
                }),
            },
        }
    }
    #[test]
    fn latest_excludes_archived_and_breaks_activity_ties_by_id_each_delivery() {
        let mut rows = vec![
            row(1, 10, false),
            row(2, 20, false),
            row(3, 20, false),
            row(4, 99, true),
        ];
        assert_eq!(
            select(&draft(SessionTarget::Latest), &rows, None)
                .unwrap()
                .target,
            Target::Session { id: 3 }
        );
        rows[0].value.last_activity = 100;
        assert_eq!(
            select(&draft(SessionTarget::Latest), &rows, None)
                .unwrap()
                .target,
            Target::Session { id: 1 }
        );
        // An explicit existing conversation may be archived; latest is a different policy.
        rows[0].value.archived = true;
        assert_eq!(
            select(&draft(SessionTarget::Existing), &rows, None)
                .unwrap()
                .target,
            Target::Session { id: 1 }
        );
        assert!(
            select(
                &draft(SessionTarget::Existing),
                &[],
                Some(&model(1, "default"))
            )
            .is_err()
        );
    }
    #[test]
    fn model_precedence_and_empty_latest_fallback_belong_to_source() {
        let mut rows = vec![row(1, 1, false)];
        let mut task = draft(SessionTarget::Existing);
        assert_eq!(
            select(&task, &rows, None).unwrap().model,
            model(9, "configured")
        );
        rows[0].value.model_override = Some(model(8, "override"));
        assert_eq!(
            select(&task, &rows, None).unwrap().model,
            model(8, "override")
        );
        task.model = Some(model(7, "task pin"));
        assert_eq!(
            select(&task, &rows, None).unwrap().model,
            model(7, "task pin")
        );
        task.model = None;
        rows[0].value.model_override = None;
        rows[0].value.connection.as_mut().unwrap().enabled = false;
        assert_eq!(
            select(&task, &rows, Some(&model(6, "current default")))
                .unwrap()
                .model,
            model(6, "current default")
        );
        assert!(select(&task, &rows, None).is_err());
        let selection = select(
            &draft(SessionTarget::Latest),
            &[],
            Some(&model(5, "new default")),
        )
        .unwrap();
        assert_eq!(
            selection.target,
            Target::New {
                title: "Task".into()
            }
        );
        assert_eq!(selection.model, model(5, "new default"));
        task.model = Some(model(1, "\n"));
        assert!(select(&task, &rows, None).is_err());
    }
}
