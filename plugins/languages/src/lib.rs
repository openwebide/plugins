use openwebide_language_syntax::{LanguageGrammar, LanguageSyntax};
use openwebide_plugin_sdk::{Plugin, languages::*, serde_json};
use std::{cell::RefCell, collections::BTreeMap};

struct Languages;
fn definitions() -> Vec<LanguageDefinition> {
    let manifest: serde_json::Value = serde_json::from_str(include_str!("../plugin.json")).unwrap();
    serde_json::from_value(manifest["contributions"]["languages"].clone()).unwrap()
}
fn engine() -> LanguageSyntax {
    let grammars = BTreeMap::from([
        (
            "rust".into(),
            LanguageGrammar::standard(SyntaxLanguage::Rust),
        ),
        (
            "python".into(),
            LanguageGrammar::standard(SyntaxLanguage::Python),
        ),
        (
            "javascript".into(),
            LanguageGrammar::standard(SyntaxLanguage::JavaScript),
        ),
        ("jsx".into(), LanguageGrammar::standard(SyntaxLanguage::Jsx)),
        (
            "typescript".into(),
            LanguageGrammar::standard(SyntaxLanguage::TypeScript),
        ),
        ("tsx".into(), LanguageGrammar::standard(SyntaxLanguage::Tsx)),
        (
            "java".into(),
            LanguageGrammar::standard(SyntaxLanguage::Java),
        ),
        (
            "csharp".into(),
            LanguageGrammar::standard(SyntaxLanguage::CSharp),
        ),
        ("php".into(), LanguageGrammar::standard(SyntaxLanguage::Php)),
        (
            "json".into(),
            LanguageGrammar::standard(SyntaxLanguage::Json),
        ),
        (
            "html".into(),
            LanguageGrammar::standard(SyntaxLanguage::Html),
        ),
        ("css".into(), LanguageGrammar::standard(SyntaxLanguage::Css)),
        (
            "markdown".into(),
            LanguageGrammar::standard(SyntaxLanguage::Markdown),
        ),
        ("ini".into(), LanguageGrammar::standard(SyntaxLanguage::Ini)),
        ("xml".into(), LanguageGrammar::standard(SyntaxLanguage::Xml)),
        (
            "shell".into(),
            LanguageGrammar::standard(SyntaxLanguage::Shell),
        ),
        (
            "toml".into(),
            LanguageGrammar::standard(SyntaxLanguage::Toml),
        ),
        (
            "yaml".into(),
            LanguageGrammar::standard(SyntaxLanguage::Yaml),
        ),
        ("sql".into(), LanguageGrammar::standard(SyntaxLanguage::Sql)),
        ("c".into(), LanguageGrammar::standard(SyntaxLanguage::C)),
        ("cpp".into(), LanguageGrammar::standard(SyntaxLanguage::Cpp)),
        ("go".into(), LanguageGrammar::standard(SyntaxLanguage::Go)),
        (
            "markdown-inline".into(),
            LanguageGrammar::standard(SyntaxLanguage::MarkdownInline),
        ),
        (
            "plain".into(),
            LanguageGrammar::standard(SyntaxLanguage::Plain),
        ),
    ]);
    LanguageSyntax::new(definitions(), grammars).unwrap()
}
thread_local! { static SYNTAX: RefCell<LanguageSyntax> = RefCell::new(engine()); }
impl Plugin for Languages {
    fn languages() -> Vec<LanguageDefinition> {
        definitions()
    }
    fn language(input: LanguageInput) -> Result<LanguageOutput, String> {
        SYNTAX.with(|engine| engine.borrow_mut().handle(input))
    }
}
openwebide_plugin_sdk::export!(Languages);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_documents_keep_plain_source_and_generic_region_folds() {
        let mut syntax = engine();
        for (source, digest) in [
            (
                "#region group\n文😀\n#endregion\nafter\n",
                "db56e74c48a5122dee888210cb465a87a71a48adb3c3527a5807b50c609715b3",
            ),
            (
                "#region group\r\n文😀\r\n#endregion\r\nafter\r\n",
                "c2d8ef9e8b382fa2f6635e3388e1e8f21985eddb07128d39d552cd6981310fa1",
            ),
        ] {
            let output = syntax
                .handle(LanguageInput::Analyze(AnalysisRequest {
                    version: VERSION,
                    document: "unrecognized".into(),
                    request: 1,
                    revision: 1,
                    language: "plain".into(),
                    path: "unknown.custom-extension".into(),
                    source_digest: digest.into(),
                    source: SourceChange::Full {
                        text: source.into(),
                    },
                    tab_width: 4,
                }))
                .unwrap();
            let LanguageOutput::Analyzed(response) = output else {
                panic!("analysis required")
            };
            assert!(matches!(response.status, AnalysisStatus::Ready { .. }));
            let analysis = response.analysis.unwrap();
            assert!(analysis.folds.contains(&FoldRange {
                start_line: 0,
                end_line: 2
            }));
            if let Some(rows) = analysis.highlights {
                for (line, spans) in source.split('\n').zip(rows) {
                    assert_eq!(spans.last().map_or(0, |span| span.end), line.len());
                    assert!(spans.iter().all(|span| span.kind == TokenKind::Plain));
                }
            }
            syntax
                .handle(LanguageInput::Close {
                    version: VERSION,
                    document: "unrecognized".into(),
                })
                .unwrap();
        }
    }
}
