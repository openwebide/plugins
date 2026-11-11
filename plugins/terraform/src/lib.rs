//! Independent HCL grammar and policies over the same public parser toolkit
//! available to community packages. No app-owned language registration.
use openwebide_language_grammars::{
    SyntaxContextKind, SyntaxContextScope, SyntaxHighlightScope, SyntaxProvider,
};
use openwebide_language_syntax::{LanguageGrammar, LanguageSyntax};
use openwebide_plugin_sdk::{Plugin, languages::*, serde_json};
use std::{cell::RefCell, collections::BTreeMap};

struct Terraform;
mod recovery;

fn definitions() -> Vec<LanguageDefinition> {
    let manifest: serde_json::Value = serde_json::from_str(include_str!("../plugin.json")).unwrap();
    serde_json::from_value(manifest["contributions"]["languages"].clone()).unwrap()
}

fn context(node: tree_sitter::Node<'_>) -> Option<SyntaxContextKind> {
    match node.kind() {
        "comment" => Some(SyntaxContextKind::Comment),
        "string_lit" => Some(SyntaxContextKind::String),
        "quoted_template" | "heredoc_template" => Some(SyntaxContextKind::Template),
        "template_interpolation"
        | "template_if_intro"
        | "template_else_intro"
        | "template_if_end"
        | "template_for_start"
        | "template_for_end" => Some(SyntaxContextKind::Interpolation),
        _ => None,
    }
}

fn highlight(
    node: tree_sitter::Node<'_>,
    parent: Option<tree_sitter::Node<'_>>,
) -> Option<TokenKind> {
    match node.kind() {
        "comment" => Some(TokenKind::Comment),
        "string_lit" | "template_literal" | "heredoc_identifier" => Some(TokenKind::String),
        "numeric_lit" => Some(TokenKind::Number),
        "bool_lit" => Some(TokenKind::Boolean),
        "null_lit" | "for" | "in" | "if" | "else" | "endif" | "endfor" => Some(TokenKind::Keyword),
        "identifier" => match parent.map(|parent| parent.kind()) {
            Some("block") => Some(TokenKind::Keyword),
            Some("function_call") => Some(TokenKind::Function),
            Some("attribute" | "object_elem" | "get_attr") => Some(TokenKind::Attribute),
            _ => None,
        },
        "block_start"
        | "block_end"
        | "object_start"
        | "object_end"
        | "tuple_start"
        | "tuple_end"
        | "("
        | ")"
        | ","
        | "."
        | ":"
        | "template_interpolation_start"
        | "template_interpolation_end"
        | "template_directive_start"
        | "template_directive_end" => Some(TokenKind::Punct),
        "=" | "=>" | "+" | "-" | "*" | "/" | "%" | "!" | "==" | "!=" | "<" | ">" | "<=" | ">="
        | "&&" | "||" | "?" | "..." => Some(TokenKind::Operator),
        _ => None,
    }
}

fn engine() -> LanguageSyntax {
    // Plain is a neutral toolkit routing tag, not a built-in grammar. The
    // actual language IDs, grammar, contexts and colors all belong to this package.
    let provider = SyntaxProvider {
        language: SyntaxLanguage::Plain,
        context: Some(context),
        context_scope: SyntaxContextScope::Node,
        highlight: Some(highlight),
        highlight_scope: SyntaxHighlightScope::Parent,
        injection: None,
        grammar: || tree_sitter_hcl::LANGUAGE.into(),
        fold_nodes: &["block", "object", "tuple", "heredoc_template", "comment"],
        parent_headers: &[],
    };
    let definitions = definitions();
    let grammars: BTreeMap<_, _> = definitions
        .iter()
        .map(|definition| {
            (
                definition.id.clone(),
                LanguageGrammar {
                    language: SyntaxLanguage::Plain,
                    provider: Some(provider),
                },
            )
        })
        .collect();
    LanguageSyntax::new_delegated(definitions, grammars).unwrap()
}

thread_local! { static SYNTAX: RefCell<LanguageSyntax> = RefCell::new(engine()); }

fn handle(engine: &mut LanguageSyntax, input: LanguageInput) -> Result<LanguageOutput, String> {
    engine.handle_with(input, recovery::literals)
}

impl Plugin for Terraform {
    fn languages() -> Vec<LanguageDefinition> {
        definitions()
    }
    fn language(input: LanguageInput) -> Result<LanguageOutput, String> {
        SYNTAX.with(|engine| handle(&mut engine.borrow_mut(), input))
    }
}
openwebide_plugin_sdk::export!(Terraform);

#[cfg(test)]
mod tests;
