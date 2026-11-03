use openwebide_language_syntax::{LanguageGrammar, LanguageSyntax};
use openwebide_plugin_sdk::{Plugin, languages::*, serde_json};
use std::{cell::RefCell, collections::BTreeMap};

struct LanguagePackage;
fn definitions() -> Vec<LanguageDefinition> {
    let manifest: serde_json::Value = serde_json::from_str(include_str!("../plugin.json")).unwrap();
    serde_json::from_value(manifest["contributions"]["languages"].clone()).unwrap()
}
fn engine() -> LanguageSyntax {
    let grammars = BTreeMap::from([("xml".into(), LanguageGrammar::standard(SyntaxLanguage::Xml))]);
    LanguageSyntax::new_delegated(definitions(), grammars).unwrap()
}
thread_local! { static SYNTAX: RefCell<LanguageSyntax> = RefCell::new(engine()); }
impl Plugin for LanguagePackage {
    fn languages() -> Vec<LanguageDefinition> {
        definitions()
    }
    fn language(input: LanguageInput) -> Result<LanguageOutput, String> {
        SYNTAX.with(|engine| engine.borrow_mut().handle(input))
    }
}
openwebide_plugin_sdk::export!(LanguagePackage);
