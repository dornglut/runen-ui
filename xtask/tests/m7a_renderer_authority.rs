#![forbid(unsafe_code)]

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

const RENDERER_MANIFEST: &str = "crates/runenui_render_wgpu/Cargo.toml";
const RENDERER_SOURCE: &str = "crates/runenui_render_wgpu/src";

const ALLOWED_CORE_IDENTIFIERS: &[&str] = &[
    "Color",
    "ImagePrimitive",
    "LogicalLength",
    "LogicalPoint",
    "LogicalRect",
    "LogicalSize",
    "LogicalTransform",
    "PaintPrimitive",
    "Radius",
    "ResourceKind",
    "ResourceRef",
    "SceneLayer",
    "SceneOpacity",
    "SceneShape",
    "ShapedTextRunPrimitive",
    "SurfaceId",
];

const ALLOWED_RUNTIME_IDENTIFIERS: &[&str] = &[
    "PaintDamage",
    "PaintPublication",
    "PaintRevision",
    "PaintScene",
    "PaintSceneItem",
    "RasterScale",
    "SceneCapabilities",
    "SceneClip",
    "SceneRequirements",
    "UnsupportedSceneRequirement",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameworkCrate {
    Core,
    Runtime,
}

impl FrameworkCrate {
    const fn name(self) -> &'static str {
        match self {
            Self::Core => "runenui_core",
            Self::Runtime => "runenui_runtime",
        }
    }

    const fn allowed_identifiers(self) -> &'static [&'static str] {
        match self {
            Self::Core => ALLOWED_CORE_IDENTIFIERS,
            Self::Runtime => ALLOWED_RUNTIME_IDENTIFIERS,
        }
    }

    fn allows(self, identifier: &str) -> bool {
        self.allowed_identifiers().contains(&identifier)
    }
}

#[test]
fn renderer_production_source_uses_only_neutral_framework_authority() -> Result<(), String> {
    let root = workspace_root()?;
    let manifest = fs::read_to_string(root.join(RENDERER_MANIFEST))
        .map_err(|error| format!("failed to read {RENDERER_MANIFEST}: {error}"))?;
    let mut failures = canonical_dependency_name_failures(&manifest);

    let source_root = root.join(RENDERER_SOURCE);
    let mut files = Vec::new();
    collect_rust_files(&source_root, &mut files)?;
    files.sort();

    if files.is_empty() {
        return Err(format!(
            "renderer source audit found no Rust files under {RENDERER_SOURCE}"
        ));
    }

    for file in files {
        let relative = file
            .strip_prefix(&root)
            .map_err(|error| format!("failed to relativize {}: {error}", file.display()))?;
        let contents = fs::read_to_string(&file)
            .map_err(|error| format!("failed to read {}: {error}", relative.display()))?;
        let production = remove_cfg_test_modules(&contents);
        audit_framework_authority(relative, &production, &mut failures);
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "M7A renderer source authority audit failed:\n{}",
            failures.join("\n")
        ))
    }
}

#[test]
fn neutral_allow_list_rejects_behavior_authority_by_default() {
    for identifier in [
        "UiApp",
        "Element",
        "WidgetTypeId",
        "SemanticRole",
        "MountedNodeId",
        "LogicalKey",
        "AppRuntime",
        "SurfacePublication",
        "HitTestScene",
        "TraceRecord",
    ] {
        assert!(
            !FrameworkCrate::Core.allows(identifier)
                && !FrameworkCrate::Runtime.allows(identifier),
            "behavior authority `{identifier}` must not enter the renderer allow-list"
        );
    }

    for identifier in ALLOWED_CORE_IDENTIFIERS {
        assert!(FrameworkCrate::Core.allows(identifier));
    }
    for identifier in ALLOWED_RUNTIME_IDENTIFIERS {
        assert!(FrameworkCrate::Runtime.allows(identifier));
    }
}

