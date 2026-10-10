//! Default-model policy over raw public configuration metadata.
use crate::{conversations::Connection, schedule::ModelSelection};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Configuration {
    pub primary: Option<ModelSelection>,
    pub default_connection: Option<i64>,
    pub servers: Vec<Connection>,
}
impl Configuration {
    pub fn default_model(&self) -> Result<ModelSelection, String> {
        let selected = if let Some(primary) = &self.primary {
            primary.clone()
        } else {
            let server = if let Some(id) = self.default_connection {
                self.servers
                    .iter()
                    .find(|server| server.id == id)
                    .ok_or("The default server no longer exists")?
            } else {
                self.servers
                    .iter()
                    .find(|server| server.enabled)
                    .ok_or("Configure a model before scheduling work")?
            };
            ModelSelection {
                server_id: server.id,
                model: server
                    .model
                    .clone()
                    .ok_or("Choose a default model before scheduling work")?,
            }
        };
        if !self
            .servers
            .iter()
            .any(|server| server.id == selected.server_id && server.enabled)
            || selected.model.trim().is_empty()
            || selected.model.len() > 256
            || selected.model.chars().any(char::is_control)
        {
            return Err(
                "Choose an enabled default server and valid model before scheduling work".into(),
            );
        }
        Ok(selected)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_model_priority_is_source_policy_and_reads_current_metadata() {
        let mut config = Configuration {
            primary: None,
            default_connection: None,
            servers: vec![
                Connection {
                    id: 1,
                    model: Some("disabled".into()),
                    enabled: false,
                },
                Connection {
                    id: 2,
                    model: Some("first enabled".into()),
                    enabled: true,
                },
                Connection {
                    id: 3,
                    model: Some("explicit server".into()),
                    enabled: true,
                },
            ],
        };
        assert_eq!(config.default_model().unwrap().model, "first enabled");
        config.default_connection = Some(3);
        assert_eq!(config.default_model().unwrap().model, "explicit server");
        config.primary = Some(ModelSelection {
            server_id: 2,
            model: "configured primary".into(),
        });
        assert_eq!(config.default_model().unwrap().model, "configured primary");
        config.primary.as_mut().unwrap().model = "new primary".into();
        assert_eq!(config.default_model().unwrap().model, "new primary");
        config.servers[1].enabled = false;
        assert!(config.default_model().is_err());
        config.primary = None;
        config.default_connection = Some(99);
        assert!(config.default_model().is_err());
        config.default_connection = Some(3);
        config.servers[2].model = None;
        assert!(config.default_model().is_err());
    }
}
