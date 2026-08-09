//! Fatal audit for retired M4 compatibility and bypass authorities.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use super::Finding;

#[derive(Clone, Copy)]
enum DeclarationScope {
    Any,
    Public,
}

#[derive(Clone, Copy)]
struct RetiredAuthority {
    code: &'static str,
    symbol: &'static str,
    scope: DeclarationScope,
}

const RETIRED_AUTHORITIES: &[RetiredAuthority] = &[
    RetiredAuthority {
        code: "source.retired_runtime_identity",
        symbol: "RuntimeNodeId",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_runtime_identity",
        symbol: "RuntimeNodeRef",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_runtime_identity",
        symbol: "RuntimeTreeIndex",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_widget_state",
        symbol: "WidgetState",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_widget_state",
        symbol: "WidgetStateMismatch",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_lifecycle_authority",
        symbol: "WidgetLifecycle",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_lifecycle_authority",
        symbol: "WidgetLifecycleRequest",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_activation_authority",
        symbol: "ActivationResult",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_input_authority",
        symbol: "InputIntent",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_input_authority",
        symbol: "InputEvent",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_input_authority",
        symbol: "PointerFocusResult",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_focus_authority",
        symbol: "FocusTargetResult",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_focus_authority",
        symbol: "KeyboardFocusResult",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        code: "source.retired_direct_dispatch",
        symbol: "dispatch",
        scope: DeclarationScope::Public,
    },
    RetiredAuthority {
        code: "source.retired_button_action",
        symbol: "on_press",
        scope: DeclarationScope::Public,
    },
    RetiredAuthority {
        code: "source.retired_pointer_helper",
        symbol: "resolve_pointer_event_target",
        scope: DeclarationScope::Public,
    },
    RetiredAuthority {
        code: "source.retired_focus_helper",
        symbol: "handle_keyboard_focus",
        scope: DeclarationScope::Public,
    },
    RetiredAuthority {
        code: "source.retired_authored_lookup",
        symbol: "node_by_authored_id",
        scope: DeclarationScope::Public,
    },
];

pub(super) fn audit(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let mut files = Vec::new();
    collect_production_rust_files(&root.join("crates"), root, &mut files)?;
    files.sort();

    for relative in files {
        let path = root.join(&relative);
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", relative.display()))?;
        for occurrence in retired_occurrences(&contents) {
            let Some(specification) = RETIRED_AUTHORITIES
                .iter()
                .find(|specification| specification.symbol == occurrence.symbol)
            else {
                continue;
            };
            if matches!(specification.scope, DeclarationScope::Public) && !occurrence.is_public {
                continue;
            }
            findings.push(Finding::fatal(
                specification.code,
                Some(relative.to_string_lossy().into_owned()),
                format!(
                    "retired M4 authority `{}` is declared or re-exported at line {}; the accepted public API migration removes this compatibility/bypass surface",
                    occurrence.symbol, occurrence.line
                ),
            ));
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Occurrence<'a> {
    symbol: &'a str,
    line: usize,
    is_public: bool,
}

fn retired_occurrences(contents: &str) -> Vec<Occurrence<'_>> {
    let mut occurrences = Vec::new();
    let mut public_use = None::<(usize, String)>;

    for (index, source_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let line = source_line.trim();

        if let Some((start_line, buffer)) = public_use.as_mut() {
            buffer.push(' ');
            buffer.push_str(line);
            if line.contains(';') {
                collect_reexport_occurrences(*start_line, buffer, &mut occurrences);
                public_use = None;
            }
            continue;
        }

        if is_public_use_start(line) {
            let mut buffer = line.to_owned();
            if line.contains(';') {
                collect_reexport_occurrences(line_number, &buffer, &mut occurrences);
            } else {
                public_use = Some((line_number, std::mem::take(&mut buffer)));
            }
            continue;
        }

        let Some((symbol, is_public)) = declaration_symbol(line) else {
            continue;
        };
        if RETIRED_AUTHORITIES
            .iter()
            .any(|specification| specification.symbol == symbol)
        {
            occurrences.push(Occurrence {
                symbol,
                line: line_number,
                is_public,
            });
        }
    }

    occurrences
}

fn collect_reexport_occurrences<'a>(
    line: usize,
    statement: &'a str,
    occurrences: &mut Vec<Occurrence<'a>>,
) {
    for symbol in RETIRED_AUTHORITIES
        .iter()
        .map(|specification| specification.symbol)
    {
        if identifiers(statement).any(|identifier| identifier == symbol) {
            occurrences.push(Occurrence {
                symbol,
                line,
                is_public: true,
            });
        }
    }
}