#[test]
fn framework_use_audit_rejects_aliases_wildcards_and_non_neutral_imports() {
    let source = r#"
use runenui_core::{Color, Element};
use runenui_runtime::*;
use runenui_runtime::PaintPublication as Publication;
let _ = runenui_core::UiApp;
"#;
    let mut failures = Vec::new();
    audit_framework_authority(Path::new("fixture.rs"), source, &mut failures);

    assert!(failures.iter().any(|failure| failure.contains("`Element`")));
    assert!(failures.iter().any(|failure| failure.contains("wildcard")));
    assert!(failures.iter().any(|failure| failure.contains("alias")));
    assert!(failures.iter().any(|failure| failure.contains("`UiApp`")));
}

#[test]
fn cfg_test_modules_are_not_treated_as_renderer_production_authority() {
    let source = r#"
use runenui_runtime::PaintPublication;

pub fn consume(_: &PaintPublication) {}

#[cfg(test)]
mod fixtures {
    use runenui_core::{UiApp, text};
    use runenui_runtime::AppRuntime;

    fn fixture() {
        let _ = text("fixture with { braces }");
    }
}
"#;

    let production = remove_cfg_test_modules(source);
    let mut failures = Vec::new();
    audit_framework_authority(Path::new("fixture.rs"), &production, &mut failures);
    assert!(failures.is_empty());
    assert!(production.contains("PaintPublication"));
    assert!(!production.contains("AppRuntime"));
    assert!(!production.contains("text("));
}

fn workspace_root() -> Result<PathBuf, String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().map(Path::to_path_buf).ok_or_else(|| {
        format!(
            "xtask manifest directory has no workspace parent: {}",
            manifest_dir.display()
        )
    })
}

fn canonical_dependency_name_failures(manifest: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for package in [FrameworkCrate::Core, FrameworkCrate::Runtime] {
        let canonical = format!("{} =", package.name());
        if !manifest
            .lines()
            .any(|line| line.trim_start().starts_with(&canonical))
        {
            failures.push(format!(
                "{RENDERER_MANIFEST} must keep canonical dependency key `{}` so source authority cannot hide behind an alias",
                package.name()
            ));
        }
        if manifest.lines().any(|line| {
            line.contains("package") && line.contains(package.name())
        }) {
            failures.push(format!(
                "{RENDERER_MANIFEST} must not rename framework package `{}` through a Cargo dependency alias",
                package.name()
            ));
        }
    }
    failures
}

fn audit_framework_authority(relative: &Path, contents: &str, failures: &mut Vec<String>) {
    for (line, statement) in framework_use_statements(contents) {
        let Some(framework) = framework_crate_in(&statement) else {
            continue;
        };
        if statement.contains('*') {
            failures.push(format!(
                "{}:{line} uses a wildcard import from `{}`; renderer framework imports must remain explicit",
                relative.display(),
                framework.name()
            ));
            continue;
        }
        if identifiers(&statement).any(|identifier| identifier == "as") {
            failures.push(format!(
                "{}:{line} aliases an import from `{}`; canonical framework names keep the authority audit explicit",
                relative.display(),
                framework.name()
            ));
            continue;
        }

        for identifier in identifiers(&statement) {
            if import_syntax_identifier(identifier) || identifier == framework.name() {
                continue;
            }
            if !framework.allows(identifier) {
                failures.push(format!(
                    "{}:{line} imports non-neutral `{}` authority `{identifier}`",
                    relative.display(),
                    framework.name()
                ));
            }
        }
    }

    for (line_index, line) in contents.lines().enumerate() {
        let code = rust_code_without_line_comments_or_strings(line);
        for framework in [FrameworkCrate::Core, FrameworkCrate::Runtime] {
            for identifier in qualified_framework_identifiers(&code, framework) {
                if !framework.allows(identifier) {
                    failures.push(format!(
                        "{}:{} directly references non-neutral `{}` authority `{identifier}`",
                        relative.display(),
                        line_index + 1,
                        framework.name()
                    ));
                }
            }
        }
    }
}

