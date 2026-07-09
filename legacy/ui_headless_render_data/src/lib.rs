//! Adapter from deterministic headless UI render reports into renderer-facing UI render data.

use std::collections::BTreeMap;

use ui_headless_render::{
    HeadlessColor, HeadlessRect, HeadlessRenderReport, HeadlessSortKey, HeadlessUiPrimitive,
};
use ui_math::{UiRect, UiSize};
use ui_render_data::{
    BorderPrimitive, RectPrimitive, UiDrawKey, UiFrame, UiLayer, UiLayerId, UiPaint, UiPrimitive,
    UiSortKey, UiSurface, UiSurfaceId,
};
use ui_render_primitives::{UiRenderPrimitiveDiagnosticSeverity, UiRenderPrimitiveReport};

pub const DIAGNOSTIC_HEADLESS_REPORT_FAILED: &str =
    "ui.headless_render_data.headless_report_failed";
pub const DIAGNOSTIC_HEADLESS_FRAME_MISSING: &str =
    "ui.headless_render_data.headless_frame_missing";
pub const DIAGNOSTIC_RENDER_PRIMITIVE_REPORT_FAILED: &str =
    "ui.headless_render_data.render_primitive_report_failed";
pub const DIAGNOSTIC_RENDER_PRIMITIVE_FRAME_MISSING: &str =
    "ui.headless_render_data.render_primitive_frame_missing";
pub const DIAGNOSTIC_TEXT_PLACEHOLDER_SHAPING: &str =
    "ui.headless_render_data.text_placeholder_shaping";

#[derive(Clone, Debug, PartialEq)]
pub struct UiHeadlessRenderDataReport {
    frame: Option<UiFrame>,
    diagnostics: Vec<UiHeadlessRenderDataDiagnostic>,
    provenance: Vec<UiHeadlessRenderDataPrimitiveProvenance>,
}

impl UiHeadlessRenderDataReport {
    pub fn from_render_primitive_report(report: &UiRenderPrimitiveReport) -> Self {
        if !report.passed() {
            return Self {
                frame: None,
                diagnostics: report
                    .diagnostics()
                    .iter()
                    .map(|diagnostic| UiHeadlessRenderDataDiagnostic {
                        code: diagnostic.code.clone(),
                        message: diagnostic.message.clone(),
                        severity: match diagnostic.severity {
                            UiRenderPrimitiveDiagnosticSeverity::Info => {
                                UiHeadlessRenderDataDiagnosticSeverity::Info
                            }
                            UiRenderPrimitiveDiagnosticSeverity::Warning => {
                                UiHeadlessRenderDataDiagnosticSeverity::Warning
                            }
                            UiRenderPrimitiveDiagnosticSeverity::Error => {
                                UiHeadlessRenderDataDiagnosticSeverity::Error
                            }
                        },
                        source_map_index: diagnostic.source_map_index,
                    })
                    .chain(std::iter::once(UiHeadlessRenderDataDiagnostic::error(
                        DIAGNOSTIC_RENDER_PRIMITIVE_REPORT_FAILED,
                        "render-data adapter refused failed render primitive report",
                        None,
                    )))
                    .collect(),
                provenance: Vec::new(),
            };
        }

        let Some(frame) = report.frame() else {
            return Self {
                frame: None,
                diagnostics: vec![UiHeadlessRenderDataDiagnostic::error(
                    DIAGNOSTIC_RENDER_PRIMITIVE_FRAME_MISSING,
                    "render-data adapter expected a passed render primitive report to contain a frame",
                    None,
                )],
                provenance: Vec::new(),
            };
        };

        Self {
            frame: Some(frame.clone()),
            diagnostics: Vec::new(),
            provenance: report
                .provenance()
                .iter()
                .map(|entry| UiHeadlessRenderDataPrimitiveProvenance {
                    surface_index: entry.surface_index,
                    layer_index: entry.layer_index,
                    primitive_index: entry.primitive_index,
                    source_map_index: entry.source_map_index,
                })
                .collect(),
        }
    }

