//! Pull-request behavior owned by this package, using public host primitives only.
use base64::{Engine, engine::general_purpose::STANDARD};
use openwebide_plugin_sdk::{HttpRequest, HttpResponse, Plugin, panels::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

struct Reviews;
openwebide_plugin_sdk::export!(Reviews);

impl Plugin for Reviews {
    fn panels() -> Vec<PanelDefinition> {
        vec![definition()]
    }
    fn panel(input: PanelInput) -> Result<PanelDocument, String> {
        handle(input, &mut Sdk)
    }
}

fn definition() -> PanelDefinition {
    PanelDefinition {
        id: "pull-requests".into(),
        title: "Pull requests".into(),
        icon: PanelIcon::GitPullRequest,
        view: "render".into(),
        refresh_seconds: Some(60),
        actions: [
            ("configure", "Save configuration", false),
            ("open", "Open pull request", false),
            ("list", "Show pull requests", false),
            ("publish", "Publish review", true),
        ]
        .into_iter()
        .map(|(id, title, requires_approval)| PanelAction {
            id: id.into(),
            handler: id.into(),
            title: title.into(),
            requires_approval,
        })
        .collect(),
    }
}

/// This boundary exists for native contract tests. Production uses the SDK's
/// general HTTP and private-record primitives, with authority supplied by the host.
trait Capabilities {
    fn records(&mut self, operation: Value) -> Result<Value, String>;
    fn http(&mut self, request: HttpRequest) -> Result<HttpResponse, String>;
}
struct Sdk;
impl Capabilities for Sdk {
    fn records(&mut self, operation: Value) -> Result<Value, String> {
        openwebide_plugin_sdk::records("github", &operation)
    }
    fn http(&mut self, request: HttpRequest) -> Result<HttpResponse, String> {
        openwebide_plugin_sdk::http(&request)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Configuration {
    api: String,
    repository: String,
    token: String,
}
struct SavedConfiguration {
    id: i64,
    revision: i64,
    value: Configuration,
}
fn configuration(caps: &mut impl Capabilities) -> Result<Option<SavedConfiguration>, String> {
    let records = caps.records(json!({"action":"list","after":0}))?;
    let Some(record) = records["records"]
        .as_array()
        .and_then(|records| records.first())
    else {
        return Ok(None);
    };
    Ok(Some(SavedConfiguration {
        id: record["id"]
            .as_i64()
            .ok_or("Invalid configuration record")?,
        revision: record["revision"]
            .as_i64()
            .ok_or("Invalid configuration revision")?,
        value: serde_json::from_value(record["value"].clone())
            .map_err(|_| "Invalid saved GitHub configuration")?,
    }))
}
fn value<'a>(values: &'a BTreeMap<String, Value>, key: &str) -> Result<&'a str, String> {
    values
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing {key}"))
}
fn validate_configuration(config: &Configuration) -> Result<(), String> {
    let url = url::Url::parse(&config.api).map_err(|_| "Enter a valid GitHub API URL")?;
    if config.api.len() > 2048
        || config.api.chars().any(char::is_whitespace)
        || !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "GitHub API URLs require HTTP(S), without credentials, queries or fragments".into(),
        );
    }
    let segments = config.repository.split('/').collect::<Vec<_>>();
    if segments.len() != 2
        || segments.iter().any(|part| {
            part.is_empty()
                || part.len() > 100
                || matches!(*part, "." | "..")
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
        || config.token.len() > 1024
        || config.token.contains(['\r', '\n', '\0'])
    {
        return Err("Enter a repository as owner/name and a valid token".into());
    }
    Ok(())
}
fn save_configuration(
    caps: &mut impl Capabilities,
    previous: Option<&SavedConfiguration>,
    values: &BTreeMap<String, Value>,
) -> Result<Configuration, String> {
    let token = values
        .get("token")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| previous.map_or_else(String::new, |config| config.value.token.clone()));
    let config = Configuration {
        api: value(values, "api")?.trim().trim_end_matches('/').into(),
        repository: value(values, "repository")?.trim().into(),
        token,
    };
    validate_configuration(&config)?;
    let operation = if let Some(previous) = previous {
        json!({"action":"update","id":previous.id,"revision":previous.revision,"value":config})
    } else {
        json!({"action":"create","value":config})
    };
    caps.records(operation)?;
    Ok(config)
}
fn http(
    caps: &mut impl Capabilities,
    config: &Configuration,
    method: &str,
    suffix: &str,
    accept: &str,
    body: Option<Value>,
) -> Result<String, String> {
    validate_configuration(config)?;
    let mut headers = BTreeMap::from([
        ("accept".into(), accept.into()),
        ("user-agent".into(), "Open WebIDE PR Review".into()),
        ("x-github-api-version".into(), "2022-11-28".into()),
    ]);
    if !config.token.is_empty() {
        headers.insert("authorization".into(), format!("Bearer {}", config.token));
    }
    let bytes = body
        .map(|body| serde_json::to_vec(&body))
        .transpose()
        .map_err(|_| "Invalid review request")?
        .unwrap_or_default();
    if !bytes.is_empty() {
        headers.insert("content-type".into(), "application/json".into());
    }
    let response = caps.http(HttpRequest {
        url: format!("{}/repos/{}/{}", config.api, config.repository, suffix),
        method: method.into(),
        headers,
        body_base64: STANDARD.encode(bytes),
    })?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "GitHub returned HTTP {}. Check the repository, API URL and token permissions.",
            response.status
        ));
    }
    response
        .text()
        .map_err(|_| "GitHub returned an unreadable response".into())
}
fn get(
    caps: &mut impl Capabilities,
    config: &Configuration,
    suffix: &str,
) -> Result<Value, String> {
    serde_json::from_str(&http(
        caps,
        config,
        "GET",
        suffix,
        "application/vnd.github+json",
        None,
    )?)
    .map_err(|_| "GitHub returned invalid JSON".into())
}
fn bounded(value: &str, maximum: usize) -> String {
    value[..value.floor_char_boundary(value.len().min(maximum))].into()
}
fn button(action: &str, label: &str, values: Value) -> PanelButton {
    PanelButton {
        action: action.into(),
        label: label.into(),
        values: values
            .as_object()
            .map(|values| {
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default(),
        disabled: false,
    }
}
fn text_field(id: &str, label: &str, text: &str, length: usize) -> PanelField {
    PanelField {
        id: id.into(),
        label: label.into(),
        kind: FieldKind::Text {
            multiline: false,
            max_length: length,
        },
        value: json!(text),
        required: true,
    }
}
fn configuration_form(config: Option<&Configuration>) -> Block {
    Block::Group {
        title: Some("GitHub configuration".into()),
        blocks: vec![Block::Form {
            id: "configuration".into(),
            fields: vec![
                text_field(
                    "api",
                    "API URL",
                    config.map_or("https://api.github.com", |config| &config.api),
                    2048,
                ),
                text_field(
                    "repository",
                    "Repository",
                    config.map_or("", |config| &config.repository),
                    201,
                ),
                PanelField {
                    id: "token".into(),
                    label: if config.is_some_and(|config| !config.token.is_empty()) {
                        "Token (leave blank to keep saved token)"
                    } else {
                        "Token (optional for public repositories)"
                    }
                    .into(),
                    kind: FieldKind::Secret { max_length: 1024 },
                    value: Value::Null,
                    required: false,
                },
            ],
            submit: button("configure", "Save configuration", json!({})),
        }],
    }
}
fn number(input: &PanelInput) -> Option<u64> {
    input
        .values
        .get("number")
        .and_then(Value::as_u64)
        .or_else(|| input.state["number"].as_u64())
        .filter(|number| *number > 0)
}
fn commit(pr: &Value) -> Result<&str, String> {
    pr["head"]["sha"]
        .as_str()
        .filter(|sha| {
            (40..=64).contains(&sha.len()) && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .ok_or_else(|| "Pull request has no valid head commit".into())
}
fn detail(
    caps: &mut impl Capabilities,
    config: &Configuration,
    number: u64,
    configuration_revision: i64,
) -> Result<Vec<Block>, String> {
    let pr = get(caps, config, &format!("pulls/{number}"))?;
    let head = commit(&pr)?.to_owned();
    let title = pr["title"].as_str().unwrap_or("Pull request");
    let mut blocks = vec![
        Block::Heading {
            text: bounded(&format!("#{number} {title}"), 1024),
        },
        Block::Link {
            text: "View on GitHub".into(),
            url: pr["html_url"]
                .as_str()
                .ok_or("Pull request has no URL")?
                .into(),
        },
        Block::Text {
            text: bounded(pr["body"].as_str().unwrap_or("No description"), 32768),
        },
    ];
    let diff = http(
        caps,
        config,
        "GET",
        &format!("pulls/{number}"),
        "application/vnd.github.diff",
        None,
    )?;
    let truncated = diff.len() > 65536;
    blocks.push(Block::Code {
        text: bounded(&diff, 65536),
        language: Some("diff".into()),
    });
    if truncated {
        blocks.push(Block::Status {
            text: "Showing the first 64 KiB of the diff. Open GitHub for the complete diff.".into(),
            tone: Tone::Warning,
        });
    }
    let reviews = get(caps, config, &format!("pulls/{number}/reviews?per_page=30"))?;
    if let Some(reviews) = reviews.as_array() {
        blocks.push(Block::List {
            items: reviews
                .iter()
                .take(30)
                .map(|review| {
                    bounded(
                        &format!(
                            "{}: {} — {}",
                            review["user"]["login"].as_str().unwrap_or("Reviewer"),
                            review["state"].as_str().unwrap_or("Review"),
                            review["body"].as_str().unwrap_or("")
                        ),
                        1024,
                    )
                })
                .collect(),
        });
    }
    if commit(&get(caps, config, &format!("pulls/{number}"))?)? != head {
        return Err("Pull request changed while loading. Refresh before reviewing.".into());
    }
    if pr["state"].as_str() == Some("open") {
        blocks.push(Block::Form {id:"review".into(),fields:vec![
            PanelField {id:"event".into(),label:"Review".into(),kind:FieldKind::Select {options:[("COMMENT","Comment"),("APPROVE","Approve"),("REQUEST_CHANGES","Request changes")].into_iter().map(|(value,label)|FieldOption {value:value.into(),label:label.into()}).collect()},value:json!("COMMENT"),required:true},
            PanelField {id:"body".into(),label:"Review message".into(),kind:FieldKind::Text {multiline:true,max_length:16384},value:Value::Null,required:true},
        ],submit:button("publish","Publish review",json!({"number":number,"repository":config.repository,"api":config.api,"configuration_revision":configuration_revision,"commit_id":head}))});
    } else {
        blocks.push(Block::Status {
            text: "This pull request is closed.".into(),
            tone: Tone::Neutral,
        });
    }
    if config.token.is_empty() {
        for block in &mut blocks {
            if let Block::Form { id, submit, .. } = block
                && id == "review"
            {
                submit.disabled = true;
            }
        }
        blocks.push(Block::Status {
            text: "Save a token with pull-request write permission to publish a review.".into(),
            tone: Tone::Neutral,
        });
    }
    blocks.push(Block::Actions {
        buttons: vec![button("list", "Back to pull requests", json!({}))],
    });
    Ok(blocks)
}
fn listing(caps: &mut impl Capabilities, config: &Configuration) -> Result<Vec<Block>, String> {
    let prs = get(
        caps,
        config,
        "pulls?state=open&sort=updated&direction=desc&per_page=30",
    )?;
    let prs = prs
        .as_array()
        .ok_or("GitHub returned an invalid pull-request list")?;
    if prs.is_empty() {
        return Ok(vec![Block::Text {
            text: "No open pull requests.".into(),
        }]);
    }
    let mut blocks = vec![Block::Heading {
        text: format!("{} — open pull requests", config.repository),
    }];
    for pr in prs.iter().take(30) {
        let number = pr["number"].as_u64().ok_or("Pull request has no number")?;
        blocks.push(Block::Text {
            text: bounded(
                &format!(
                    "#{number} {} · {}",
                    pr["title"].as_str().unwrap_or("Pull request"),
                    pr["user"]["login"].as_str().unwrap_or("Author")
                ),
                2048,
            ),
        });
        blocks.push(Block::Actions {
            buttons: vec![button(
                "open",
                "Open pull request",
                json!({"number":number}),
            )],
        });
    }
    Ok(blocks)
}
fn handle(input: PanelInput, caps: &mut impl Capabilities) -> Result<PanelDocument, String> {
    let mut saved = configuration(caps)?;
    let mut config = saved.as_ref().map(|saved| saved.value.clone());
    let mut selected = number(&input);
    let mut notice = None;
    match input.handler.as_str() {
        "render" => {}
        "configure" => {
            save_configuration(caps, saved.as_ref(), &input.values)?;
            saved = configuration(caps)?;
            config = saved.as_ref().map(|saved| saved.value.clone());
            selected = None;
        }
        "open" => {
            selected = Some(number(&input).ok_or("Choose a pull request")?);
        }
        "list" => {
            selected = None;
        }
        "publish" => {
            let config = config
                .as_ref()
                .ok_or("Configure GitHub before publishing")?;
            if config.token.is_empty() {
                return Err(
                    "Save a GitHub token with pull-request write permission before publishing"
                        .into(),
                );
            }
            if value(&input.values, "repository")? != config.repository
                || value(&input.values, "api")? != config.api
                || input
                    .values
                    .get("configuration_revision")
                    .and_then(Value::as_i64)
                    != saved.as_ref().map(|saved| saved.revision)
            {
                return Err(
                    "GitHub configuration changed. Refresh and review the destination again."
                        .into(),
                );
            }
            let number = number(&input).ok_or("Choose a pull request")?;
            let approved_commit = value(&input.values, "commit_id")?;
            let current = get(caps, config, &format!("pulls/{number}"))?;
            if current["state"].as_str() != Some("open") || commit(&current)? != approved_commit {
                return Err("Pull request changed. Refresh and review its current commit before publishing.".into());
            }
            let event = value(&input.values, "event")?;
            let body = value(&input.values, "body")?;
            if !["COMMENT", "APPROVE", "REQUEST_CHANGES"].contains(&event)
                || body.trim().is_empty()
                || body.len() > 16384
            {
                return Err("Choose a review type and enter a message up to 16 KiB".into());
            }
            http(
                caps,
                config,
                "POST",
                &format!("pulls/{number}/reviews"),
                "application/vnd.github+json",
                Some(json!({"event":event,"body":body,"commit_id":approved_commit})),
            )?;
            notice = Some(Block::Status {
                text: "Review published.".into(),
                tone: Tone::Success,
            });
            selected = Some(number);
        }
        _ => return Err("Unknown pull-request panel handler".into()),
    }
    let mut blocks = Vec::new();
    if let Some(notice) = notice {
        blocks.push(notice);
    }
    if let Some(config) = &config {
        let view = if let Some(number) = selected {
            detail(
                caps,
                config,
                number,
                saved.as_ref().map_or(0, |saved| saved.revision),
            )
        } else {
            listing(caps, config)
        };
        match view {
            Ok(view) => blocks.extend(view),
            Err(message) => blocks.push(Block::Status {
                text: message,
                tone: Tone::Warning,
            }),
        }
    } else {
        blocks.push(Block::Text {
            text: "Choose a GitHub repository to view pull requests.".into(),
        });
    }
    blocks.push(configuration_form(config.as_ref()));
    let mut document = PanelDocument {
        schema_version: UI_VERSION,
        revision: input
            .revision
            .checked_add(1)
            .ok_or("Panel revision exhausted")?,
        blocks,
        state: selected.map_or(Value::Null, |number| json!({"number":number})),
    };
    let mut shortened = false;
    for _ in 0..12 {
        if serde_json::to_vec(&document)
            .map_err(|_| "Invalid panel document")?
            .len()
            <= MAX_DOCUMENT_BYTES - 512
        {
            if shortened {
                document.blocks.insert(0,Block::Status {text:"Some content was shortened to fit this panel. Open GitHub for the complete content.".into(),tone:Tone::Warning});
            }
            return Ok(document);
        }
        shortened = true;
        shorten(&mut document.blocks);
    }
    Err("GitHub content cannot fit in this panel. View it on GitHub.".into())
}

fn shorten(blocks: &mut [Block]) {
    for block in blocks {
        match block {
            Block::Text { text } | Block::Code { text, .. } => {
                *text = bounded(text, text.len() / 2)
            }
            Block::List { items } => {
                for item in items {
                    *item = bounded(item, item.len() / 2);
                }
            }
            Block::Group { blocks, .. } => shorten(blocks),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        record: Option<Value>,
        requests: Vec<HttpRequest>,
        head: String,
        change_during_diff: bool,
        content: String,
    }
    impl Fake {
        fn configured() -> Self {
            Self {
                record: Some(
                    json!({"id":1,"revision":1,"value":{"api":"https://api.github.com","repository":"owner/repo","token":"fixture-private-token"}}),
                ),
                requests: Vec::new(),
                head: "a".repeat(40),
                change_during_diff: false,
                content: "A useful change".into(),
            }
        }
        fn post_count(&self) -> usize {
            self.requests
                .iter()
                .filter(|request| request.method == "POST")
                .count()
        }
    }
    impl Capabilities for Fake {
        fn records(&mut self, operation: Value) -> Result<Value, String> {
            match operation["action"].as_str().unwrap() {
                "create" | "update" => {
                    let revision = self
                        .record
                        .as_ref()
                        .map_or(0, |record| record["revision"].as_i64().unwrap())
                        + 1;
                    self.record =
                        Some(json!({"id":1,"revision":revision,"value":operation["value"]}));
                }
                "list" => {}
                _ => panic!("Unexpected record primitive"),
            }
            Ok(json!({"records":self.record.clone().into_iter().collect::<Vec<_>>(),"next":null}))
        }
        fn http(&mut self, request: HttpRequest) -> Result<HttpResponse, String> {
            let body = if request.method == "POST" {
                "{}".into()
            } else if request.headers["accept"] == "application/vnd.github.diff" {
                if self.change_during_diff {
                    self.head = "b".repeat(40);
                }
                format!("diff --git a/file b/file\n+{}", self.content)
            } else if request.url.ends_with("reviews?per_page=30") {
                "[]".into()
            } else if request.url.ends_with("pulls/42") {
                json!({"number":42,"title":"Improve things","body":self.content,"state":"open","head":{"sha":self.head},"html_url":"https://github.com/owner/repo/pull/42"}).to_string()
            } else {
                json!([{"number":42,"title":self.content,"user":{"login":"contributor"}}])
                    .to_string()
            };
            self.requests.push(request);
            Ok(HttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body_base64: STANDARD.encode(body),
            })
        }
    }
    fn input(handler: &str) -> PanelInput {
        PanelInput {
            panel: "pull-requests".into(),
            instance: "a".repeat(32),
            request: "b".repeat(32),
            revision: 1,
            handler: handler.into(),
            action: (handler != "render").then(|| handler.into()),
            state: Value::Null,
            values: BTreeMap::new(),
        }
    }
    fn publish() -> PanelInput {
        let mut input = input("publish");
        input.values = [
            ("number".into(), json!(42)),
            ("repository".into(), json!("owner/repo")),
            ("api".into(), json!("https://api.github.com")),
            ("configuration_revision".into(), json!(1)),
            ("commit_id".into(), json!("a".repeat(40))),
            ("event".into(), json!("COMMENT")),
            ("body".into(), json!("Looks good")),
        ]
        .into();
        input
    }
    #[test]
    fn views_use_readonly_http_and_never_return_credentials() {
        let mut caps = Fake::configured();
        let view = handle(input("render"), &mut caps).unwrap();
        assert_eq!(view.revision, 2);
        assert!(
            !serde_json::to_string(&view)
                .unwrap()
                .contains("fixture-private-token")
        );
        assert!(
            caps.requests
                .iter()
                .all(|request| request.method == "GET" && request.body_base64.is_empty())
        );
        assert!(
            view.blocks
                .iter()
                .any(|block| matches!(block, Block::Actions { .. }))
        );
        assert!(
            definition()
                .actions
                .iter()
                .find(|action| action.id == "publish")
                .unwrap()
                .requires_approval
        );
    }
    #[test]
    fn publishing_is_owned_http_and_pins_the_reviewed_commit() {
        let mut caps = Fake::configured();
        let view = handle(publish(), &mut caps).unwrap();
        assert_eq!(caps.post_count(), 1);
        let request = caps
            .requests
            .iter()
            .find(|request| request.method == "POST")
            .unwrap();
        assert_eq!(
            request.url,
            "https://api.github.com/repos/owner/repo/pulls/42/reviews"
        );
        let body: Value =
            serde_json::from_slice(&STANDARD.decode(&request.body_base64).unwrap()).unwrap();
        assert_eq!(
            body,
            json!({"event":"COMMENT","body":"Looks good","commit_id":"a".repeat(40)})
        );
        assert!(view.blocks.iter().any(|block| matches!(
            block,
            Block::Status {
                tone: Tone::Success,
                ..
            }
        )));
        assert!(
            !serde_json::to_string(&view)
                .unwrap()
                .contains("fixture-private-token")
        );
    }
    #[test]
    fn changed_configuration_and_head_cannot_publish() {
        let mut caps = Fake::configured();
        caps.record.as_mut().unwrap()["revision"] = json!(2);
        assert!(handle(publish(), &mut caps).is_err());
        assert_eq!(caps.post_count(), 0);
        let mut caps = Fake::configured();
        caps.head = "b".repeat(40);
        assert!(handle(publish(), &mut caps).is_err());
        assert_eq!(caps.post_count(), 0);
    }
    #[test]
    fn changed_diff_has_no_publish_controls() {
        let mut caps = Fake::configured();
        caps.change_during_diff = true;
        let mut open = input("open");
        open.values.insert("number".into(), json!(42));
        let view = handle(open, &mut caps).unwrap();
        assert!(view.blocks.iter().any(|block| matches!(
            block,
            Block::Status {
                tone: Tone::Warning,
                ..
            }
        )));
        assert!(
            !view
                .blocks
                .iter()
                .any(|block| matches!(block,Block::Form {id,..} if id=="review"))
        );
        assert_eq!(caps.post_count(), 0);
    }
    #[test]
    fn oversized_upstream_content_stays_within_the_ui_document_limit() {
        let mut caps = Fake::configured();
        caps.content = "\u{1}".repeat(200_000);
        let mut open = input("open");
        open.values.insert("number".into(), json!(42));
        let view = handle(open, &mut caps).unwrap();
        assert!(serde_json::to_vec(&view).unwrap().len() <= MAX_DOCUMENT_BYTES);
        assert_eq!(caps.post_count(), 0);
    }
    #[test]
    fn blank_configuration_token_preserves_the_saved_secret() {
        let mut caps = Fake::configured();
        let mut configure = input("configure");
        configure.values = [
            ("api".into(), json!("https://api.github.com")),
            ("repository".into(), json!("owner/other")),
            ("token".into(), json!("")),
        ]
        .into();
        let view = handle(configure, &mut caps).unwrap();
        assert_eq!(
            caps.record.as_ref().unwrap()["value"]["token"],
            "fixture-private-token"
        );
        assert_eq!(
            caps.record.as_ref().unwrap()["value"]["repository"],
            "owner/other"
        );
        assert!(
            !serde_json::to_string(&view)
                .unwrap()
                .contains("fixture-private-token")
        );
        assert_eq!(caps.post_count(), 0);
    }
}