fn framework_use_statements(contents: &str) -> Vec<(usize, String)> {
    let mut statements = Vec::new();
    let mut current: Option<(usize, String)> = None;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let code = rust_code_without_line_comments_or_strings(line);
        let trimmed = code.trim_start();

        if let Some((start, statement)) = current.as_mut() {
            statement.push(' ');
            statement.push_str(trimmed);
            if trimmed.contains(';') {
                statements.push((*start, core::mem::take(statement)));
                current = None;
            }
            continue;
        }

        if framework_use_start(trimmed) {
            if trimmed.contains(';') {
                statements.push((line_number, trimmed.to_owned()));
            } else {
                current = Some((line_number, trimmed.to_owned()));
            }
        }
    }

    statements
}

fn framework_use_start(line: &str) -> bool {
    let declaration = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub(self) "))
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);
    declaration.starts_with("use runenui_core::")
        || declaration.starts_with("use runenui_runtime::")
}

fn framework_crate_in(statement: &str) -> Option<FrameworkCrate> {
    if identifiers(statement).any(|identifier| identifier == FrameworkCrate::Core.name()) {
        Some(FrameworkCrate::Core)
    } else if identifiers(statement).any(|identifier| identifier == FrameworkCrate::Runtime.name())
    {
        Some(FrameworkCrate::Runtime)
    } else {
        None
    }
}

fn import_syntax_identifier(identifier: &str) -> bool {
    matches!(identifier, "pub" | "crate" | "self" | "super" | "use")
}

fn qualified_framework_identifiers<'a>(
    code: &'a str,
    framework: FrameworkCrate,
) -> Vec<&'a str> {
    let marker = format!("{}::", framework.name());
    let mut identifiers = Vec::new();
    let mut remaining = code;

    while let Some(index) = remaining.find(&marker) {
        let after_marker = &remaining[index + marker.len()..];
        let identifier = after_marker
            .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .next()
            .unwrap_or_default();
        if !identifier.is_empty() {
            identifiers.push(identifier);
            remaining = &after_marker[identifier.len()..];
        } else if after_marker.is_empty() {
            break;
        } else {
            remaining = &after_marker[1..];
        }
    }

    identifiers
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension() == Some(OsStr::new("rs")) {
            files.push(path);
        }
    }
    Ok(())
}

fn identifiers(code: &str) -> impl Iterator<Item = &str> {
    code.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter(|token| !token.is_empty())
}

fn rust_code_without_line_comments_or_strings(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();
    let mut in_string = false;
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

        if character == '/' && characters.peek() == Some(&'/') {
            break;
        }
        if character == '"' {
            in_string = true;
            output.push(' ');
        } else {
            output.push(character);
        }
    }

    output
}

fn remove_cfg_test_modules(contents: &str) -> String {
    let lines = contents.lines().collect::<Vec<_>>();
    let mut output = String::with_capacity(contents.len());
    let mut index = 0_usize;

    while index < lines.len() {
        if lines[index].trim() == "#[cfg(test)]"
            && lines.get(index + 1).is_some_and(|line| {
                let declaration = line.trim_start();
                declaration.starts_with("mod ") && declaration.contains('{')
            })
        {
            output.push('\n');
            index += 1;
            let mut depth = 0_i32;
            let mut entered = false;
            while index < lines.len() {
                let code = rust_code_without_line_comments_or_strings(lines[index]);
                for character in code.chars() {
                    match character {
                        '{' => {
                            depth += 1;
                            entered = true;
                        }
                        '}' if entered => depth -= 1,
                        _ => {}
                    }
                }
                output.push('\n');
                index += 1;
                if entered && depth == 0 {
                    break;
                }
            }
            continue;
        }

        output.push_str(lines[index]);
        output.push('\n');
        index += 1;
    }

    output
}
