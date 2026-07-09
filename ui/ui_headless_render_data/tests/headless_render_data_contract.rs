use std::{fs, path::PathBuf};

use ui_artifacts::{UiRuntimeArtifact, UiRuntimeArtifactDiagnostic};
use ui_compiler::UiCompiler;
use ui_controls::{ControlPackageRegistry, runenwerk_control_package};
use ui_definition::UiNodeDefinition;
use ui_headless_render_data::{
    DIAGNOSTIC_RENDER_PRIMITIVE_REPORT_FAILED, UiHeadlessRenderDataReport,
};
use ui_program_lowering::form_ui_program_report_from_node_with_registry_snapshot;
use ui_render_data::{UiFrame, UiPrimitive, UiSortKey};
use ui_render_primitives::UiRenderPrimitiveReport;
use ui_runtime_view::{ButtonRuntimeHostData, UiRuntimeView};
use ui_text::{FontAtlasSource, FontFaceMetrics, FontId, GlyphMetrics, MsdfFontAtlas};
use ui_theme::ThemeTokens;

#[test]
fn render_data_adapter_refuses_failed_render_primitive_report() {
    let mut artifact = compiled_selected_button_artifact();
    artifact
        .manifest
        .push_diagnostic(UiRuntimeArtifactDiagnostic::error(
            "fixture.artifact.error",
            "fixture artifact error",
        ));
    let runtime_report = UiRuntimeView::from_artifact_report(&artifact);
    let primitive_report = UiRenderPrimitiveReport::from_runtime_view_report_with_host_data(
        &runtime_report,
        ui_math::UiSize::new(320.0, 200.0),
        &ThemeTokens::default(),
        &TestFontAtlasSource::default(),
        TEST_FONT_ID,
        &ButtonRuntimeHostData::new().with_bool("ui_gallery.button.selected.active", true),
    );

    let report = UiHeadlessRenderDataReport::from_render_primitive_report(&primitive_report);

    assert!(!runtime_report.passed());
    assert!(!primitive_report.passed());
    assert!(!report.passed());
    assert!(report.frame().is_none());
    assert_has_diagnostic(&report, DIAGNOSTIC_RENDER_PRIMITIVE_REPORT_FAILED);
}

#[test]
fn render_data_adapter_converts_selected_button_static_frame() {
    let primitive_report = selected_button_render_primitive_report();
    let report = UiHeadlessRenderDataReport::from_render_primitive_report(&primitive_report);

    assert!(report.passed(), "{:?}", report.diagnostics());
    let frame = report.frame().expect("render-data frame should exist");

    assert!(has_rect_primitive(frame));
    assert!(has_border_primitive(frame));
    assert!(has_glyph_run_primitive(frame));
    assert_eq!(
        primitive_sort_keys(frame),
        sorted_primitive_sort_keys(frame)
    );
    assert_eq!(frame.surfaces.len(), 1);
    assert_eq!(frame.surfaces[0].size.width, 320.0);
    assert_eq!(frame.surfaces[0].size.height, 200.0);
}

#[test]
fn render_data_adapter_is_deterministic() {
    let primitive_report = selected_button_render_primitive_report();

    let first = UiHeadlessRenderDataReport::from_render_primitive_report(&primitive_report);
    let second = UiHeadlessRenderDataReport::from_render_primitive_report(&primitive_report);

    assert_eq!(first.frame(), second.frame());
    assert_eq!(first.diagnostics(), second.diagnostics());
    assert_eq!(first.provenance(), second.provenance());
}

#[test]
fn render_data_adapter_preserves_or_reports_source_map_provenance() {
    let primitive_report = selected_button_render_primitive_report();
    let report = UiHeadlessRenderDataReport::from_render_primitive_report(&primitive_report);
    let frame = report.frame().expect("render-data frame should exist");
    let primitive_count = frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .map(|layer| layer.primitives.len())
        .sum::<usize>();

    assert_eq!(primitive_count, report.provenance().len());
    assert!(
        report
            .provenance()
            .iter()
            .all(|entry| entry.source_map_index > 0),
        "{:?}",
        report.provenance()
    );
}

#[test]
fn render_data_adapter_uses_glyph_runs_for_static_gallery_path() {
    let primitive_report = selected_button_render_primitive_report();
    let report = UiHeadlessRenderDataReport::from_render_primitive_report(&primitive_report);
    let frame = report.frame().expect("render-data frame should exist");

    assert!(report.passed(), "{:?}", report.diagnostics());
    assert!(has_glyph_run_primitive(frame));
    assert!(
        report.diagnostics().is_empty(),
        "{:?}",
        report.diagnostics()
    );
}

