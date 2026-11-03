use openwebide_plugin_sdk::{Plugin, languages::*, serde_json};
use std::{cell::RefCell, collections::BTreeMap};

struct Notes;
#[derive(Default)]
struct Sources {
    documents: BTreeMap<String, (u64, String, u64)>,
    tick: u64,
}
thread_local! { static SOURCES: RefCell<Sources> = RefCell::new(Sources::default()); }

impl Plugin for Notes {
    fn languages() -> Vec<LanguageDefinition> {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../plugin.json")).unwrap();
        serde_json::from_value(manifest["contributions"]["languages"].clone()).unwrap()
    }

    fn language(input: LanguageInput) -> Result<LanguageOutput, String> {
        match input {
            LanguageInput::Close { version, document } => {
                if version != VERSION {
                    return Err("Unsupported language protocol".into());
                }
                SOURCES.with(|cache| {
                    cache.borrow_mut().documents.remove(&document);
                });
                Ok(LanguageOutput::Closed { version, document })
            }
            LanguageInput::Analyze(request) => analyze(request),
        }
    }
}

fn response(
    request: &AnalysisRequest,
    status: AnalysisStatus,
    analysis: Option<SyntaxAnalysis>,
) -> LanguageOutput {
    LanguageOutput::Analyzed(Box::new(AnalysisResponse {
        version: VERSION,
        document: request.document.clone(),
        request: request.request,
        revision: request.revision,
        language: request.language.clone(),
        source_digest: request.source_digest.clone(),
        status,
        analysis,
    }))
}

fn source(request: &AnalysisRequest) -> Result<Option<String>, String> {
    SOURCES.with(|cache| {
        let cache = cache.borrow();
        let previous = cache.documents.get(&request.document);
        if previous.is_some_and(|(revision, _, _)| *revision > request.revision) {
            return Err("Document revision is stale".into());
        }
        match &request.source {
            SourceChange::Full { text } => Ok(Some(text.clone())),
            SourceChange::Replace {
                base_revision,
                start,
                end,
                text,
            } => {
                let Some((revision, previous, _)) = previous else {
                    return Ok(None);
                };
                if revision != base_revision {
                    return Ok(None);
                }
                if start > end || previous.get(*start..*end).is_none() {
                    return Err("Invalid UTF-8 source replacement".into());
                }
                let mut source = previous.clone();
                source.replace_range(*start..*end, text);
                Ok(Some(source))
            }
        }
    })
}

fn remember(request: &AnalysisRequest, source: String) {
    SOURCES.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.tick = cache.tick.wrapping_add(1);
        let tick = cache.tick;
        cache
            .documents
            .insert(request.document.clone(), (request.revision, source, tick));
        while cache.documents.len() > 16
            || cache
                .documents
                .values()
                .map(|(_, source, _)| source.len())
                .sum::<usize>()
                > 8 * 1024 * 1024
        {
            let oldest = cache
                .documents
                .iter()
                .min_by_key(|(_, (_, _, tick))| tick)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                cache.documents.remove(&oldest);
            } else {
                break;
            }
        }
    });
}

fn analyze(request: AnalysisRequest) -> Result<LanguageOutput, String> {
    if request.version != VERSION || request.document.is_empty() || request.document.len() > 4096 {
        return Err("Invalid language request".into());
    }
    if request.language != "notes" {
        return Ok(response(&request, AnalysisStatus::Unsupported, None));
    }
    let Some(source) = source(&request)? else {
        return Ok(response(&request, AnalysisStatus::NeedsSource, None));
    };
    if source.len() > MAX_SOURCE_BYTES {
        return Ok(response(&request, AnalysisStatus::TooLarge, None));
    }
    let Some(analysis) = scan(&source) else {
        return Ok(response(&request, AnalysisStatus::TooLarge, None));
    };
    remember(&request, source);
    Ok(response(
        &request,
        AnalysisStatus::Ready { incremental: false },
        Some(analysis),
    ))
}

fn token(row: &mut Vec<TokenSpan>, end: usize, kind: TokenKind) -> usize {
    if let Some(last) = row.last_mut()
        && last.kind == kind
    {
        last.end = end;
        return 0;
    }
    row.push(TokenSpan { end, kind });
    1
}

