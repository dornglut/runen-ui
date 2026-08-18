use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use super::{Finding, path_text};

const LARGE_SOURCE_LINES: usize = 900;
const HIGH_PUBLIC_ITEMS: usize = 40;
const BROAD_REEXPORTS: usize = 10;
const HIGH_TEST_LINES: usize = 800;
const HIGH_TEST_COUNT: usize = 20;
const MULTI_RESPONSIBILITY_COUNT: usize = 5;
const SURFACE_PUBLICATION_ENTRYPOINT: &str = "publish_surface";
const SURFACE_PUBLICATION_ENTRYPOINT_PATH: &str = "crates/runenui_runtime/src/app/surface.rs";

const RESPONSIBILITY_TERMS: &[&str] = &[
    "application",
    "command",
    "completion",
    "focus",
    "identity",
    "input",
    "layout",
    "lifecycle",
    "mounted",
    "queue",
    "reconcile",
    "scheduler",
    "style",
    "subscription",
    "surface",
    "timer",
    "trace",
    "wake",
    "work",
];

const VOLATILE_ARCHITECTURE_PATTERNS: &[&str] = &[
    "Current head:",
    "Draft PR:",
    "Exact accepted base SHA",
    "codex/",
    "governance/",
    "tooling/",
    "https://github.com/Crystonix/runen-ui/pull/",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct SourceMetrics {
    pub(super) production_modules: usize,
    pub(super) test_modules: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ModuleMetrics {
    relative: PathBuf,
    lines: usize,
    public_items: usize,
    reexports: usize,
    tests: usize,
    responsibilities: BTreeSet<&'static str>,
}

#[derive(Clone, Copy)]
struct AuthoritySpec {
    code: &'static str,
    symbol: &'static str,
    expected_path: &'static str,
}

const AUTHORITIES: &[AuthoritySpec] = &[
    AuthoritySpec {
        code: "source.canonical_queue_authority",
        symbol: "WorkQueue",
        expected_path: "crates/runenui_runtime/src/queue.rs",
    },
    AuthoritySpec {
        code: "source.canonical_trace_store_authority",
        symbol: "Trace",
        expected_path: "crates/runenui_runtime/src/trace/store.rs",
    },
    AuthoritySpec {
        code: "source.surface_publication_authority",
        symbol: "SurfacePublicationState",
        expected_path: "crates/runenui_runtime/src/runtime/surface_publication.rs",
    },
];

#[derive(Clone, Copy)]
enum RetiredAuthorityScope {
    AnyDeclaration,
    ExternallyPublicDeclaration,
    PublicReexportOnly,
}

#[derive(Clone, Copy)]
struct RetiredAuthoritySpec {
    symbol: &'static str,
    scope: RetiredAuthorityScope,
}

const RETIRED_AUTHORITIES: &[RetiredAuthoritySpec] = &[
    RetiredAuthoritySpec {
        symbol: "RuntimeNodeId",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "RuntimeNodeRef",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "RuntimeTreeIndex",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "WidgetState",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "WidgetStateMismatch",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "WidgetLifecycle",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "WidgetLifecycleRequest",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "ActivationCapacity",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "ActivationCommit",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "ActivationResult",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "PointerActivationResult",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "KeyboardActivationResult",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "InputEventResult",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "InputIntent",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "InputEvent",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "PointerFocusResult",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "FocusTargetResult",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "KeyboardFocusResult",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "dispatch",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "activate_node",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "handle_pointer_activation",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "handle_keyboard_activation",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "handle_input_event",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "on_press",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "resolve_pointer_event_target",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "set_focus",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "focus_first",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "focus_last",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "focus_next",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "focus_previous",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "handle_keyboard_focus",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "node_by_authored_id",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: SURFACE_PUBLICATION_ENTRYPOINT,
        scope: RetiredAuthorityScope::PublicReexportOnly,
    },
];

const RETIRED_M5_AUTHORITIES: &[RetiredAuthoritySpec] = &[
    RetiredAuthoritySpec {
        symbol: "WidgetSemanticProof",
        scope: RetiredAuthorityScope::AnyDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "activate_semantic",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
    RetiredAuthoritySpec {
        symbol: "mounted_node_id",
        scope: RetiredAuthorityScope::ExternallyPublicDeclaration,
    },
];

const RETIRED_M5_CODE_PATTERNS: &[&str] = &["SemanticAction::LogicalScroll"];
const SEMANTIC_ALIAS_TARGETS: &[&str] = &[
    "SemanticAction",
    "SemanticActionRequest",
    "SemanticPublication",
    "SemanticSnapshot",
    "SemanticTarget",
    "SemanticUpdate",
    "SemanticUpdateResult",
];

pub(super) fn audit(root: &Path, findings: &mut Vec<Finding>) -> Result<SourceMetrics, String> {
    let production_files = collect_rust_files(root, &root.join("crates"), FileKind::Production)?;
    let test_files = collect_test_files(root)?;

    let mut production_metrics = Vec::new();
    for relative in &production_files {
        let contents = read_rust_file(root, relative)?;
        let metrics = module_metrics(relative, &contents);
        add_production_diagnostics(&metrics, findings);
        production_metrics.push((metrics, contents));
    }

    for relative in &test_files {
        let contents = read_rust_file(root, relative)?;
        let metrics = module_metrics(relative, &contents);
        add_test_diagnostics(&metrics, findings);
    }

    audit_authority_definitions(&production_metrics, findings);
    audit_surface_publication_entrypoint(&production_metrics, findings);
    audit_retired_authorities(&production_metrics, findings);
    audit_retired_m5_authorities(&production_metrics, findings);
    audit_volatile_architecture_state(root, findings)?;

    Ok(SourceMetrics {
        production_modules: production_files.len(),
        test_modules: test_files.len(),
    })
}

fn add_production_diagnostics(metrics: &ModuleMetrics, findings: &mut Vec<Finding>) {
    let path = path_text(&metrics.relative);
    if metrics.lines >= LARGE_SOURCE_LINES {
        findings.push(Finding::diagnostic(
            "diagnostic.large_source_module",
            Some(path.clone()),
            format!(
                "module has {} lines; inspect responsibility boundaries rather than treating line count as a correctness failure",
                metrics.lines
            ),
        ));
    }
    if metrics.public_items >= HIGH_PUBLIC_ITEMS {
        findings.push(Finding::diagnostic(
            "diagnostic.public_item_concentration",
            Some(path.clone()),
            format!(
                "module contains {} public or crate-visible item declarations",
                metrics.public_items
            ),
        ));
    }
    if metrics.reexports >= BROAD_REEXPORTS {
        findings.push(Finding::diagnostic(
            "diagnostic.reexport_concentration",
            Some(path.clone()),
            format!("module contains {} `pub use` statements", metrics.reexports),
        ));
    }
    if metrics.responsibilities.len() >= MULTI_RESPONSIBILITY_COUNT {
        findings.push(Finding::diagnostic(
            "diagnostic.responsibility_concentration",
            Some(path.clone()),
            format!(
                "item declarations reference {} responsibility vocabularies: {}",
                metrics.responsibilities.len(),
                metrics
                    .responsibilities
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    if metrics.lines >= LARGE_SOURCE_LINES
        && (metrics.public_items >= 30
            || metrics.responsibilities.len() >= MULTI_RESPONSIBILITY_COUNT)
    {
        findings.push(Finding::diagnostic(
            "diagnostic.god_file_candidate",
            Some(path),
            "module crosses the composite concentration threshold; review cohesion before expanding it",
        ));
    }
}

fn add_test_diagnostics(metrics: &ModuleMetrics, findings: &mut Vec<Finding>) {
    if metrics.lines >= HIGH_TEST_LINES || metrics.tests >= HIGH_TEST_COUNT {
        findings.push(Finding::diagnostic(
            "diagnostic.test_file_concentration",
            Some(path_text(&metrics.relative)),
            format!(
                "test module contains {} lines and {} `#[test]` cases",
                metrics.lines, metrics.tests
            ),
        ));
    }
}

fn audit_authority_definitions(
    production: &[(ModuleMetrics, String)],
    findings: &mut Vec<Finding>,
) {
    for authority in AUTHORITIES {
        let mut locations = Vec::new();
        for (metrics, contents) in production {
            for (index, line) in contents.lines().enumerate() {
                if defines_struct(line, authority.symbol) {
                    locations.push(format!("{}:{}", path_text(&metrics.relative), index + 1));
                }
            }
        }

        match locations.as_slice() {
            [location] if location.starts_with(authority.expected_path) => {}
            [location] => findings.push(Finding::fatal(
                authority.code,
                Some(location.clone()),
                format!(
                    "canonical `{}` authority must be defined in {}",
                    authority.symbol, authority.expected_path
                ),
            )),
            [] => findings.push(Finding::fatal(
                authority.code,
                Some(authority.expected_path.to_owned()),
                format!(
                    "canonical `{}` authority definition is missing",
                    authority.symbol
                ),
            )),
            _ => findings.push(Finding::fatal(
                authority.code,
                None::<String>,
                format!(
                    "canonical `{}` authority is defined multiple times: {}",
                    authority.symbol,
                    locations.join(", ")
                ),
            )),
        }
    }
}

fn audit_surface_publication_entrypoint(
    production: &[(ModuleMetrics, String)],
    findings: &mut Vec<Finding>,
) {
    let mut locations = Vec::new();
    for (metrics, contents) in production {
        for (index, line) in contents.lines().enumerate() {
            if declaration_symbol(line).is_some_and(|(symbol, externally_public)| {
                externally_public && symbol == SURFACE_PUBLICATION_ENTRYPOINT
            }) {
                locations.push(format!("{}:{}", path_text(&metrics.relative), index + 1));
            }
        }
    }

    match locations.as_slice() {
        [] => {}
        [location] if location.starts_with(SURFACE_PUBLICATION_ENTRYPOINT_PATH) => {}
        [location] => findings.push(Finding::fatal(
            "source.surface_publication_entrypoint_authority",
            Some(location.clone()),
            format!(
                "any public `{SURFACE_PUBLICATION_ENTRYPOINT}` declaration must remain the `AppRuntime` method in {SURFACE_PUBLICATION_ENTRYPOINT_PATH}"
            ),
        )),
        _ => findings.push(Finding::fatal(
            "source.surface_publication_entrypoint_authority",
            None::<String>,
            format!(
                "public `{SURFACE_PUBLICATION_ENTRYPOINT}` authority is declared multiple times: {}",
                locations.join(", ")
            ),
        )),
    }
}

fn audit_retired_authorities(production: &[(ModuleMetrics, String)], findings: &mut Vec<Finding>) {
    for (metrics, contents) in production {
        let path = path_text(&metrics.relative);
        for (line_index, line) in contents.lines().enumerate() {
            let Some((symbol, externally_public)) = declaration_symbol(line) else {
                continue;
            };
            let Some(retired) = RETIRED_AUTHORITIES
                .iter()
                .find(|retired| retired.symbol == symbol)
            else {
                continue;
            };
            let forbidden = match retired.scope {
                RetiredAuthorityScope::AnyDeclaration => true,
                RetiredAuthorityScope::ExternallyPublicDeclaration => externally_public,
                RetiredAuthorityScope::PublicReexportOnly => false,
            };
            if forbidden {
                findings.push(Finding::fatal(
                    "source.retired_m4_authority",
                    Some(format!("{path}:{}", line_index + 1)),
                    format!(
                        "retired M1-M4 transitional authority `{symbol}` must not be declared in production source"
                    ),
                ));
            }
        }

        for (line, statement) in public_reexport_statements(contents) {
            for retired in RETIRED_AUTHORITIES {
                if statement_identifiers(&statement).any(|token| token == retired.symbol) {
                    findings.push(Finding::fatal(
                        "source.retired_m4_authority",
                        Some(format!("{path}:{line}")),
                        format!(
                            "retired M1-M4 transitional authority `{}` must not be externally re-exported",
                            retired.symbol
                        ),
                    ));
                }
            }
        }
    }
}

fn audit_retired_m5_authorities(
    production: &[(ModuleMetrics, String)],
    findings: &mut Vec<Finding>,
) {
    for (metrics, contents) in production {
        let path = path_text(&metrics.relative);
        for (line_index, line) in contents.lines().enumerate() {
            let line_number = line_index + 1;
            if let Some((symbol, externally_public)) = declaration_symbol(line)
                && let Some(retired) = RETIRED_M5_AUTHORITIES
                    .iter()
                    .find(|retired| retired.symbol == symbol)
            {
                let forbidden = match retired.scope {
                    RetiredAuthorityScope::AnyDeclaration => true,
                    RetiredAuthorityScope::ExternallyPublicDeclaration => externally_public,
                    RetiredAuthorityScope::PublicReexportOnly => false,
                };
                if forbidden {
                    findings.push(Finding::fatal(
                        "source.retired_m5_authority",
                        Some(format!("{path}:{line_number}")),
                        format!(
                            "retired M5 semantic/testing authority `{symbol}` must be removed rather than retained as compatibility API"
                        ),
                    ));
                }
            }

            let code = rust_code_without_comments_or_literals(line);
            for pattern in RETIRED_M5_CODE_PATTERNS {
                if code.contains(pattern) {
                    findings.push(Finding::fatal(
                        "source.retired_m5_authority",
                        Some(format!("{path}:{line_number}")),
                        format!(
                            "retired M5 semantic authority `{pattern}` must remain absent; routed M4 LogicalScroll does not authorize a semantic compatibility path"
                        ),
                    ));
                }
            }

            let trimmed = code.trim_start();
            if trimmed.starts_with("pub type ")
                && let Some((_, target)) = trimmed.split_once('=')
                && SEMANTIC_ALIAS_TARGETS.iter().any(|semantic| {
                    statement_identifiers(target).any(|identifier| identifier == *semantic)
                })
            {
                findings.push(Finding::fatal(
                    "source.retired_m5_authority",
                    Some(format!("{path}:{line_number}")),
                    "public type aliases around accepted semantic/testing authority are forbidden by the M5 clean-cutover contract",
                ));
            }
        }

        for (line, statement) in public_reexport_statements(contents) {
            for retired in RETIRED_M5_AUTHORITIES {
                if statement_identifiers(&statement).any(|token| token == retired.symbol) {
                    findings.push(Finding::fatal(
                        "source.retired_m5_authority",
                        Some(format!("{path}:{line}")),
                        format!(
                            "retired M5 semantic/testing authority `{}` must not be externally re-exported",
                            retired.symbol
                        ),
                    ));
                }
            }
        }
    }
}

fn rust_code_without_comments_or_literals(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;

    while let Some(character) = characters.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            output.push(' ');
            continue;
        }
        if in_char {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '\'' {
                in_char = false;
            }
            output.push(' ');
            continue;
        }
        if character == '/' && characters.peek() == Some(&'/') {
            break;
        }
        match character {
            '"' => {
                in_string = true;
                output.push(' ');
            }
            '\'' => {
                in_char = true;
                output.push(' ');
            }
            _ => output.push(character),
        }
    }

    output
}

fn declaration_symbol(line: &str) -> Option<(&str, bool)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
        return None;
    }
    let externally_public = trimmed.starts_with("pub ");
    let mut declaration = trimmed
        .strip_prefix("pub(crate) ")
        .or_else(|| trimmed.strip_prefix("pub(super) "))
        .or_else(|| trimmed.strip_prefix("pub(self) "))
        .or_else(|| trimmed.strip_prefix("pub "))
        .or_else(|| strip_pub_in(trimmed))
        .unwrap_or(trimmed);

    loop {
        if let Some(rest) = declaration.strip_prefix("async ") {
            declaration = rest;
        } else if let Some(rest) = declaration.strip_prefix("const ") {
            declaration = rest;
        } else if let Some(rest) = declaration.strip_prefix("unsafe ") {
            declaration = rest;
        } else {
            break;
        }
    }

    for prefix in ["struct ", "enum ", "trait ", "type ", "fn "] {
        if let Some(rest) = declaration.strip_prefix(prefix) {
            let symbol = rest
                .split(|character: char| {
                    character.is_whitespace()
                        || matches!(character, '<' | '(' | '{' | ';' | ':' | '=')
                })
                .next()
                .unwrap_or_default();
            if !symbol.is_empty() {
                return Some((symbol, externally_public));
            }
        }
    }
    None
}

fn public_reexport_statements(contents: &str) -> Vec<(usize, String)> {
    let mut statements = Vec::new();
    let mut current: Option<(usize, String)> = None;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim_start();
        if let Some((start, statement)) = current.as_mut() {
            statement.push(' ');
            statement.push_str(trimmed);
            if trimmed.contains(';') {
                statements.push((*start, core::mem::take(statement)));
                current = None;
            }
            continue;
        }
        if trimmed.starts_with("pub use ") {
            if trimmed.contains(';') {
                statements.push((line_number, trimmed.to_owned()));
            } else {
                current = Some((line_number, trimmed.to_owned()));
            }
        }
    }

    statements
}

fn statement_identifiers(statement: &str) -> impl Iterator<Item = &str> {
    statement
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter(|token| !token.is_empty())
}

fn audit_volatile_architecture_state(
    root: &Path,
    findings: &mut Vec<Finding>,
) -> Result<(), String> {
    let mut files = Vec::new();
    let root_architecture = root.join("docs/architecture.md");
    if root_architecture.is_file() {
        files.push(PathBuf::from("docs/architecture.md"));
    }
    let architecture_directory = root.join("docs/architecture");
    if architecture_directory.is_dir() {
        collect_markdown_files(root, &architecture_directory, &mut files)?;
    }
    files.sort();

    for relative in files {
        let contents = fs::read_to_string(root.join(&relative)).map_err(|error| {
            format!(
                "failed to read architecture document {}: {error}",
                relative.display()
            )
        })?;
        let patterns = VOLATILE_ARCHITECTURE_PATTERNS
            .iter()
            .copied()
            .filter(|pattern| contents.contains(pattern))
            .collect::<Vec<_>>();
        if !patterns.is_empty() {
            findings.push(Finding::diagnostic(
                "diagnostic.volatile_architecture_state",
                Some(path_text(&relative)),
                format!(
                    "architecture document contains volatile execution markers: {}",
                    patterns.join(", ")
                ),
            ));
        }
    }

    Ok(())
}

fn collect_test_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let crates = root.join("crates");
    if crates.is_dir() {
        collect_rust_files_from(root, &crates, FileKind::Test, &mut files)?;
    }
    let tests = root.join("tests");
    if tests.is_dir() {
        collect_rust_files_from(root, &tests, FileKind::AllRust, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

#[derive(Clone, Copy)]
enum FileKind {
    Production,
    Test,
    AllRust,
}

fn collect_rust_files(
    root: &Path,
    directory: &Path,
    kind: FileKind,
) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    if directory.is_dir() {
        collect_rust_files_from(root, directory, kind, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_rust_files_from(
    root: &Path,
    directory: &Path,
    kind: FileKind,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files_from(root, &path, kind, files)?;
            continue;
        }
        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("failed to relativize {}: {error}", path.display()))?;
        let include = match kind {
            FileKind::Production => relative
                .components()
                .any(|component| component.as_os_str() == "src"),
            FileKind::Test => relative
                .components()
                .any(|component| component.as_os_str() == "tests"),
            FileKind::AllRust => true,
        };
        if include {
            files.push(relative.to_path_buf());
        }
    }
    Ok(())
}

fn collect_markdown_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(root, &path, files)?;
        } else if path.extension() == Some(OsStr::new("md")) {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("failed to relativize {}: {error}", path.display()))?;
            files.push(relative.to_path_buf());
        }
    }
    Ok(())
}

