#![forbid(unsafe_code)]

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

const RENDERER_SOURCE: &str = "crates/runenui_render_wgpu/src";

const FORBIDDEN_EXACT_IDENTIFIERS: &[&str] = &[
    // Core/runtime hidden or live behavior authority.
    "__runtime",
    "AppRuntime",
    // Concrete built-in authoring/widget authority.
    "Button",
    "Container",
    "Text",
    "button",
    "column",
    "container",
    "row",
    "text",
    // Layout/debug/runtime products are not renderer input authority.
    "AxisConstraints",
    "AxisLimit",
    "ComputedStyle",
    "DebugSurfaceRenderer",
    "LayoutConstraints",
    "LayoutStyle",
    "MeasurementProvider",
    "SurfaceFrame",
    "SurfaceLayoutNode",
    "SurfaceLayoutReport",
    "SurfaceNode",
    "SurfacePhase",
    "SurfacePhaseReport",
    "SurfacePublication",
    "SurfaceStyleNode",
    "SurfaceStyleReport",
    "TextMeasurement",
    "TextMeasurementKind",
    "TextMeasurementRequest",
    "render_debug_surface_frame",
    "render_debug_surface_style_report",
];

const FORBIDDEN_IDENTIFIER_PREFIXES: &[&str] = &["Mounted", "Semantic", "Trace", "Widget"];

#[test]
fn renderer_production_source_has_no_forbidden_behavior_authority() -> Result<(), String> {
    let root = workspace_root()?;
    let source_root = root.join(RENDERER_SOURCE);
    let mut files = Vec::new();
    collect_rust_files(&source_root, &mut files)?;
    files.sort();

    if files.is_empty() {
        return Err(format!(
            "renderer source audit found no Rust files under {RENDERER_SOURCE}"
        ));
    }

    let mut failures = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .map_err(|error| format!("failed to relativize {}: {error}", file.display()))?;
        let contents = fs::read_to_string(&file)
            .map_err(|error| format!("failed to read {}: {error}", relative.display()))?;
        let production = remove_cfg_test_modules(&contents);

        for (line_index, line) in production.lines().enumerate() {
            let code = rust_code_without_line_comments_or_strings(line);
            for identifier in identifiers(&code) {
                if forbidden_identifier(identifier) {
                    failures.push(format!(
                        "{}:{} imports or references forbidden renderer authority `{identifier}`",
                        relative.display(),
                        line_index + 1
                    ));
                }
            }
        }
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
fn forbidden_identifier_policy_covers_m7a_authority_categories() {
    for identifier in [
        "__runtime",
        "Button",
        "text",
        "WidgetTypeId",
        "SemanticRole",
        "SemanticPublication",
        "MountedTreeIndex",
        "SurfaceFrame",
        "ComputedStyle",
        "TraceRecord",
        "AppRuntime",
    ] {
        assert!(
            forbidden_identifier(identifier),
            "expected `{identifier}` to be forbidden renderer authority"
        );
    }

    for identifier in [
        "PaintPublication",
        "PaintRevision",
        "PaintScene",
        "PaintPrimitive",
        "ResourceRef",
        "ResourceKind",
        "RasterScale",
        "LogicalPoint",
        "LogicalRect",
        "LogicalTransform",
        "Color",
    ] {
        assert!(
            !forbidden_identifier(identifier),
            "ordinary renderer-neutral input `{identifier}` must remain allowed"
        );
    }
}

#[test]
fn cfg_test_modules_are_not_treated_as_renderer_production_authority() {
    let source = r#"
use runenui_runtime::PaintPublication;

pub fn consume(_: &PaintPublication) {}

#[cfg(test)]
mod tests {
    use runenui_core::{UiApp, text};
    use runenui_runtime::AppRuntime;

    fn fixture() {
        let _ = text("fixture with { braces }");
    }
}
"#;

    let production = remove_cfg_test_modules(source);
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

fn forbidden_identifier(identifier: &str) -> bool {
    FORBIDDEN_EXACT_IDENTIFIERS.contains(&identifier)
        || FORBIDDEN_IDENTIFIER_PREFIXES
            .iter()
            .any(|prefix| identifier.starts_with(prefix))
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
            && lines
                .get(index + 1)
                .is_some_and(|line| line.trim_start().starts_with("mod tests"))
        {
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
