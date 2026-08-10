use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy)]
enum DeclarationScope {
    Any,
    Public,
}

#[derive(Clone, Copy)]
struct RetiredAuthority {
    symbol: &'static str,
    scope: DeclarationScope,
}

const RETIRED_AUTHORITIES: &[RetiredAuthority] = &[
    RetiredAuthority {
        symbol: "RuntimeNodeId",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "RuntimeNodeRef",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "RuntimeTreeIndex",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "WidgetState",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "WidgetStateMismatch",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "WidgetLifecycle",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "WidgetLifecycleRequest",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "ActivationResult",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "InputIntent",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "InputEvent",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "PointerFocusResult",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "FocusTargetResult",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "KeyboardFocusResult",
        scope: DeclarationScope::Any,
    },
    RetiredAuthority {
        symbol: "dispatch",
        scope: DeclarationScope::Public,
    },
    RetiredAuthority {
        symbol: "on_press",
        scope: DeclarationScope::Public,
    },
    RetiredAuthority {
        symbol: "resolve_pointer_event_target",
        scope: DeclarationScope::Public,
    },
    RetiredAuthority {
        symbol: "handle_keyboard_focus",
        scope: DeclarationScope::Public,
    },
    RetiredAuthority {
        symbol: "node_by_authored_id",
        scope: DeclarationScope::Public,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Occurrence<'a> {
    symbol: &'a str,
    line: usize,
    is_public: bool,
}

#[test]
fn accepted_m4_retired_authorities_are_absent_from_production_sources() -> Result<(), String> {
    let root = workspace_root()?;
    let mut files = Vec::new();
    collect_production_rust_files(&root.join("crates"), &root, &mut files)?;
    files.sort();

    let mut failures = Vec::new();
    for relative in files {
        let contents = fs::read_to_string(root.join(&relative))
            .map_err(|error| format!("failed to read {}: {error}", relative.display()))?;
        for occurrence in retired_occurrences(&contents) {
            let specification = RETIRED_AUTHORITIES
                .iter()
                .find(|specification| specification.symbol == occurrence.symbol)
                .unwrap_or_else(|| unreachable!("scanner emits only audited symbols"));
            if matches!(specification.scope, DeclarationScope::Public) && !occurrence.is_public {
                continue;
            }
            failures.push(format!(
                "{}:{} declares or publicly re-exports retired M4 authority `{}`",
                relative.display(),
                occurrence.line,
                occurrence.symbol
            ));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "retired M4 compatibility/bypass authorities remain in production source:\n{}",
            failures.join("\n")
        ))
    }
}

fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest directory has no workspace parent".to_owned())
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
                collect_public_reexport_occurrences(*start_line, buffer, &mut occurrences);
                public_use = None;
            }
            continue;
        }

        if line.starts_with("pub use ") {
            if line.contains(';') {
                collect_public_reexport_occurrences(line_number, line, &mut occurrences);
            } else {
                public_use = Some((line_number, line.to_owned()));
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

fn collect_public_reexport_occurrences<'a>(
    line: usize,
    statement: &str,
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
    if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
        return None;
    }

    let is_public = line.starts_with("pub ");
    let mut words = line.split_whitespace();
    let first = words.next()?;
    let (keyword, symbol) = if first == "pub" || first.starts_with("pub(") {
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
    declaration_after_keyword(words.next()?, words)
}

fn declaration_after_keyword<'a>(
    keyword: &'a str,
    words: &mut impl Iterator<Item = &'a str>,
) -> Option<(&'a str, &'a str)> {
    if matches!(keyword, "async" | "const" | "unsafe") {
        Some((words.next()?, words.next()?))
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
            files.push(
                path.strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod scanner_tests {
    use super::{Occurrence, retired_occurrences};

    #[test]
    fn detects_declarations_and_external_public_reexports() {
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
        assert!(occurrences.iter().any(|item| item.symbol == "InputIntent"));
        assert!(occurrences.iter().any(|item| item.symbol == "RuntimeNodeId"));
        assert!(occurrences.iter().any(|item| item.symbol == "dispatch"));
        assert!(occurrences.iter().any(|item| item.symbol == "FocusTargetResult"));
    }

    #[test]
    fn ignores_comments_strings_calls_prefixed_names_and_crate_private_ambiguous_methods() {
        let source = r#"
// pub struct RuntimeNodeId;
const MESSAGE: &str = "pub fn dispatch";
fn current() { runtime.dispatch_internal(); }
struct InputIntentional;
pub(crate) fn dispatch() {}
"#;
        let occurrences = retired_occurrences(source);
        assert_eq!(
            occurrences,
            [Occurrence {
                symbol: "dispatch",
                line: 6,
                is_public: false,
            }]
        );
    }
}
