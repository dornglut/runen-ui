//! Deterministic static mount proof for renderer-facing UI frames.

use ui_headless_render_data::UiHeadlessRenderDataReport;
use ui_render_data::{UiFrame, UiPrimitive, UiSortKey};

pub const DIAGNOSTIC_RENDER_DATA_FAILED: &str = "ui.static_mount.render_data_failed";
pub const DIAGNOSTIC_FRAME_MISSING: &str = "ui.static_mount.frame_missing";
pub const DIAGNOSTIC_SURFACE_MISSING: &str = "ui.static_mount.surface_missing";
pub const DIAGNOSTIC_PRIMITIVE_MISSING: &str = "ui.static_mount.primitive_missing";

#[derive(Clone, Debug, PartialEq)]
pub struct UiStaticMountReport {
    mounted_frame: Option<MountedStaticUiFrame>,
    diagnostics: Vec<UiStaticMountDiagnostic>,
}

impl UiStaticMountReport {
    /// Builds static mount evidence from a headless render-data report.
    ///
    /// This path first requires the headless report to pass, then validates the
    /// renderer-neutral `UiFrame` through the same frame validator used by
    /// direct proof adapters.
    pub fn from_render_data_report(report: &UiHeadlessRenderDataReport) -> Self {
        if !report.passed() {
            return Self {
                mounted_frame: None,
                diagnostics: vec![UiStaticMountDiagnostic::error(
                    DIAGNOSTIC_RENDER_DATA_FAILED,
                    "static mount refused failed render-data report",
                )],
            };
        }

        let Some(frame) = report.frame() else {
            return Self {
                mounted_frame: None,
                diagnostics: vec![UiStaticMountDiagnostic::error(
                    DIAGNOSTIC_FRAME_MISSING,
                    "static mount requires a render-data frame",
                )],
            };
        };

        Self::from_frame(frame.clone())
    }

    /// Builds static mount evidence directly from a renderer-neutral frame.
    ///
    /// This is for proof adapters that already produce `UiFrame` and do not
    /// need to go through headless render-data generation. The validation still
    /// requires a surface, primitives, background/rect evidence, border/outline
    /// evidence, and stable draw order.
    pub fn from_frame(frame: UiFrame) -> Self {
        let summary = StaticMountSummary::from_frame(&frame);
        let mut diagnostics = Vec::new();
        if summary.surface_count == 0 {
            diagnostics.push(UiStaticMountDiagnostic::error(
                DIAGNOSTIC_SURFACE_MISSING,
                "static mount requires at least one surface",
            ));
        }
        if summary.primitive_count == 0 {
            diagnostics.push(UiStaticMountDiagnostic::error(
                DIAGNOSTIC_PRIMITIVE_MISSING,
                "static mount requires at least one primitive",
            ));
        }
        if !summary.has_rect_primitive {
            diagnostics.push(UiStaticMountDiagnostic::error(
                DIAGNOSTIC_PRIMITIVE_MISSING,
                "static mount requires a rectangle/background primitive",
            ));
        }
        if !summary.has_border_primitive {
            diagnostics.push(UiStaticMountDiagnostic::error(
                DIAGNOSTIC_PRIMITIVE_MISSING,
                "static mount requires a border/outline primitive",
            ));
        }
        if !summary.draw_order_stable {
            diagnostics.push(UiStaticMountDiagnostic::error(
                DIAGNOSTIC_PRIMITIVE_MISSING,
                "static mount requires primitives to preserve stable draw order",
            ));
        }

        let mounted_frame = diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != UiStaticMountDiagnosticSeverity::Error)
            .then(|| MountedStaticUiFrame { frame, summary });

        Self {
            mounted_frame,
            diagnostics,
        }
    }

    pub fn passed(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != UiStaticMountDiagnosticSeverity::Error)
    }

    pub fn mounted_frame(&self) -> Option<&MountedStaticUiFrame> {
        self.mounted_frame.as_ref()
    }

    pub fn diagnostics(&self) -> &[UiStaticMountDiagnostic] {
        &self.diagnostics
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MountedStaticUiFrame {
    pub frame: UiFrame,
    pub summary: StaticMountSummary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticMountSummary {
    pub surface_count: usize,
    pub primitive_count: usize,
    pub has_rect_primitive: bool,
    pub has_border_primitive: bool,
    pub glyph_run_count: usize,
    pub draw_order_stable: bool,
}

impl StaticMountSummary {
    pub fn from_frame(frame: &UiFrame) -> Self {
        let mut sort_keys = Vec::new();
        let mut primitive_count = 0usize;
        let mut has_rect_primitive = false;
        let mut has_border_primitive = false;
        let mut glyph_run_count = 0usize;

        for primitive in frame
            .surfaces
            .iter()
            .flat_map(|surface| surface.layers.iter())
            .flat_map(|layer| layer.primitives.iter())
        {
            primitive_count += 1;
            sort_keys.push(primitive_sort_key(primitive));
            match primitive {
                UiPrimitive::Rect(_) => has_rect_primitive = true,
                UiPrimitive::Border(_) => has_border_primitive = true,
                UiPrimitive::GlyphRun(_) => glyph_run_count += 1,
                UiPrimitive::Image(_)
                | UiPrimitive::Stroke(_)
                | UiPrimitive::ViewportSurfaceEmbed(_)
                | UiPrimitive::ProductSurface(_)
                | UiPrimitive::Clip(_) => {}
            }
        }

        let mut sorted_sort_keys = sort_keys.clone();
        sorted_sort_keys.sort();

        Self {
            surface_count: frame.surfaces.len(),
            primitive_count,
            has_rect_primitive,
            has_border_primitive,
            glyph_run_count,
            draw_order_stable: sort_keys == sorted_sort_keys,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiStaticMountDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: UiStaticMountDiagnosticSeverity,
}

impl UiStaticMountDiagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: UiStaticMountDiagnosticSeverity::Error,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiStaticMountDiagnosticSeverity {
    Info,
    Warning,
    Error,
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