fn declaration_symbol(line: &str) -> Option<(&str, bool)> {
    if line.is_empty() || line.starts_with("//") || line.starts_with("#") {
        return None;
    }

    let is_public = line.starts_with("pub ");
    let mut words = line.split_whitespace();
    let first = words.next()?;
    let (keyword, symbol) = if first == "pub" {
        declaration_after_visibility(&mut words)?
    } else if first.starts_with("pub(") {
        declaration_after_visibility(&mut words)?
    } else {
        declaration_after_keyword(first, &mut words)?
    };

    let symbol = clean_identifier(symbol);
    (!symbol.is_empty() && is_declaration_keyword(keyword)).then_some((symbol, is_public))
}

fn declaration_after_visibility<'a>(
    words: &mut impl Iterator<Item = &'a str>,
) -> Option<(&'a str, &'a str)> {
    let first = words.next()?;
    if first == "async" || first == "const" || first == "unsafe" {
        let keyword = words.next()?;
        let symbol = words.next()?;
        Some((keyword, symbol))
    } else {
        declaration_after_keyword(first, words)
    }
}

fn declaration_after_keyword<'a>(
    keyword: &'a str,
    words: &mut impl Iterator<Item = &'a str>,
) -> Option<(&'a str, &'a str)> {
    if keyword == "async" || keyword == "const" || keyword == "unsafe" {
        let keyword = words.next()?;
        let symbol = words.next()?;
        Some((keyword, symbol))
    } else {
        Some((keyword, words.next()?))
    }
}

fn is_declaration_keyword(value: &str) -> bool {
    matches!(
        value,
        "struct" | "enum" | "type" | "trait" | "fn" | "mod" | "static" | "const"
    )
}

fn clean_identifier(value: &str) -> &str {
    value.trim_matches(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
}

fn is_public_use_start(line: &str) -> bool {
    line.starts_with("pub use ")
        || line.starts_with("pub(crate) use ")
        || line.starts_with("pub(super) use ")
}

fn identifiers(statement: &str) -> impl Iterator<Item = &str> {
    statement
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter(|token| !token.is_empty())
}

fn collect_production_rust_files(
    directory: &Path,
    root: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?
    {
        let entry = entry
            .map_err(|error| format!("failed to inspect {} entry: {error}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_production_rust_files(&path, root, files)?;
            continue;
        }
        if path.extension() == Some(OsStr::new("rs"))
            && path.components().any(|component| component.as_os_str() == "src")
        {
            files.push(path.strip_prefix(root).map_err(|error| error.to_string())?.to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Occurrence, retired_occurrences};

    #[test]
    fn declarations_and_multiline_reexports_are_detected() {
        let source = r#"
struct InputIntent;
pub struct RuntimeNodeId;
pub fn dispatch(&mut self) {}
pub use crate::{
    FocusTargetResult,
    SomethingCurrent,
};
"#;
        let occurrences = retired_occurrences(source);
        assert!(occurrences.iter().any(|occurrence| occurrence.symbol == "InputIntent"));
        assert!(occurrences.iter().any(|occurrence| occurrence.symbol == "RuntimeNodeId"));
        assert!(occurrences.iter().any(|occurrence| occurrence.symbol == "dispatch"));
        assert!(occurrences.iter().any(|occurrence| occurrence.symbol == "FocusTargetResult"));
    }

    #[test]
    fn comments_strings_calls_and_prefixed_names_do_not_count_as_authority() {
        let source = r#"
// pub struct RuntimeNodeId;
const MESSAGE: &str = "pub fn dispatch";
fn current() {
    runtime.dispatch_internal();
}
struct InputIntentional;
"#;
        assert_eq!(retired_occurrences(source), Vec::<Occurrence<'_>>::new());
    }

    #[test]
    fn private_ambiguous_method_is_distinguished_from_public_compatibility_surface() {
        let source = "fn dispatch() {}\npub(crate) fn dispatch() {}\npub fn dispatch() {}\n";
        let occurrences = retired_occurrences(source);
        assert_eq!(occurrences.len(), 3);
        assert!(!occurrences[0].is_public);
        assert!(!occurrences[1].is_public);
        assert!(occurrences[2].is_public);
    }
}
