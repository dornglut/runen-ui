#![forbid(unsafe_code)]

use std::{fs, path::Path};

const CORE_COORDINATES: &str = "crates/runenui_core/src/text_coordinates.rs";
const TEXT_LIB: &str = "crates/runenui_text/src/lib.rs";
const TEXT_LAYOUT_STATE: &str = "crates/runenui_text/src/layout_state.rs";
const TEXT_CARET_MAP: &str = "crates/runenui_text/src/caret_map.rs";
const TEXT_PREEDIT: &str = "crates/runenui_text/src/preedit.rs";
const TEXT_MANIFEST: &str = "crates/runenui_text/Cargo.toml";
const NON_TEXT_PRODUCTION_ROOTS: [&str; 4] = [
    "crates/runenui_core/src",
    "crates/runenui_runtime/src",
    "crates/runenui_render_wgpu/src",
    "crates/runenui_winit/src",
];

#[test]
fn public_coordinates_remain_runenui_owned_and_revision_scoped() -> Result<(), String> {
    let root = workspace_root()?;
    let coordinates = read(&root.join(CORE_COORDINATES))?;
    for required in [
        "pub struct TextDocumentId(u64);",
        "pub struct TextDocumentRevision(u64);",
        "pub struct TextDocumentSnapshot",
        "pub enum TextAffinity",
        "pub struct TextPosition",
        "pub struct TextRange",
        "pub struct TextSelection",
        "pub enum TextDisplayPosition",
        "pub struct TextPreeditPosition",
        "pub fn from_utf16_offset(",
    ] {
        if !coordinates.contains(required) {
            return Err(format!(
                "M10B public coordinate authority lost required seam `{required}` in {CORE_COORDINATES}"
            ));
        }
    }
    for forbidden in ["parley::", "accesskit::", "winit::", "glyph_index"] {
        if coordinates.contains(forbidden) {
            return Err(format!(
                "M10B core coordinates leaked dependency/native authority `{forbidden}`"
            ));
        }
    }
    Ok(())
}

#[test]
fn caret_geometry_reuses_the_single_retained_layout_and_artifact() -> Result<(), String> {
    let root = workspace_root()?;
    let layout_state = read(&root.join(TEXT_LAYOUT_STATE))?;
    let caret_map = read(&root.join(TEXT_CARET_MAP))?;

    for required in [
        "cached: Arc<CachedTextLayout>",
        "grapheme_boundaries: Arc<[usize]>",
        "Cursor::from_byte_index(&self.cached.layout",
        "Cursor::from_point(&self.cached.layout",
        "let surface_to_layout = layout_to_surface",
        ".inverse()",
        "pub fn artifact(&self) -> &TextArtifact",
        "pub fn is_correlated_with(&self, artifact: &TextArtifact)",
        "pub fn preedit_selection(",
        "pub fn is_selection_collapsed(",
        "Arc::ptr_eq(&self.cached, &other.cached)",
        "for run in line.runs()",
        "for cluster in run.visual_clusters()",
    ] {
        if !caret_map.contains(required) {
            return Err(format!(
                "M10B caret-map correlation lost required seam `{required}` in {TEXT_CARET_MAP}"
            ));
        }
    }
    for required in [
        "layout: Layout<[u8; 4]>",
        "pub fn caret_map(",
        "pub fn preedit_caret_map(",
        "TextCaretMap::document(cached, snapshot)",
        "TextCaretMap::preedit(cached, projection)",
    ] {
        if !layout_state.contains(required) {
            return Err(format!(
                "M10B retained-layout seam `{required}` is missing from {TEXT_LAYOUT_STATE}"
            ));
        }
    }
    for forbidden in [
        "LayoutContext",
        "FontContext",
        "TextSystem",
        "PlainEditor",
        "AccessKit",
        "accesskit::",
        "break_all_lines",
    ] {
        if caret_map.contains(forbidden) {
            return Err(format!(
                "M10B caret map acquired forbidden second-layout/editor authority `{forbidden}`"
            ));
        }
    }
    Ok(())
}

#[test]
fn preedit_is_an_explicit_transient_projection_not_a_document_store() -> Result<(), String> {
    let root = workspace_root()?;
    let preedit = read(&root.join(TEXT_PREEDIT))?;
    for required in [
        "pub struct TextPreeditProjection",
        "document: Arc<str>",
        "replacement: TextRange",
        "generation: CompositionGeneration",
        "preedit: Arc<str>",
        "selection: Option<CompositionRange>",
        "display_text: Arc<str>",
        "pub fn position_from_display_offset(",
        "pub fn display_offset_for_position(",
    ] {
        if !preedit.contains(required) {
            return Err(format!(
                "M10B transient preedit projection lost required seam `{required}` in {TEXT_PREEDIT}"
            ));
        }
    }
    for forbidden in [
        "EditIntent",
        "EditRequest",
        "UiApp",
        "update(",
        "PlainEditor",
        "undo",
        "redo",
    ] {
        if preedit.contains(forbidden) {
            return Err(format!(
                "M10B preedit projection acquired M10C/editor authority `{forbidden}`"
            ));
        }
    }
    Ok(())
}

#[test]
fn dependency_algorithms_remain_private_and_no_parallel_editor_reappears() -> Result<(), String> {
    let root = workspace_root()?;
    let text_lib = read(&root.join(TEXT_LIB))?;
    let manifest = read(&root.join(TEXT_MANIFEST))?;

    if text_lib.contains("pub use parley") || text_lib.contains("pub use unicode_segmentation") {
        return Err("M10B must not re-export private text dependency types".to_owned());
    }
    for required in [
        "default-features = false, features = [\"std\", \"complex-scripts\"]",
        "unicode-segmentation = \"1.13.3\"",
    ] {
        if !manifest.contains(required) {
            return Err(format!(
                "M10B private dependency contract lost `{required}` in {TEXT_MANIFEST}"
            ));
        }
    }
    for forbidden in ["accesskit", "ropey", "xi-rope", "cosmic-text"] {
        if manifest.contains(forbidden) {
            return Err(format!(
                "M10B text manifest acquired forbidden alternate/native authority `{forbidden}`"
            ));
        }
    }
    for relative in NON_TEXT_PRODUCTION_ROOTS {
        for forbidden in ["PlainEditor", "parley::editing::", "unicode_segmentation::"] {
            assert_rust_tree_omits(&root.join(relative), forbidden)?;
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
                    "M10B non-text production source {} contains forbidden authority `{forbidden}`",
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
