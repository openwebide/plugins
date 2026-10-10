use openwebide_plugin_sdk::{Outcome, Plugin, Tool, serde_json::{Value, json}};
struct Echo;
impl Plugin for Echo {
    fn tools() -> Vec<Tool> {
        vec![Tool {
            name: "echo_message".into(), description: "Return a message using plugin-owned Rust code.".into(),
            parameters: json!({"type":"object","properties":{"message":{"type":"string"}},"required":["message"],"additionalProperties":false}),
            requires_approval: false,
        }]
    }
    fn execute(name: &str, arguments: Value) -> Result<Outcome, String> {
        if name != "echo_message" { return Err("Unknown tool".into()); }
        let message = arguments.get("message").and_then(Value::as_str)
            .ok_or("Expected a message string")?;
        if message.len() > 4096 { return Err("Message is too long".into()); }
        Ok(Outcome { ok: true, content: format!("Plugin says: {message}"), summary: "Echoed message".into() })
    }
}
openwebide_plugin_sdk::export!(Echo);
