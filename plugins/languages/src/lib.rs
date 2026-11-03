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