    pub fn from_headless_report(report: &HeadlessRenderReport) -> Self {
        if !report.passed() {
            return Self {
                frame: None,
                diagnostics: vec![UiHeadlessRenderDataDiagnostic::error(
                    DIAGNOSTIC_HEADLESS_REPORT_FAILED,
                    "render-data adapter refused failed headless render report",
                    None,
                )],
                provenance: Vec::new(),
            };
        }

        let Some(headless_frame) = report.frame() else {
            return Self {
                frame: None,
                diagnostics: vec![UiHeadlessRenderDataDiagnostic::error(
                    DIAGNOSTIC_HEADLESS_FRAME_MISSING,
                    "render-data adapter expected a passed headless report to contain a frame",
                    None,
                )],
                provenance: Vec::new(),
            };
        };

        let mut diagnostics = Vec::new();
        let mut primitive_records = Vec::new();

        for primitive in &headless_frame.primitives {
            match primitive {
                HeadlessUiPrimitive::Rectangle {
                    bounds,
                    radius,
                    color,
                    sort_key,
                    source_map_index,
                    ..
                } => primitive_records.push(PrimitiveRecord {
                    sort_key: *sort_key,
                    primitive: UiPrimitive::Rect(RectPrimitive::new(
                        rect_from_headless(*bounds),
                        *radius,
                        paint_from_headless(*color),
                        UiDrawKey::new(1, None),
                        sort_key_from_headless(*sort_key),
                    )),
                    source_map_index: *source_map_index,
                }),
                HeadlessUiPrimitive::Border {
                    bounds,
                    radius,
                    width,
                    color,
                    sort_key,
                    source_map_index,
                    ..
                } => primitive_records.push(PrimitiveRecord {
                    sort_key: *sort_key,
                    primitive: UiPrimitive::Border(BorderPrimitive::new(
                        rect_from_headless(*bounds),
                        *radius,
                        *width,
                        paint_from_headless(*color),
                        UiDrawKey::new(2, None),
                        sort_key_from_headless(*sort_key),
                    )),
                    source_map_index: *source_map_index,
                }),
                HeadlessUiPrimitive::Text {
                    primitive_id,
                    source_map_index,
                    ..
                } => diagnostics.push(UiHeadlessRenderDataDiagnostic::warning(
                    DIAGNOSTIC_TEXT_PLACEHOLDER_SHAPING,
                    format!(
                        "text primitive {primitive_id} was not converted because ui_render_data text primitives require shaped glyph runs"
                    ),
                    *source_map_index,
                )),
            }
        }

        primitive_records.sort_by_key(|record| record.sort_key);

        let mut layers = BTreeMap::<u32, Vec<PrimitiveRecord>>::new();
        for record in primitive_records {
            layers
                .entry(record.sort_key.layer_order)
                .or_default()
                .push(record);
        }

        let mut provenance = Vec::new();
        let render_layers = layers
            .into_iter()
            .enumerate()
            .map(|(layer_index, (layer_order, records))| {
                let primitives = records
                    .into_iter()
                    .enumerate()
                    .map(|(primitive_index, record)| {
                        if let Some(source_map_index) = record.source_map_index {
                            provenance.push(UiHeadlessRenderDataPrimitiveProvenance {
                                surface_index: 0,
                                layer_index,
                                primitive_index,
                                source_map_index,
                            });
                        }
                        record.primitive
                    })
                    .collect();

                UiLayer::with_primitives(UiLayerId(layer_order as u64), primitives)
            })
            .collect();

        let frame = UiFrame::with_surfaces(vec![UiSurface::with_layers(
            UiSurfaceId(0),
            UiSize::new(
                headless_frame.viewport.width,
                headless_frame.viewport.height,
            ),
            render_layers,
        )]);

        Self {
            frame: Some(frame),
            diagnostics,
            provenance,
        }
    }

    pub fn passed(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != UiHeadlessRenderDataDiagnosticSeverity::Error)
    }

    pub fn frame(&self) -> Option<&UiFrame> {
        self.frame.as_ref()
    }

    pub fn diagnostics(&self) -> &[UiHeadlessRenderDataDiagnostic] {
        &self.diagnostics
    }

    pub fn provenance(&self) -> &[UiHeadlessRenderDataPrimitiveProvenance] {
        &self.provenance
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiHeadlessRenderDataDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: UiHeadlessRenderDataDiagnosticSeverity,
    pub source_map_index: Option<u32>,
}

impl UiHeadlessRenderDataDiagnostic {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        source_map_index: Option<u32>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: UiHeadlessRenderDataDiagnosticSeverity::Error,
            source_map_index,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        source_map_index: Option<u32>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: UiHeadlessRenderDataDiagnosticSeverity::Warning,
            source_map_index,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiHeadlessRenderDataDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiHeadlessRenderDataPrimitiveProvenance {
    pub surface_index: usize,
    pub layer_index: usize,
    pub primitive_index: usize,
    pub source_map_index: u32,
}

#[derive(Debug)]
struct PrimitiveRecord {
    sort_key: HeadlessSortKey,
    primitive: UiPrimitive,
    source_map_index: Option<u32>,
}

fn sort_key_from_headless(sort_key: HeadlessSortKey) -> UiSortKey {
    UiSortKey::new(0, sort_key.layer_order, sort_key.primitive_order)
}

fn rect_from_headless(rect: HeadlessRect) -> UiRect {
    UiRect::new(rect.x, rect.y, rect.width, rect.height)
}

fn paint_from_headless(color: HeadlessColor) -> UiPaint {
    UiPaint::rgba(color.r, color.g, color.b, color.a)
}
