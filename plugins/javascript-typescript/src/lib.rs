use openwebide_language_syntax::{LanguageGrammar, LanguageSyntax};
use openwebide_plugin_sdk::{Plugin, languages::*, serde_json};
use std::{cell::RefCell, collections::BTreeMap};

struct LanguagePackage;
fn definitions() -> Vec<LanguageDefinition> {
    let manifest: serde_json::Value = serde_json::from_str(include_str!("../plugin.json")).unwrap();
    serde_json::from_value(manifest["contributions"]["languages"].clone()).unwrap()
}
fn engine() -> LanguageSyntax {
    let grammars = BTreeMap::from([
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
    ]);
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
    fn public_metadata_preserves_language_ids_and_shares_one_server_configuration() {
        let definitions = LanguagePackage::languages();
        let expected = [
            ("javascript", "javascript"),
            ("jsx", "javascriptreact"),
            ("typescript", "typescript"),
            ("tsx", "typescriptreact"),
        ];
        assert_eq!(definitions.len(), expected.len());
        let configuration = definitions[0]
            .features
            .language_server
            .as_ref()
            .unwrap()
            .shared_configuration();
        for (definition, (id, wire)) in definitions.iter().zip(expected) {
            assert_eq!(definition.id, id);
            let server = definition.features.language_server.as_ref().unwrap();
            assert_eq!(server.language_id.as_deref(), Some(wire));
            assert_eq!(server.shared_configuration(), configuration);
        }
        // Construct the actual parser component too: metadata and syntax must
        // agree under the same public SDK, not only pass offline JSON checks.
        let _ = engine();
    }
}
