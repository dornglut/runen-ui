#![forbid(unsafe_code)]

use std::{fs, path::Path};

const TAFFY_LAYOUT: &str = "crates/runenui_runtime/src/surface/taffy_layout.rs";
const PLANNING: &str = "crates/runenui_runtime/src/surface/planning.rs";
const RESOLVE: &str = "crates/runenui_runtime/src/surface/resolve.rs";
const CURRENT_RENDERER_AUTHORITY: [&str; 3] = [
    "crates/runenui_render_wgpu/README.md",
    "crates/runenui_render_wgpu/src/lib.rs",
    "examples/counter/README.md",
];
const PRODUCTION_SOURCE_ROOTS: [&str; 4] = [
    "crates/runenui_core/src",
    "crates/runenui_text/src",
    "crates/runenui_runtime/src",
    "crates/runenui_render_wgpu/src",
];
const RETIRED_PRODUCTION_PATHS: [&str; 2] = [
    "crates/runenui_runtime/src/measurement.rs",
    "crates/runenui_runtime/src/surface/measure.rs",
];
const RETIRED_PRODUCTION_AUTHORITIES: [&str; 6] = [
    "MeasurementProvider",
    "DeterministicMeasurementProvider",
    "TextMeasurementRequest",
    "SurfaceMeasurer",
    "vertical linear fallback",
    "ShapedRunRaster",
];

#[test]
fn production_text_measurement_and_paint_share_one_retained_artifact_path() -> Result<(), String> {
    let root = workspace_root()?;
    let taffy_layout = read(&root.join(TAFFY_LAYOUT))?;
    let resolve = read(&root.join(RESOLVE))?;

    for required in [
        "let request = TextRequest::new(content, typography, constraints);",
        "match self.text_system.layout_text(&mut state, &request)",
        "let artifact = outcome.artifact();",
        "let text_size = artifact.size();",
        "self.final_text_states[index] = Some(state.clone());",
        "self.text_layouts[index] = state;",
    ] {
        if !taffy_layout.contains(required) {
            return Err(format!(
                "M8D text/layout authority lost required production seam `{required}` in {TAFFY_LAYOUT}"
            ));
        }
    }

    for required in [
        "if let Some(artifact) = layout.text_layouts[mounted_preorder].artifact()",
        ".lease_shaped_run(run.resource_ref())",
        "let computed = effective.node(mounted_preorder).computed_style();",
        "let item = text_run_item(run, computed);",
    ] {
        if !resolve.contains(required) {
            return Err(format!(
                "M8D paint correlation lost required retained-artifact/effective-style seam `{required}` in {RESOLVE}"
            ));
        }
    }
    if resolve.contains("let item = text_run_item(run, &styles.resolutions[mounted_preorder]);") {
        return Err(
            "M8D text paint must consume the accepted effective style rather than bypassing motion through target style resolution"
                .to_owned(),
        );
    }
    if resolve.contains("layout_text(") {
        return Err(
            "M8D forbids paint composition from reshaping or re-line-breaking text".to_owned(),
        );
    }
    Ok(())
}

#[test]
fn runtime_layout_has_one_bounded_taffy_entrypoint_without_parallel_stabilization_authority()
-> Result<(), String> {
    let root = workspace_root()?;
    let taffy_layout = read(&root.join(TAFFY_LAYOUT))?;
    let planning = read(&root.join(PLANNING))?;

    let root_layout_calls = taffy_layout.matches("compute_root_layout(").count();
    if root_layout_calls != 1 {
        return Err(format!(
            "M8D requires one runtime Taffy entrypoint per layout transaction; found {root_layout_calls} source call sites in {TAFFY_LAYOUT}"
        ));
    }
    if !taffy_layout
        .contains("compute_root_layout(&mut kernel, root, available_space(root_constraints));")
    {
        return Err(
            "M8D runtime layout no longer enters Taffy through the reviewed root call".to_owned(),
        );
    }
    for forbidden in [
        "measure_until_stable",
        "layout_until_stable",
        "stabilization_loop",
    ] {
        if taffy_layout.contains(forbidden) {
            return Err(format!(
                "M8D forbids a parallel framework stabilization authority: `{forbidden}`"
            ));
        }
    }

    let retained_layout_calls = planning
        .matches("current.layout = Arc::new(resolve_layout_phase")
        .count();
    if retained_layout_calls != 1 {
        return Err(format!(
            "M8D requires one retained dirty-layout phase call site; found {retained_layout_calls} in {PLANNING}"
        ));
    }
    Ok(())
}

#[test]
fn proof_era_production_authorities_remain_retired() -> Result<(), String> {
    let root = workspace_root()?;

    for relative in RETIRED_PRODUCTION_PATHS {
        if root.join(relative).exists() {
            return Err(format!(
                "M8D retired production authority path must not reappear: {relative}"
            ));
        }
    }

    for relative in PRODUCTION_SOURCE_ROOTS {
        for forbidden in RETIRED_PRODUCTION_AUTHORITIES {
            assert_rust_tree_omits(&root.join(relative), forbidden)?;
        }
    }
    Ok(())
}

#[test]
fn proof_era_renderer_and_showcase_authority_is_not_current() -> Result<(), String> {
    let root = workspace_root()?;
    for relative in CURRENT_RENDERER_AUTHORITY {
        let content = read(&root.join(relative))?;
        for forbidden in [
            "M7A",
            "M7B",
            "does not render glyphs",
            "literal fill paint only",
            "future surface drawing",
        ] {
            if content.contains(forbidden) {
                return Err(format!(
                    "M8D current authority {relative} still presents proof-era renderer/showcase wording `{forbidden}`"
                ));
            }
        }
    }
    Ok(())
}

fn assert_rust_tree_omits(directory: &Path, forbidden: &str) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            assert_rust_tree_omits(&path, forbidden)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            let content = read(&path)?;
            if content.contains(forbidden) {
                return Err(format!(
                    "M8D production source {} still contains replaced authority `{forbidden}`",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn workspace_root() -> Result<std::path::PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest directory has no workspace parent".to_owned())
}
