use super::*;

fn request(language: &str, source: &str, revision: u64) -> AnalysisRequest {
    AnalysisRequest {
        version: VERSION,
        document: format!("fixture/{language}"),
        request: revision,
        revision,
        language: language.into(),
        path: "main.tf".into(),
        source_digest: openwebide_core::plugins::languages::source_digest(source),
        source: SourceChange::Full {
            text: source.into(),
        },
        tab_width: 4,
    }
}
fn analyze(engine: &mut LanguageSyntax, input: AnalysisRequest) -> AnalysisResponse {
    let LanguageOutput::Analyzed(response) = handle(engine, LanguageInput::Analyze(input)).unwrap()
    else {
        panic!("analysis required")
    };
    assert!(
        matches!(response.status, AnalysisStatus::Ready { .. }),
        "{:?}",
        response.status
    );
    *response
}

#[test]
fn independent_grammar_covers_all_declared_dialects_with_exact_unicode_crlf_rows() {
    let source = "variable \"label\" {\r\n  default = \"文😀 }\" # { comment\r\n}\r\nlocals {\r\n  answer = 42\r\n  enabled = true\r\n}\r\n";
    let mut engine = engine();
    for definition in definitions() {
        let response = analyze(&mut engine, request(&definition.id, source, 1));
        let data = response.analysis.unwrap();
        openwebide_core::plugins::languages::validate_analysis(&data, source, &definition.id)
            .unwrap();
        let rows = data.highlights.unwrap();
        assert_eq!(rows.len(), source.split('\n').count());
        for (row, line) in rows.iter().zip(source.split('\n')) {
            assert_eq!(row.last().map_or(0, |span| span.end), line.len());
            assert!(row.iter().all(|span| line.is_char_boundary(span.end)));
        }
        assert!(rows[0].iter().any(|span| span.kind == TokenKind::Keyword));
        assert!(rows[1].iter().any(|span| span.kind == TokenKind::String));
        assert!(rows[1].iter().any(|span| span.kind == TokenKind::Comment));
        assert!(rows[4].iter().any(|span| span.kind == TokenKind::Number));
        assert!(rows[5].iter().any(|span| span.kind == TokenKind::Boolean));
        assert!(
            data.folds
                .iter()
                .any(|fold| fold.start_line == 0 && fold.end_line == 2)
        );
        let structure = data.structure.unwrap();
        assert_eq!(structure.language, definition.id);
        assert_eq!(
            structure.brackets.len(),
            4,
            "literal/comment braces must remain opaque"
        );
        assert!(
            structure
                .brackets
                .iter()
                .all(|bracket| bracket.partner.is_some())
        );
    }
}

#[test]
fn unfinished_literals_keep_braces_out_of_structural_commands() {
    for source in [
        "locals {\r\n  text = \"文😀unfinished {",
        "locals {\r\n  text = <<EOT\r\n文😀unfinished {",
    ] {
        let mut engine = engine();
        let data = analyze(&mut engine, request("terraform", source, 1))
            .analysis
            .unwrap();
        openwebide_core::plugins::languages::validate_analysis(&data, source, "terraform").unwrap();
        let literal = source.find("unfinished").unwrap();
        let structure = data.structure.unwrap();
        assert!(
            structure
                .protected
                .iter()
                .any(|region| region.range.start <= literal && literal < region.range.end),
            "unfinished literal must remain opaque: {structure:?}"
        );
        assert_eq!(
            structure.brackets.len(),
            1,
            "only the actual locals block opens a structural bracket"
        );
    }
}

#[test]
fn heredoc_literals_and_template_expressions_have_distinct_editing_regions() {
    let source = "locals {\n  text = <<-EOT\n    文😀 } ${upper(\"world\")}\n    %{ if true }yes%{ else }no%{ endif }\n  EOT\n}\n";
    let mut engine = engine();
    let data = analyze(&mut engine, request("terraform", source, 1))
        .analysis
        .unwrap();
    let structure = data.structure.unwrap();
    let literal = source.find("文😀").unwrap();
    assert!(
        structure
            .protected
            .iter()
            .any(|region| region.range.start <= literal && literal < region.range.end)
    );
    let expression = source.find("upper").unwrap();
    assert!(
        !structure
            .protected
            .iter()
            .any(|region| region.range.start <= expression && expression < region.range.end)
    );
    assert!(
        data.highlights.unwrap()[2]
            .iter()
            .any(|span| span.kind == TokenKind::Function)
    );
    assert!(
        data.folds
            .iter()
            .any(|fold| fold.start_line == 1 && fold.end_line == 4)
    );
}

#[test]
fn incremental_replacements_match_fresh_parse_and_close_requires_resynchronization() {
    let source = "locals {\r\n  text = \"文😀\"\r\n}\r\n";
    let mut engine = engine();
    let mut input = request("terraform", source, 1);
    analyze(&mut engine, input.clone());
    let start = source.find("文😀").unwrap();
    input.revision = 2;
    input.request = 2;
    input.source = SourceChange::Replace {
        base_revision: 1,
        start,
        end: start + "文😀".len(),
        text: "updated".into(),
    };
    let updated = source.replace("文😀", "updated");
    input.source_digest = openwebide_core::plugins::languages::source_digest(&updated);
    let changed = analyze(&mut engine, input.clone());
    let fresh = analyze(&mut super::engine(), request("terraform", &updated, 2));
    assert_eq!(changed.analysis, fresh.analysis);
    handle(
        &mut engine,
        LanguageInput::Close {
            version: VERSION,
            document: input.document.clone(),
        },
    )
    .unwrap();
    let LanguageOutput::Analyzed(response) =
        handle(&mut engine, LanguageInput::Analyze(input)).unwrap()
    else {
        panic!("analysis required")
    };
    assert_eq!(response.status, AnalysisStatus::NeedsSource);
}
