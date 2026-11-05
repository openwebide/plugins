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
        "csharp".into(),
        LanguageGrammar::standard(SyntaxLanguage::CSharp),
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
    fn public_metadata_keeps_csharp_identity_and_native_project_discovery() {
        let definitions = LanguagePackage::languages();
        assert_eq!(definitions.len(), 1);
        let definition = &definitions[0];
        assert_eq!(definition.id, "csharp");
        assert_eq!(definition.extensions, ["cs"]);
        let server = definition.features.language_server.as_ref().unwrap();
        assert_eq!(server.command, "csharp-ls");
        // The server selects the solution from the authorized project root,
        // including .slnx and projects whose names are not known to the host.
        assert!(server.root_markers.is_empty());
        assert_eq!(server.settings["csharp"]["useMetadataUris"], true);
        assert_eq!(
            server.virtual_documents,
            [VirtualDocument {
                scheme: "csharp".into(),
                method: "csharp/metadata".into(),
                parameters: VirtualDocumentParameters::TextDocument,
                text_pointer: "/source".into(),
            }]
        );
        let _ = engine();
    }
}