fn read_rust_file(root: &Path, relative: &Path) -> Result<String, String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {}: {error}", relative.display()))
}

fn module_metrics(relative: &Path, contents: &str) -> ModuleMetrics {
    let mut responsibilities = BTreeSet::new();
    let mut public_items = 0_usize;
    let mut reexports = 0_usize;

    for line in contents.lines() {
        let trimmed = line.trim_start();
        if is_public_item(trimmed) {
            public_items += 1;
        }
        if trimmed.starts_with("pub use ") || trimmed.starts_with("pub(crate) use ") {
            reexports += 1;
        }
        if is_item_declaration(trimmed) {
            let lower = trimmed.to_ascii_lowercase();
            for term in RESPONSIBILITY_TERMS {
                if lower.contains(term) {
                    responsibilities.insert(*term);
                }
            }
        }
    }

    ModuleMetrics {
        relative: relative.to_path_buf(),
        lines: contents.lines().count(),
        public_items,
        reexports,
        tests: contents.matches("#[test]").count(),
        responsibilities,
    }
}

fn is_public_item(line: &str) -> bool {
    (line.starts_with("pub ")
        || line.starts_with("pub(")
        || line.starts_with("pub(crate)")
        || line.starts_with("pub(in "))
        && is_item_declaration(line)
}

