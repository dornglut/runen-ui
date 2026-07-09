//! File: domain/ui/ui_text/src/evidence.rs
//! Purpose: Renderer-neutral text layout evidence contracts.

use ui_math::{UiPoint, UiRect, UiSize};

use crate::{
    FontId, TextBlockId, TextDirectionPolicy, TextEllipsisPlacement, TextHorizontalAlign,
    TextRunId, TextSemanticRole, TextSourceRange, TextSpanId, TextStyle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextLayoutDiagnostic {
    pub code: String,
    pub severity: TextDiagnosticSeverity,
    pub message: String,
}

impl TextLayoutDiagnostic {
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: TextDiagnosticSeverity::Warning,
            message: message.into(),
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: TextDiagnosticSeverity::Error,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextClusterRange {
    pub start: u32,
    pub end: u32,
}

impl TextClusterRange {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextCluster {
    pub cluster_index: u32,
    pub run_id: TextRunId,
    pub span_id: Option<TextSpanId>,
    pub source_range: TextSourceRange,
    pub text_preview: String,
    pub style: TextStyle,
    pub semantic_role: Option<TextSemanticRole>,
    pub is_whitespace: bool,
    pub is_newline: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextGlyph {
    pub draw_order: u32,
    pub line_index: u32,
    pub run_id: TextRunId,
    pub span_id: Option<TextSpanId>,
    pub font_id: FontId,
    pub glyph_key: String,
    pub cluster_range: TextClusterRange,
    pub origin: UiPoint,
    pub advance: f32,
    pub bounds: UiRect,
    pub source_text_preview: String,
    pub replacement: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextVisualRun {
    pub visual_run_id: u32,
    pub line_index: u32,
    pub run_id: TextRunId,
    pub span_id: Option<TextSpanId>,
    pub font_id: FontId,
    pub style: TextStyle,
    pub direction: TextDirectionPolicy,
    pub glyphs: Vec<TextGlyph>,
    pub bounds: UiRect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextLineMetrics {
    pub line_index: u32,
    pub source_range: TextClusterRange,
    pub visual_order: u32,
    pub origin: UiPoint,
    pub baseline_y: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub line_height: f32,
    pub line_box: UiRect,
    pub content_width: f32,
    pub ink_bounds: UiRect,
    pub horizontal_align: TextHorizontalAlign,
    pub is_wrapped: bool,
    pub is_explicit_break: bool,
    pub is_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextOverflowEvidence {
    pub horizontal_overflow: bool,
    pub vertical_overflow: bool,
    pub clipped: bool,
    pub ellipsized: bool,
    pub ellipsis_placement: Option<TextEllipsisPlacement>,
    pub max_lines_applied: bool,
    pub omitted_cluster_count: u32,
    pub omitted_run_count: u32,
    pub visible_source_range: TextClusterRange,
}

impl TextOverflowEvidence {
    pub const fn none() -> Self {
        Self {
            horizontal_overflow: false,
            vertical_overflow: false,
            clipped: false,
            ellipsized: false,
            ellipsis_placement: None,
            max_lines_applied: false,
            omitted_cluster_count: 0,
            omitted_run_count: 0,
            visible_source_range: TextClusterRange::new(0, 0),
        }
    }
}

impl Default for TextOverflowEvidence {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFallbackEvidence {
    pub requested_font: FontId,
    pub resolved_font: FontId,
    pub missing_glyph_count: u32,
    pub fallback_glyph_count: u32,
    pub replacement_glyph_count: u32,
}

impl TextFallbackEvidence {
    pub const fn none(font_id: FontId) -> Self {
        Self {
            requested_font: font_id,
            resolved_font: font_id,
            missing_glyph_count: 0,
            fallback_glyph_count: 0,
            replacement_glyph_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlockLayoutResult {
    pub block_id: TextBlockId,
    pub input_run_count: usize,
    pub resolved_run_count: usize,
    pub resolved_cluster_count: usize,
    pub line_count: usize,
    pub glyph_run_count: usize,
    pub glyph_count: usize,
    pub measured_size: UiSize,
    pub content_bounds: UiRect,
    pub ink_bounds: UiRect,
    pub line_metrics: Vec<TextLineMetrics>,
    pub visual_runs: Vec<TextVisualRun>,
    pub clusters: Vec<TextCluster>,
    pub overflow_evidence: TextOverflowEvidence,
    pub fallback_evidence: Vec<TextFallbackEvidence>,
    pub diagnostics: Vec<TextLayoutDiagnostic>,
}
