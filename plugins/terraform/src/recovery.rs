//! HCL's error tree retains literal-opening nodes even when the surrounding
//! template is unavailable. Keep their unfinished bodies opaque while typing.
use openwebide_plugin_sdk::languages::*;

pub fn literals(source: &str, _: &str, data: &mut SyntaxAnalysis) -> Result<(), String> {
    let Some(structure) = &data.structure else {
        return Ok(());
    };
    let candidates: Vec<_> = structure
        .selections
        .iter()
        .copied()
        .filter(|range| {
            matches!(
                source.get(range.start..range.end),
                Some("\"" | "<<" | "<<-")
            )
        })
        .collect();
    for opening in candidates {
        let structure = data.structure.as_mut().unwrap();
        if structure
            .protected
            .iter()
            .any(|region| region.range.start <= opening.start && opening.start < region.range.end)
        {
            continue;
        }
        let quote = source.get(opening.start..opening.end) == Some("\"");
        let end = if quote {
            // The parser has no trustworthy template here. Keep this line
            // opaque until its grammar can resolve interpolation and quoting.
            source[opening.end..]
                .find('\n')
                .map_or(source.len(), |end| opening.end + end)
        } else {
            heredoc_end(source, opening)
        };
        let range = TextRange {
            start: opening.start,
            end,
        };
        let mut retained = Vec::new();
        for region in structure.protected.drain(..) {
            if region.range.end <= range.start || region.range.start >= range.end {
                retained.push(region);
            } else {
                if region.range.start < range.start {
                    retained.push(ProtectedRegion {
                        range: TextRange {
                            start: region.range.start,
                            end: range.start,
                        },
                        complete: false,
                        kind: region.kind,
                    });
                }
                if region.range.end > range.end {
                    retained.push(ProtectedRegion {
                        range: TextRange {
                            start: range.end,
                            end: region.range.end,
                        },
                        ..region
                    });
                }
            }
        }
        retained.push(ProtectedRegion {
            range,
            complete: false,
            kind: RegionKind::Template,
        });
        retained.sort_by_key(|region| region.range.start);
        structure.protected = retained;
        structure.opaque_starts.retain(|start| {
            structure
                .protected
                .iter()
                .any(|region| region.range.start == *start)
        });
        if let Some(rows) = &mut data.highlights {
            paint(source, rows, range);
        }
    }
    Ok(())
}

fn heredoc_end(source: &str, opening: TextRange) -> usize {
    let Some(newline) = source[opening.end..]
        .find('\n')
        .map(|offset| opening.end + offset)
    else {
        return source.len();
    };
    let label = source[opening.end..newline].trim();
    if label.is_empty() {
        return source.len();
    }
    let mut offset = newline + 1;
    for line in source[offset..].split_inclusive('\n') {
        if line.trim() == label {
            return offset + line.trim_end_matches(['\r', '\n']).len();
        }
        offset += line.len();
    }
    source.len()
}

fn paint(source: &str, rows: &mut [Vec<TokenSpan>], range: TextRange) {
    let mut offset = 0;
    for (line, row) in source.split('\n').zip(rows) {
        let start = range.start.saturating_sub(offset).min(line.len());
        let end = range.end.saturating_sub(offset).min(line.len());
        if start < end {
            let mut next: Vec<TokenSpan> = Vec::new();
            let mut before = 0;
            for span in row.iter() {
                for boundary in [
                    start.clamp(before, span.end),
                    end.clamp(before, span.end),
                    span.end,
                ] {
                    if boundary <= before {
                        continue;
                    }
                    let kind = if before >= start && before < end {
                        TokenKind::String
                    } else {
                        span.kind
                    };
                    if let Some(last) = next.last_mut()
                        && last.kind == kind
                    {
                        last.end = boundary;
                    } else {
                        next.push(TokenSpan {
                            end: boundary,
                            kind,
                        });
                    }
                    before = boundary;
                }
            }
            *row = next;
        }
        offset += line.len() + 1;
    }
}
