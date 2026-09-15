#![forbid(unsafe_code)]

use std::{fs, path::Path};

const PAINT: &str = "crates/runenui_core/src/paint.rs";
const COMPUTED_STYLE: &str = "crates/runenui_core/src/computed_style.rs";
const IMAGE: &str = "crates/runenui_core/src/visual/image.rs";
const PRESENTATION: &str = "crates/runenui_core/src/visual/presentation.rs";
const RUNTIME_RESOLVE: &str = "crates/runenui_runtime/src/surface/resolve.rs";
const RENDERER_ROOT: &str = "crates/runenui_render_wgpu/src";

#[test]
fn generic_visual_primitives_and_brush_background_remain_the_only_public_core_authority()
-> Result<(), String> {
    let root = workspace_root()?;
    let paint = read(&root.join(PAINT))?;
    let computed = read(&root.join(COMPUTED_STYLE))?;

    for required in [
        "Fill { shape: SceneShape, brush: Brush }",
        "Stroke {",
        "shape: SceneShape,",
        "brush: Brush,",
        "style: StrokeStyle,",
        "Image(ImagePrimitive)",
        "ShapedTextRun(ShapedTextRunPrimitive)",
        "pub const fn image(descriptor: ImagePaintDescriptor) -> Self",
    ] {
        if !paint.contains(required) {
            return Err(format!(
                "M9C generic paint authority lost required current seam `{required}` in {PAINT}"
            ));
        }
    }
    for forbidden in ["FillRect", "StrokeRect", "fill_rect(", "stroke_rect("] {
        if paint.contains(forbidden) {
            return Err(format!(
                "M9C public core paint authority restored retired specialized rectangle seam `{forbidden}`"
            ));
        }
    }
    if !computed.contains("background: Option<Brush>") {
        return Err(format!(
            "M9C brush-valued background authority is missing from {COMPUTED_STYLE}"
        ));
    }
    if computed.contains("background: Option<Color>") {
        return Err(
            "M9C forbids restoring color-only computed background authority".to_owned(),
        );
    }
    Ok(())
}

#[test]
fn image_and_shaped_text_publication_paths_have_no_retired_exact_mapping_or_paint_bypass()
-> Result<(), String> {
    let root = workspace_root()?;
    let paint = read(&root.join(PAINT))?;
    let image = read(&root.join(IMAGE))?;
    let resolve = read(&root.join(RUNTIME_RESOLVE))?;

    for required in [
        "pub enum ImageMapping",
        "Fit {",
        "NineSlice {",
        "pub struct ImagePaintDescriptor",
    ] {
        if !image.contains(required) {
            return Err(format!(
                "M9C accepted image mapping authority lost required seam `{required}` in {IMAGE}"
            ));
        }
    }
    if paint.contains("pub fn image(\n        resource: ResourceRef,\n        destination: LogicalRect")
        || paint.contains("ImagePrimitive::new(resource, destination)")
    {
        return Err(
            "M9C retired exact resource/destination image constructor must not reappear"
                .to_owned(),
        );
    }

    for required in [
        "PaintContributionItem::shaped_text_run(",
        ".lease_shaped_run(run.resource_ref())",
        "let item = text_run_item(run, computed);",
    ] {
        if !resolve.contains(required) {
            return Err(format!(
                "M9C shaped-text publication authority lost required seam `{required}` in {RUNTIME_RESOLVE}"
            ));
        }
    }
    if resolve.contains("layout_text(") {
        return Err(
            "M9C forbids paint publication from acquiring a second text-shaping authority"
                .to_owned(),
        );
    }
    Ok(())
}

#[test]
fn presentation_composition_keeps_one_core_resolver_and_one_runtime_application_path()
-> Result<(), String> {
    let root = workspace_root()?;
    let presentation = read(&root.join(PRESENTATION))?;
    let resolve = read(&root.join(RUNTIME_RESOLVE))?;

    for required in [
        "let origin_x = self.origin.x().get() * size.width();",
        "let origin_y = self.origin.y().get() * size.height();",
        "let m11 = cos * self.scale.x();",
        "let m12 = sin * self.scale.x();",
        "let m21 = -sin * self.scale.y();",
        "let m22 = cos * self.scale.y();",
        "let tx = origin_x + self.translation.x()",
        "let ty = origin_y + self.translation.y()",
    ] {
        if !presentation.contains(required) {
            return Err(format!(
                "M9C frozen presentation order lost required seam `{required}` in {PRESENTATION}"
            ));
        }
    }
    for required in [
        ".presentation()",
        "presentation.resolve_in_box(bounds.size())",
        "let placement = LogicalTransform::translation(bounds.x(), bounds.y())",
        "let owner_to_surface = node_presentation",
        ".then(placement)",
    ] {
        if !resolve.contains(required) {
            return Err(format!(
                "M9C runtime presentation composition lost required seam `{required}` in {RUNTIME_RESOLVE}"
            ));
        }
    }
    if resolve.matches("presentation.resolve_in_box(bounds.size())").count() != 1 {
        return Err(
            "M9C requires one reviewed runtime application of the sampled node presentation"
                .to_owned(),
        );
    }
    Ok(())
}

#[test]
fn concrete_renderer_does_not_acquire_motion_or_interaction_authority() -> Result<(), String> {
    let root = workspace_root()?;
    let renderer = root.join(RENDERER_ROOT);
    for forbidden in [
        "AnimationId",
        "ExplicitTimeline",
        "MotionEasing",
        "MotionKeyframe",
        "MotionRepeat",
        "MotionTarget",
        "MotionValue",
        "ReducedMotionStrategy",
        "TimelineSpec",
        "TransitionSpec",
        "StyleInteractionState",
        "StylePreferences",
        "ManualClock",
        "MonotonicClock",
    ] {
        assert_rust_tree_omits(&renderer, forbidden)?;
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
                    "M9C renderer source {} contains competing motion/interaction authority `{forbidden}`",
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