fn is_item_declaration(line: &str) -> bool {
    let declaration = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub(self) "))
        .or_else(|| line.strip_prefix("pub "))
        .or_else(|| strip_pub_in(line))
        .unwrap_or(line);

    [
        "async fn ",
        "const fn ",
        "enum ",
        "fn ",
        "mod ",
        "static ",
        "struct ",
        "trait ",
        "type ",
        "use ",
        "const ",
    ]
    .iter()
    .any(|prefix| declaration.starts_with(prefix))
}

fn strip_pub_in(line: &str) -> Option<&str> {
    let after = line.strip_prefix("pub(in ")?;
    let end = after.find(") ")?;
    Some(&after[end + 2..])
}

fn defines_struct(line: &str, symbol: &str) -> bool {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    tokens
        .windows(2)
        .any(|window| window[0] == "struct" && normalized_identifier(window[1]) == symbol)
}

fn normalized_identifier(token: &str) -> &str {
    token.split(['<', '{', '(', ';']).next().unwrap_or(token)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        SURFACE_PUBLICATION_ENTRYPOINT_PATH, audit_retired_authorities,
        audit_retired_m5_authorities, audit_surface_publication_entrypoint, declaration_symbol,
        defines_struct, module_metrics, normalized_identifier,
        rust_code_without_comments_or_literals,
    };

    fn production_source(path: &str, contents: &str) -> (super::ModuleMetrics, String) {
        (
            module_metrics(Path::new(path), contents),
            contents.to_owned(),
        )
    }

    #[test]
    fn authority_definition_parser_handles_visibility_and_generics() {
        assert!(defines_struct(
            "pub(crate) struct WorkQueue<Action> {",
            "WorkQueue"
        ));
        assert!(defines_struct("pub struct Trace;", "Trace"));
        assert!(!defines_struct("let trace = Trace::new();", "Trace"));
    }

    #[test]
    fn declaration_parser_handles_visibility_and_function_modifiers() {
        assert_eq!(
            declaration_symbol("pub const fn dispatch() {}"),
            Some(("dispatch", true))
        );
        assert_eq!(
            declaration_symbol("pub(crate) async unsafe fn internal() {}"),
            Some(("internal", false))
        );
    }

    #[test]
    fn identifier_normalization_removes_declaration_suffixes() {
        assert_eq!(normalized_identifier("WorkQueue<Action>"), "WorkQueue");
        assert_eq!(normalized_identifier("Trace;"), "Trace");
    }

    #[test]
    fn module_metrics_are_deterministic() {
        let metrics = module_metrics(
            Path::new("crates/example/src/lib.rs"),
            "pub struct SurfaceQueue;\npub use crate::trace::Trace;\n#[test]\nfn test() {}\n",
        );
        assert_eq!(metrics.public_items, 2);
        assert_eq!(metrics.reexports, 1);
        assert_eq!(metrics.tests, 1);
        assert!(metrics.responsibilities.contains("surface"));
        assert!(metrics.responsibilities.contains("queue"));
        assert!(metrics.responsibilities.contains("trace"));
    }

    #[test]
    fn retired_authority_audit_rejects_declarations_and_reexports() {
        let production = vec![production_source(
            "crates/example/src/lib.rs",
            "pub enum ActivationCapacity {}\npub const fn focus_next() {}\npub use crate::legacy::KeyboardActivationResult;\n",
        )];
        let mut findings = Vec::new();
        audit_retired_authorities(&production, &mut findings);
        assert_eq!(findings.len(), 3);
        assert!(findings.iter().all(|finding| {
            finding.code == "source.retired_m4_authority"
                && finding.severity == super::super::Severity::Fatal
        }));
    }

    #[test]
    fn retired_m5_authority_audit_rejects_stubs_scroll_and_aliases() {
        let production = vec![production_source(
            "crates/example/src/lib.rs",
            "pub struct WidgetSemanticProof;\npub fn activate_semantic() {}\npub type AccessibilityAction = SemanticAction;\nfn old_scroll() { let _ = SemanticAction::LogicalScroll; }\n",
        )];
        let mut findings = Vec::new();
        audit_retired_m5_authorities(&production, &mut findings);
        assert_eq!(findings.len(), 4);
        assert!(findings.iter().all(|finding| {
            finding.code == "source.retired_m5_authority"
                && finding.severity == super::super::Severity::Fatal
        }));
    }

    #[test]
    fn retired_m5_pattern_scanner_ignores_comments_and_literals() {
        assert!(
            !rust_code_without_comments_or_literals("// SemanticAction::LogicalScroll")
                .contains("SemanticAction::LogicalScroll")
        );
        assert!(
            !rust_code_without_comments_or_literals(
                "let note = \"SemanticAction::LogicalScroll\";"
            )
            .contains("SemanticAction::LogicalScroll")
        );
        assert!(
            rust_code_without_comments_or_literals("let action = SemanticAction::LogicalScroll;")
                .contains("SemanticAction::LogicalScroll")
        );
    }

    #[test]
    fn surface_publication_entrypoint_rejects_alternative_or_duplicate_authority() {
        let mut findings = Vec::new();
        audit_surface_publication_entrypoint(&[], &mut findings);
        assert!(findings.is_empty());

        let canonical = production_source(
            SURFACE_PUBLICATION_ENTRYPOINT_PATH,
            "pub fn publish_surface(&mut self) {}\n",
        );
        audit_surface_publication_entrypoint(std::slice::from_ref(&canonical), &mut findings);
        assert!(findings.is_empty());

        let alternative = production_source(
            "crates/runenui_runtime/src/lib.rs",
            "pub fn publish_surface() {}\n",
        );
        audit_surface_publication_entrypoint(std::slice::from_ref(&alternative), &mut findings);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].code,
            "source.surface_publication_entrypoint_authority"
        );
        assert_eq!(findings[0].severity, super::super::Severity::Fatal);

        findings.clear();
        audit_surface_publication_entrypoint(&[canonical, alternative], &mut findings);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].code,
            "source.surface_publication_entrypoint_authority"
        );
        assert_eq!(findings[0].severity, super::super::Severity::Fatal);
    }
}
