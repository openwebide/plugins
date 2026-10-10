//! First-party Web behavior uses the same SDK as community plugins.
mod html;
mod url;
mod web;
use openwebide_plugin_sdk::{
    Outcome, Plugin, Tool,
    serde_json::{self, Value},
};
struct Web;
impl Plugin for Web {
    fn tools() -> Vec<Tool> {
        let manifest: Value =
            serde_json::from_str(include_str!("../plugin.json")).expect("valid embedded manifest");
        serde_json::from_value(manifest["contributions"]["tools"].clone())
            .expect("valid embedded tools")
    }
    fn execute(name: &str, arguments: Value) -> Result<Outcome, String> {
        match name {
            "search_web" => {
                let query = arguments["query"].as_str().unwrap_or_default().trim();
                if query.is_empty() || query.len() > 1024 {
                    return Err("Search query must contain 1–1024 bytes.".into());
                }
                let limit = usize::try_from(arguments["limit"].as_i64().unwrap_or(5).clamp(1, 10))
                    .expect("bounded limit");
                let results = web::search_web_internal(query, limit)?;
                let count = results.len();
                let content = if results.is_empty() {
                    format!("no web results found for '{query}'")
                } else {
                    results
                        .iter()
                        .map(|result| {
                            format!(
                                "Title: {}\nURL: {}\nSnippet: {}",
                                result.title, result.url, result.snippet
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n---\n")
                };
                Ok(Outcome {
                    ok: true,
                    content,
                    summary: format!("search web '{query}' ({count} results)"),
                })
            }
            "fetch_web_page" => {
                let url = arguments["url"].as_str().unwrap_or_default().trim();
                if url.is_empty() || url.len() > 4096 {
                    return Err("Page URL must contain 1–4096 bytes.".into());
                }
                let content = web::fetch_page_internal(url)?;
                let summary = format!("fetch {url} ({} bytes)", content.len());
                Ok(Outcome {
                    ok: true,
                    content,
                    summary,
                })
            }
            _ => Err("Unknown Web tool".into()),
        }
    }
}
openwebide_plugin_sdk::export!(Web);