fn selected_button_render_primitive_report() -> UiRenderPrimitiveReport {
    let artifact = compiled_selected_button_artifact();
    let runtime_report = UiRuntimeView::from_artifact_report(&artifact);
    assert!(
        runtime_report.passed(),
        "{:?}",
        runtime_report.view.diagnostics
    );

    let primitive_report = UiRenderPrimitiveReport::from_runtime_view_report_with_host_data(
        &runtime_report,
        ui_math::UiSize::new(320.0, 200.0),
        &ThemeTokens::default(),
        &TestFontAtlasSource::default(),
        TEST_FONT_ID,
        &ButtonRuntimeHostData::new().with_bool("ui_gallery.button.selected.active", true),
    );
    assert!(
        primitive_report.passed(),
        "{:?}",
        primitive_report.diagnostics()
    );
    primitive_report
}

fn compiled_selected_button_artifact() -> UiRuntimeArtifact {
    let node = load_node("assets/ui_gallery/button/selected.ron");
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk controls package should register");

    let formation_report = form_ui_program_report_from_node_with_registry_snapshot(
        "ui_gallery.button.selected",
        "assets.ui_gallery.button.selected",
        &node,
        &registry.snapshot(),
    );

    assert!(
        formation_report.passed(),
        "{:?}",
        formation_report.diagnostics
    );

    let report = UiCompiler.compile_report(&formation_report.program);
    assert!(
        report.passed(),
        "{:?}",
        report.artifact.manifest.diagnostics
    );

    report.artifact
}

fn has_rect_primitive(frame: &UiFrame) -> bool {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .any(|primitive| matches!(primitive, UiPrimitive::Rect(_)))
}

fn has_border_primitive(frame: &UiFrame) -> bool {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .any(|primitive| matches!(primitive, UiPrimitive::Border(_)))
}

fn has_glyph_run_primitive(frame: &UiFrame) -> bool {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .any(|primitive| matches!(primitive, UiPrimitive::GlyphRun(_)))
}

fn primitive_sort_keys(frame: &UiFrame) -> Vec<UiSortKey> {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .map(primitive_sort_key)
        .collect()
}

fn sorted_primitive_sort_keys(frame: &UiFrame) -> Vec<UiSortKey> {
    let mut sort_keys = primitive_sort_keys(frame);
    sort_keys.sort();
    sort_keys
}

fn primitive_sort_key(primitive: &UiPrimitive) -> UiSortKey {
    match primitive {
        UiPrimitive::Rect(value) => value.sort_key,
        UiPrimitive::Border(value) => value.sort_key,
        UiPrimitive::GlyphRun(value) => value.sort_key,
        UiPrimitive::Image(value) => value.sort_key,
        UiPrimitive::Stroke(value) => value.sort_key,
        UiPrimitive::ViewportSurfaceEmbed(value) => value.sort_key,
        UiPrimitive::ProductSurface(value) => value.sort_key,
        UiPrimitive::Clip(ui_render_data::ClipPrimitive::Push { sort_key, .. })
        | UiPrimitive::Clip(ui_render_data::ClipPrimitive::Pop { sort_key }) => *sort_key,
    }
}

fn assert_has_diagnostic(report: &UiHeadlessRenderDataReport, code: &str) {
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == code),
        "{:?}",
        report.diagnostics()
    );
}

fn load_node(relative_repo_path: &str) -> UiNodeDefinition {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .expect("ui_headless_render_data should live under domain/ui/ui_headless_render_data")
        .to_path_buf();

    let path = repo_root.join(relative_repo_path);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {:?}: {error}", path));

    ron::from_str(&source).expect("fixture should parse as UiNodeDefinition")
}

const TEST_FONT_ID: FontId = FontId(77);

#[derive(Clone, Debug)]
struct TestFontAtlasSource {
    atlas: MsdfFontAtlas,
}

impl Default for TestFontAtlasSource {
    fn default() -> Self {
        let glyphs = (32_u8..=126_u8)
            .map(|byte| {
                (
                    char::from(byte),
                    GlyphMetrics {
                        advance: 8.0,
                        plane_left: 0.0,
                        plane_top: 10.0,
                        plane_right: 7.0,
                        plane_bottom: -2.0,
                        atlas_left: 0.0,
                        atlas_top: 0.0,
                        atlas_right: 1.0,
                        atlas_bottom: 1.0,
                    },
                )
            })
            .collect();

        Self {
            atlas: MsdfFontAtlas {
                font_id: TEST_FONT_ID,
                texture_width: 1,
                texture_height: 1,
                metrics: FontFaceMetrics {
                    ascender: 10.0,
                    descender: -2.0,
                    line_height: 14.0,
                    base_size: 14.0,
                },
                glyphs,
            },
        }
    }
}

impl FontAtlasSource for TestFontAtlasSource {
    fn atlas(&self, font_id: FontId) -> Option<&MsdfFontAtlas> {
        (font_id == TEST_FONT_ID).then_some(&self.atlas)
    }
}