fn scan(source: &str) -> Option<SyntaxAnalysis> {
    let mut highlights = Vec::new();
    let mut folds = Vec::new();
    let mut brackets = Vec::<Bracket>::new();
    let mut protected = Vec::new();
    let mut selections = Vec::new();
    let mut stack = Vec::<(usize, usize)>::new();
    let mut offset = 0;
    let mut records = 0;
    for (line_number, line) in source.split('\n').enumerate() {
        let mut row = Vec::new();
        let mut cursor = 0;
        while cursor < line.len() {
            let start = cursor;
            let ch = line[cursor..].chars().next()?;
            let kind = if ch == '#' {
                cursor = line.len();
                protected.push(ProtectedRegion {
                    range: TextRange {
                        start: offset + start,
                        end: offset + cursor,
                    },
                    complete: offset + cursor < source.len(),
                    kind: RegionKind::LineComment,
                });
                TokenKind::Comment
            } else if ch == '"' {
                cursor += 1;
                let mut complete = false;
                while cursor < line.len() {
                    let next = line[cursor..].chars().next()?;
                    cursor += next.len_utf8();
                    if next == '\\' {
                        if let Some(escaped) = line[cursor..].chars().next() {
                            cursor += escaped.len_utf8();
                        }
                    } else if next == '"' {
                        complete = true;
                        break;
                    }
                }
                protected.push(ProtectedRegion {
                    range: TextRange {
                        start: offset + start,
                        end: offset + cursor,
                    },
                    complete,
                    kind: RegionKind::String,
                });
                TokenKind::String
            } else if ch.is_alphanumeric() || ch == '_' {
                cursor += ch.len_utf8();
                while let Some(next) = line[cursor..].chars().next() {
                    if !next.is_alphanumeric() && next != '_' {
                        break;
                    }
                    cursor += next.len_utf8();
                }
                match &line[start..cursor] {
                    "note" | "task" => TokenKind::Keyword,
                    value if value.bytes().all(|ch| ch.is_ascii_digit()) => TokenKind::Number,
                    _ => TokenKind::Plain,
                }
            } else {
                cursor += ch.len_utf8();
                if matches!(ch, '{' | '}') {
                    let index = brackets.len();
                    brackets.push(Bracket {
                        offset: offset + start,
                        character: ch,
                        partner: None,
                    });
                    if ch == '{' {
                        stack.push((index, line_number));
                    } else if let Some((opening, first_line)) = stack.pop() {
                        brackets[opening].partner = Some(offset + start);
                        brackets[index].partner = Some(brackets[opening].offset);
                        selections.push(TextRange {
                            start: brackets[opening].offset,
                            end: offset + cursor,
                        });
                        if first_line < line_number {
                            folds.push(FoldRange {
                                start_line: first_line,
                                end_line: line_number,
                            });
                        }
                    }
                    TokenKind::Punct
                } else {
                    TokenKind::Plain
                }
            };
            records += token(&mut row, cursor, kind);
            if records + brackets.len() + protected.len() + selections.len() + folds.len()
                > MAX_RECORDS
            {
                return None;
            }
        }
        highlights.push(row);
        offset += line.len() + 1;
        if highlights.len() > MAX_RECORDS {
            return None;
        }
    }
    folds.sort_by_key(|fold| (fold.start_line, std::cmp::Reverse(fold.end_line)));
    folds.dedup_by_key(|fold| fold.start_line);
    selections.sort_by_key(|range| (range.start, range.end));
    Some(SyntaxAnalysis {
        highlights: Some(highlights),
        folds,
        structure: Some(Structure {
            language: "notes".into(),
            scopes: vec![],
            selections,
            opaque_starts: vec![],
            protected,
            brackets,
        }),
    })
}

openwebide_plugin_sdk::export!(Notes);

#[cfg(test)]
mod tests {
    use super::*;
    fn request(source: SourceChange, revision: u64) -> AnalysisRequest {
        AnalysisRequest {
            version: VERSION,
            document: "fixture".into(),
            request: revision,
            revision,
            language: "notes".into(),
            path: "file.notes".into(),
            source_digest: "a".repeat(64),
            source,
            tab_width: 4,
        }
    }
    fn result(input: AnalysisRequest) -> AnalysisResponse {
        let LanguageOutput::Analyzed(output) =
            Notes::language(LanguageInput::Analyze(input)).unwrap()
        else {
            panic!("analysis response required");
        };
        *output
    }
    #[test]
    fn syntax_keeps_unicode_crlf_offsets_and_ignores_braces_inside_literals() {
        let source = "note {\r\n  task \"文😀}\" # { comment\r\n}\r\n";
        let output = result(request(
            SourceChange::Full {
                text: source.into(),
            },
            1,
        ));
        assert_eq!(output.status, AnalysisStatus::Ready { incremental: false });
        let syntax = output.analysis.unwrap();
        let rows = syntax.highlights.unwrap();
        for (row, line) in rows.iter().zip(source.split('\n')) {
            assert_eq!(row.last().map_or(0, |span| span.end), line.len());
            assert!(row.iter().all(|span| line.is_char_boundary(span.end)));
        }
        assert!(rows[1].iter().any(|span| span.kind == TokenKind::String));
        assert!(rows[1].iter().any(|span| span.kind == TokenKind::Comment));
        assert_eq!(
            syntax.folds,
            vec![FoldRange {
                start_line: 0,
                end_line: 2
            }]
        );
        let structure = syntax.structure.unwrap();
        assert_eq!(structure.brackets.len(), 2);
        assert_eq!(
            structure.brackets[0].partner,
            Some(source.rfind('}').unwrap())
        );
    }
    #[test]
    fn source_replacements_require_a_retained_revision_and_utf8_boundaries() {
        let mut full = request(
            SourceChange::Full {
                text: "文😀\r\n".into(),
            },
            1,
        );
        full.document = "replace".into();
        result(full.clone());
        full.revision = 2;
        full.source = SourceChange::Replace {
            base_revision: 1,
            start: 1,
            end: 2,
            text: "x".into(),
        };
        assert!(Notes::language(LanguageInput::Analyze(full.clone())).is_err());
        full.source = SourceChange::Replace {
            base_revision: 1,
            start: 0,
            end: 3,
            text: "note".into(),
        };
        assert!(matches!(
            result(full.clone()).status,
            AnalysisStatus::Ready { .. }
        ));
        full.revision = 3;
        full.source = SourceChange::Replace {
            base_revision: 1,
            start: 0,
            end: 0,
            text: "task".into(),
        };
        assert_eq!(result(full.clone()).status, AnalysisStatus::NeedsSource);
        Notes::language(LanguageInput::Close {
            version: VERSION,
            document: full.document.clone(),
        })
        .unwrap();
        full.source = SourceChange::Replace {
            base_revision: 2,
            start: 0,
            end: 0,
            text: "task".into(),
        };
        assert_eq!(result(full).status, AnalysisStatus::NeedsSource);
    }
}
