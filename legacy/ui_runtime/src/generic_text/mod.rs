//! Renderer-neutral generic text runtime proof.
//!
//! Generic Text runtime proof consumes package-backed text-display declarations
//! and deterministic `ui_text` layout evidence. It emits source, layout,
//! overflow, projection, static-mount, and no-bypass evidence without owning a
//! renderer backend, product copy store, text editor, authored UI mutation, or
//! plugin framework.

use std::collections::HashMap;

use ui_controls::{ControlInspectionSection, runenwerk_control_package};
use ui_math::{UiRect, UiSize};
use ui_render_data::{
    BorderPrimitive, GlyphRunPrimitive, RectPrimitive, UiDrawKey, UiFrame, UiLayer, UiLayerId,
    UiPaint, UiPrimitive, UiSortKey, UiSurface, UiSurfaceId,
};
use ui_text::{
    AtlasTextLayouter, FontAtlasSource, FontFaceMetrics, FontId, GlyphMetrics, MsdfFontAtlas,
    TextBlock, TextBlockLayoutRequest, TextBlockLayoutResult, TextDecoration,
    TextEllipsisPlacement, TextHorizontalAlign, TextLayoutPolicy, TextLayouter, TextOverflowPolicy,
    TextSemanticRole, TextSourceRange, TextSpan, TextSpanId, TextSpanStyle, TextWhitespacePolicy,
    TextWidthConstraint, TextWrapPolicy,
};

pub const BASE_CONTROLS_GENERIC_TEXT_PROOF_ID: &str = "base-controls.generic-text.proof";

