//! Host-executed Scheduling behavior using only the public SDK.
pub mod configuration;
pub mod conversations;
pub mod schedule;
pub mod service;
use openwebide_plugin_sdk::{
    EventInput, Outcome, Plugin, Tool,
    serde_json::{self, Value},
};

pub struct SchedulingPlugin;
impl Plugin for SchedulingPlugin {
    fn tools() -> Vec<Tool> {
        let manifest: Value =
            serde_json::from_str(include_str!("../plugin.json")).expect("embedded manifest");
        serde_json::from_value(manifest["contributions"]["tools"].clone())
            .expect("declared tool schemas")
    }
    fn events() -> Vec<String> {
        vec!["task_due".into(), "task_completed".into()]
    }
    fn execute(name: &str, arguments: Value) -> Result<Outcome, String> {
        service::execute(&mut service::SdkHost, name, arguments)
    }
    fn event(input: EventInput) -> Result<Outcome, String> {
        match input.name.as_str() {
            "task_due" => service::due(
                &mut service::SdkHost,
                serde_json::from_value(input.payload).map_err(|error| error.to_string())?,
            ),
            "task_completed" => service::completed(
                &mut service::SdkHost,
                serde_json::from_value(input.payload).map_err(|error| error.to_string())?,
            ),
            _ => Err("Unknown scheduling event".into()),
        }
    }
}
openwebide_plugin_sdk::export!(SchedulingPlugin);
