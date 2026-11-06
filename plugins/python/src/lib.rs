use openwebide_language_syntax::{LanguageGrammar, LanguageSyntax};
use openwebide_plugin_sdk::{Plugin, languages::*, serde_json};
use std::{cell::RefCell, collections::BTreeMap};

struct LanguagePackage;
fn definitions() -> Vec<LanguageDefinition> {
    let manifest: serde_json::Value = serde_json::from_str(include_str!("../plugin.json")).unwrap();
    serde_json::from_value(manifest["contributions"]["languages"].clone()).unwrap()
}
fn engine() -> LanguageSyntax {
    let grammars = BTreeMap::from([(
        "python".into(),
        LanguageGrammar::standard(SyntaxLanguage::Python),
    )]);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_metadata_retains_python_editing_and_declares_pyright() {
        let definitions = LanguagePackage::languages();
        assert_eq!(definitions.len(), 1);
        let definition = &definitions[0];
        assert_eq!(definition.id, "python");
        assert_eq!(definition.extensions, ["py", "pyi"]);
        assert_eq!(definition.configuration.line_comment.as_deref(), Some("#"));
        assert!(definition.features.syntax);
        assert!(definition.features.folding);
        assert!(definition.features.structure);
        let server = definition.features.language_server.as_ref().unwrap();
        assert_eq!(server.command, "pyright-langserver");
        assert_eq!(server.args, ["--stdio"]);
        // The server owns project configuration and environment discovery.
        assert!(server.root_markers.is_empty());
        assert_eq!(
            server.settings["python"]["analysis"]["diagnosticMode"],
            "openFilesOnly"
        );
        assert_eq!(
            server.settings["python"]["analysis"]["autoImportCompletions"],
            true
        );
        let _ = engine();
    }
}