#[derive(Debug, Clone, PartialEq)]
pub struct GenericTextProofReport {
    pub proof_id: String,
    pub descriptor_evidence: Vec<String>,
    pub source_block_evidence: Vec<String>,
    pub layout_request_evidence: Vec<String>,
    pub layout_result_evidence: Vec<TextBlockLayoutResult>,
    pub line_metric_evidence: Vec<String>,
    pub glyph_run_evidence: Vec<String>,
    pub overflow_evidence: Vec<String>,
    pub catalog_projection_evidence: Vec<String>,
    pub inspection_projection_evidence: Vec<String>,
    pub static_mount_expectations: Vec<String>,
    pub boundary_assertions: GenericTextBoundaryAssertions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GenericTextBoundaryAssertions {
    pub host_commands_executed: u32,
    pub product_mutations: u32,
    pub authored_ui_edits: u32,
    pub product_undo_redo_operations: u32,
    pub plugin_framework_operations: u32,
    pub renderer_backend_operations: u32,
}

impl GenericTextBoundaryAssertions {
    pub const fn no_bypass_evidence(self) -> bool {
        self.host_commands_executed == 0
            && self.product_mutations == 0
            && self.authored_ui_edits == 0
            && self.product_undo_redo_operations == 0
            && self.plugin_framework_operations == 0
            && self.renderer_backend_operations == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericTextProofRenderFrame {
    pub proof_id: String,
    pub frame: UiFrame,
    pub summary: GenericTextProofRenderSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericTextProofRenderSummary {
    pub source_blocks: usize,
    pub source_runs: usize,
    pub inline_spans: usize,
    pub line_count: usize,
    pub glyph_run_count: usize,
    pub glyph_count: usize,
    pub wrapped_lines: usize,
    pub aligned_lines: usize,
    pub truncated_lines: usize,
    pub fallback_rows: usize,
    pub catalog_rows: usize,
    pub inspection_rows: usize,
    pub has_source_layout_and_evidence_panels: bool,
    pub no_bypass_proven: bool,
}

pub fn base_controls_generic_text_report() -> GenericTextProofReport {
    let package = runenwerk_control_package();
    let catalog = ui_controls::ControlCatalogIndex::from_packages([&package]);
    let inspection = ui_controls::BaseControlsPlugin::new().inspection();
    let atlas = ProofAtlasSource::new();
    let layouter = AtlasTextLayouter;
    let blocks = generic_text_fixture_blocks();
    let layouts = blocks
        .iter()
        .cloned()
        .map(|block| layouter.layout(&atlas, TextBlockLayoutRequest::new(block)))
        .collect::<Vec<_>>();

    let descriptor_evidence = package
        .generic_text_descriptors
        .iter()
        .map(|descriptor| {
            format!(
                "{}:{}",
                descriptor.control_kind_id.as_str(),
                descriptor.roles.len()
            )
        })
        .collect::<Vec<_>>();
    let source_block_evidence = blocks
        .iter()
        .map(|block| format!("block:{} runs:{}", block.text_block_id.0, block.runs.len()))
        .collect::<Vec<_>>();
    let layout_request_evidence = blocks
        .iter()
        .map(|block| {
            format!(
                "wrap:{} align:{}",
                block.layout.wrap.as_str(),
                block.layout.horizontal_align.as_str()
            )
        })
        .collect::<Vec<_>>();
    let line_metric_evidence = layouts
        .iter()
        .flat_map(|layout| {
            layout.line_metrics.iter().map(|line| {
                format!(
                    "line:{} baseline:{} width:{}",
                    line.line_index, line.baseline_y, line.content_width
                )
            })
        })
        .collect::<Vec<_>>();
    let glyph_run_evidence = layouts
        .iter()
        .flat_map(|layout| {
            layout.visual_runs.iter().map(|run| {
                format!(
                    "visual-run:{} glyphs:{}",
                    run.visual_run_id,
                    run.glyphs.len()
                )
            })
        })
        .collect::<Vec<_>>();
    let overflow_evidence = layouts
        .iter()
        .map(|layout| {
            format!(
                "clip:{} ellipsis:{} omitted:{}",
                layout.overflow_evidence.clipped,
                layout.overflow_evidence.ellipsized,
                layout.overflow_evidence.omitted_cluster_count
            )
        })
        .collect::<Vec<_>>();
    let catalog_projection_evidence = catalog
        .entries
        .iter()
        .filter(|entry| entry.generic_text_supported)
        .map(|entry| {
            format!(
                "{} roles:{} inline:{}",
                entry.control_kind_id,
                entry.text_roles.len(),
                entry.inline_spans_supported
            )
        })
        .collect::<Vec<_>>();
    let inspection_projection_evidence = inspection
        .controls
        .iter()
        .filter_map(|control| {
            control
                .fact(
                    ControlInspectionSection::TextDisplay,
                    "text_display.supported",
                )
                .map(|value| format!("{}:{}", control.control_kind_id, value))
        })
        .collect::<Vec<_>>();

    GenericTextProofReport {
        proof_id: BASE_CONTROLS_GENERIC_TEXT_PROOF_ID.to_owned(),
        descriptor_evidence,
        source_block_evidence,
        layout_request_evidence,
        layout_result_evidence: layouts,
        line_metric_evidence,
        glyph_run_evidence,
        overflow_evidence,
        catalog_projection_evidence,
        inspection_projection_evidence,
        static_mount_expectations: vec![
            "source-panel".to_owned(),
            "layout-panel".to_owned(),
            "evidence-panel".to_owned(),
            "renderer-neutral-glyph-run-primitive".to_owned(),
        ],
        boundary_assertions: GenericTextBoundaryAssertions::default(),
    }
}

pub fn base_controls_generic_text_proof_frame() -> GenericTextProofRenderFrame {
    generic_text_report_to_frame(base_controls_generic_text_report())
}

pub fn generic_text_report_to_frame(report: GenericTextProofReport) -> GenericTextProofRenderFrame {
    let mut primitives = Vec::new();
    let mut order = 0_u32;
    panel(
        &mut primitives,
        &mut order,
        UiRect::new(16.0, 16.0, 300.0, 600.0),
    );
    panel(
        &mut primitives,
        &mut order,
        UiRect::new(332.0, 16.0, 300.0, 600.0),
    );
    panel(
        &mut primitives,
        &mut order,
        UiRect::new(648.0, 16.0, 300.0, 600.0),
    );
    for (index, layout) in report.layout_result_evidence.iter().cloned().enumerate() {
        primitives.push(
            GlyphRunPrimitive::new(
                layout,
                Some(UiRect::new(
                    28.0 + (index % 3) as f32 * 316.0,
                    60.0 + (index / 3) as f32 * 52.0,
                    260.0,
                    40.0,
                )),
                UiPaint::WHITE,
                UiDrawKey::new(1503, None),
                sort_key(&mut order),
            )
            .into(),
        );
    }
    let mut surface = UiSurface::new(UiSurfaceId(15), UiSize::new(964.0, 640.0));
    surface.push_layer(UiLayer::with_primitives(UiLayerId(0), primitives));
    let summary = render_summary(&report);
    GenericTextProofRenderFrame {
        proof_id: report.proof_id,
        frame: UiFrame::with_surfaces(vec![surface]),
        summary,
    }
}

fn render_summary(report: &GenericTextProofReport) -> GenericTextProofRenderSummary {
    let source_blocks = report.source_block_evidence.len();
    let source_runs = report
        .layout_result_evidence
        .iter()
        .map(|layout| layout.input_run_count)
        .sum();
    let inline_spans = report
        .layout_result_evidence
        .iter()
        .flat_map(|layout| layout.visual_runs.iter())
        .flat_map(|run| run.glyphs.iter())
        .filter(|glyph| glyph.span_id.is_some())
        .count();
    let line_count = report
        .layout_result_evidence
        .iter()
        .map(|layout| layout.line_count)
        .sum();
    let glyph_run_count = report
        .layout_result_evidence
        .iter()
        .map(|layout| layout.glyph_run_count)
        .sum();
    let glyph_count = report
        .layout_result_evidence
        .iter()
        .map(|layout| layout.glyph_count)
        .sum();
    let wrapped_lines = report
        .layout_result_evidence
        .iter()
        .flat_map(|layout| layout.line_metrics.iter())
        .filter(|line| line.is_wrapped)
        .count();
    let aligned_lines = report
        .layout_result_evidence
        .iter()
        .flat_map(|layout| layout.line_metrics.iter())
        .filter(|line| line.horizontal_align != TextHorizontalAlign::Start)
        .count();
    let truncated_lines = report
        .layout_result_evidence
        .iter()
        .flat_map(|layout| layout.line_metrics.iter())
        .filter(|line| line.is_truncated)
        .count();
    let fallback_rows = report
        .layout_result_evidence
        .iter()
        .flat_map(|layout| layout.fallback_evidence.iter())
        .filter(|row| row.replacement_glyph_count > 0)
        .count();
    GenericTextProofRenderSummary {
        source_blocks,
        source_runs,
        inline_spans,
        line_count,
        glyph_run_count,
        glyph_count,
        wrapped_lines,
        aligned_lines,
        truncated_lines,
        fallback_rows,
        catalog_rows: report.catalog_projection_evidence.len(),
        inspection_rows: report.inspection_projection_evidence.len(),
        has_source_layout_and_evidence_panels: report
            .static_mount_expectations
            .iter()
            .any(|row| row == "source-panel")
            && report
                .static_mount_expectations
                .iter()
                .any(|row| row == "layout-panel")
            && report
                .static_mount_expectations
                .iter()
                .any(|row| row == "evidence-panel"),
        no_bypass_proven: report.boundary_assertions.no_bypass_evidence(),
    }
}

fn generic_text_fixture_blocks() -> Vec<TextBlock> {
    vec![
        TextBlock::label("Simple label"),
        TextBlock::inline_spans(
            "Heading body helper",
            vec![
                TextSpan::new(TextSpanId(1), TextSourceRange::new(0, 7))
                    .with_style(
                        TextSpanStyle::inherit().with_font_weight(ui_text::TextFontWeight::Bold),
                    )
                    .with_semantic_role(TextSemanticRole::Heading),
                TextSpan::new(TextSpanId(2), TextSourceRange::new(8, 12))
                    .with_semantic_role(TextSemanticRole::Body),
                TextSpan::new(TextSpanId(3), TextSourceRange::new(13, 19))
                    .with_style(
                        TextSpanStyle::inherit().with_decoration(TextDecoration::underline()),
                    )
                    .with_semantic_role(TextSemanticRole::Helper),
            ],
        ),
        TextBlock::body("alpha beta gamma delta", 54.0),
        TextBlock::body("aaaaaaaaaaaa", 36.0).with_layout(TextLayoutPolicy {
            wrap: TextWrapPolicy::Character,
            width_constraint: TextWidthConstraint::Max(36.0),
            ..TextLayoutPolicy::default()
        }),
        TextBlock::label("center").with_layout(TextLayoutPolicy {
            width_constraint: TextWidthConstraint::Exact(100.0),
            horizontal_align: TextHorizontalAlign::Center,
            ..TextLayoutPolicy::default()
        }),
        TextBlock::label("end").with_layout(TextLayoutPolicy {
            width_constraint: TextWidthConstraint::Exact(100.0),
            horizontal_align: TextHorizontalAlign::End,
            ..TextLayoutPolicy::default()
        }),
        TextBlock::label("clip overflow text").with_layout(TextLayoutPolicy {
            width_constraint: TextWidthConstraint::Max(40.0),
            overflow: TextOverflowPolicy::Clip,
            ..TextLayoutPolicy::default()
        }),
        TextBlock::label("ellipsis overflow text").with_layout(TextLayoutPolicy {
            width_constraint: TextWidthConstraint::Max(56.0),
            overflow: TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::End),
            ..TextLayoutPolicy::default()
        }),
        TextBlock::body("one two three four five six", 48.0).with_layout(TextLayoutPolicy {
            width_constraint: TextWidthConstraint::Max(48.0),
            wrap: TextWrapPolicy::Word,
            whitespace: TextWhitespacePolicy::CollapseRuns,
            max_lines: Some(2),
            overflow: TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::End),
            ..TextLayoutPolicy::default()
        }),
        TextBlock::label("fallback Ω"),
        TextBlock::label("line one\nline two"),
    ]
}

fn panel(primitives: &mut Vec<UiPrimitive>, order: &mut u32, area: UiRect) {
    primitives.push(
        RectPrimitive::new(
            area,
            3.0,
            UiPaint::rgba(0.10, 0.11, 0.13, 1.0),
            UiDrawKey::new(1501, None),
            sort_key(order),
        )
        .into(),
    );
    primitives.push(
        BorderPrimitive::new(
            area,
            3.0,
            1.0,
            UiPaint::WHITE,
            UiDrawKey::new(1502, None),
            sort_key(order),
        )
        .into(),
    );
}
fn sort_key(order: &mut u32) -> UiSortKey {
    let key = UiSortKey::new(0, 0, *order);
    *order += 1;
    key
}

#[derive(Default)]
struct ProofAtlasSource {
    atlas: MsdfFontAtlas,
}
impl ProofAtlasSource {
    fn new() -> Self {
        let mut glyphs = HashMap::new();
        for ch in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 .-:?\n".chars() {
            glyphs.insert(
                ch,
                GlyphMetrics {
                    advance: if ch == ' ' { 4.0 } else { 8.0 },
                    plane_left: 0.0,
                    plane_top: 8.0,
                    plane_right: 8.0,
                    plane_bottom: -2.0,
                    atlas_left: 0.0,
                    atlas_top: 0.0,
                    atlas_right: 0.1,
                    atlas_bottom: 0.1,
                },
            );
        }
        Self {
            atlas: MsdfFontAtlas {
                font_id: FontId(0),
                texture_width: 256,
                texture_height: 256,
                metrics: FontFaceMetrics {
                    ascender: 9.0,
                    descender: -3.0,
                    line_height: 12.0,
                    base_size: 12.0,
                },
                glyphs,
            },
        }
    }
}
impl FontAtlasSource for ProofAtlasSource {
    fn atlas(&self, _font_id: FontId) -> Option<&MsdfFontAtlas> {
        Some(&self.atlas)
    }
}
