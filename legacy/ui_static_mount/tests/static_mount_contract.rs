use std::{fs, path::PathBuf};

use ui_artifacts::{UiRuntimeArtifact, UiRuntimeArtifactDiagnostic};
use ui_compiler::UiCompiler;
use ui_controls::{ControlPackageRegistry, runenwerk_control_package};
use ui_definition::UiNodeDefinition;
use ui_headless_render_data::UiHeadlessRenderDataReport;
use ui_program_lowering::form_ui_program_report_from_node_with_registry_snapshot;
use ui_render_primitives::UiRenderPrimitiveReport;
use ui_runtime_view::{ButtonRuntimeHostData, UiRuntimeView};
use ui_static_mount::{DIAGNOSTIC_RENDER_DATA_FAILED, UiStaticMountReport};
use ui_text::{FontAtlasSource, FontFaceMetrics, FontId, GlyphMetrics, MsdfFontAtlas};
use ui_theme::ThemeTokens;

#[test]
fn static_mount_refuses_failed_render_data_report() {
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
    let render_data_report =
        UiHeadlessRenderDataReport::from_render_primitive_report(&primitive_report);

    let mount_report = UiStaticMountReport::from_render_data_report(&render_data_report);

    assert!(!runtime_report.passed());
    assert!(!primitive_report.passed());
    assert!(!render_data_report.passed());
    assert!(!mount_report.passed());
    assert!(mount_report.mounted_frame().is_none());
    assert_has_mount_diagnostic(&mount_report, DIAGNOSTIC_RENDER_DATA_FAILED);
}

#[test]
fn static_mount_accepts_selected_button_render_data_frame() {
    let render_data_report = selected_button_render_data_report();
    let mount_report = UiStaticMountReport::from_render_data_report(&render_data_report);

    assert!(mount_report.passed(), "{:?}", mount_report.diagnostics());
    let mounted = mount_report
        .mounted_frame()
        .expect("static mount should produce a mounted frame");

    assert_eq!(mounted.summary.surface_count, 1);
    assert!(mounted.summary.primitive_count >= 2);
    assert!(mounted.summary.has_rect_primitive);
    assert!(mounted.summary.has_border_primitive);
    assert_eq!(mounted.summary.glyph_run_count, 1);
    assert!(mounted.summary.draw_order_stable);
    assert_eq!(mounted.frame.surfaces[0].size.width, 320.0);
    assert_eq!(mounted.frame.surfaces[0].size.height, 200.0);
}

#[test]
fn static_mount_is_deterministic() {
    let first = UiStaticMountReport::from_render_data_report(&selected_button_render_data_report());
    let second =
        UiStaticMountReport::from_render_data_report(&selected_button_render_data_report());

    assert_eq!(first, second);
}

#[test]
fn static_mount_requires_shaped_static_gallery_text() {
    let render_data_report = selected_button_render_data_report();
    let mount_report = UiStaticMountReport::from_render_data_report(&render_data_report);

    assert!(mount_report.passed(), "{:?}", mount_report.diagnostics());
    assert!(
        render_data_report.diagnostics().is_empty(),
        "{:?}",
        render_data_report.diagnostics()
    );
    assert_eq!(
        mount_report
            .mounted_frame()
            .expect("static mount should produce a mounted frame")
            .summary
            .glyph_run_count,
        1
    );
}

#[test]
fn static_mount_does_not_depend_on_authoring_or_compiler_in_production() {
    let manifest = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("ui_static_mount manifest should be readable");
    let production_dependencies = manifest
        .split("[dev-dependencies]")
        .next()
        .expect("manifest should contain a production section");

    for crate_name in [
        "ui_definition",
        "ui_controls",
        "ui_program_lowering",
        "ui_compiler",
    ] {
        assert!(
            !production_dependencies.contains(crate_name),
            "{crate_name} must not be a production dependency"
        );
    }
}

fn selected_button_render_data_report() -> UiHeadlessRenderDataReport {
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

    let render_data_report =
        UiHeadlessRenderDataReport::from_render_primitive_report(&primitive_report);
    assert!(
        render_data_report.passed(),
        "{:?}",
        render_data_report.diagnostics()
    );
    render_data_report
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

fn assert_has_mount_diagnostic(report: &UiStaticMountReport, code: &str) {
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
        .expect("ui_static_mount should live under domain/ui/ui_static_mount")
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
