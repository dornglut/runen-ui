//! File: domain/ui/ui_render_data/src/primitives/glyph_run.rs
//! Purpose: Renderer-neutral text visual-run primitive referencing layout evidence.

use ui_math::UiRect;
use ui_text::{TextBlockId, TextBlockLayoutResult, TextOverflowEvidence, TextVisualRun};

use crate::{UiDrawKey, UiPaint, UiSortKey};

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphRunPrimitive {
    pub block_id: TextBlockId,
    pub visual_runs: Vec<TextVisualRun>,
    pub line_count: usize,
    pub glyph_count: usize,
    pub overflow_evidence: TextOverflowEvidence,
    pub baseline_origin_clip: Option<UiRect>,
    pub tint: UiPaint,
    pub draw_key: UiDrawKey,
    pub sort_key: UiSortKey,
}

impl GlyphRunPrimitive {
    pub fn new(
        layout: TextBlockLayoutResult,
        baseline_origin_clip: Option<UiRect>,
        tint: UiPaint,
        draw_key: UiDrawKey,
        sort_key: UiSortKey,
    ) -> Self {
        Self {
            block_id: layout.block_id,
            visual_runs: layout.visual_runs,
            line_count: layout.line_count,
            glyph_count: layout.glyph_count,
            overflow_evidence: layout.overflow_evidence,
            baseline_origin_clip,
            tint,
            draw_key,
            sort_key,
        }
    }

    pub fn from_visual_runs(
        block_id: TextBlockId,
        visual_runs: Vec<TextVisualRun>,
        overflow_evidence: TextOverflowEvidence,
        baseline_origin_clip: Option<UiRect>,
        tint: UiPaint,
        draw_key: UiDrawKey,
        sort_key: UiSortKey,
    ) -> Self {
        let line_count = visual_runs
            .iter()
            .map(|run| run.line_index)
            .max()
            .map_or(0, |index| index as usize + 1);
        let glyph_count = visual_runs.iter().map(|run| run.glyphs.len()).sum();
        Self {
            block_id,
            visual_runs,
            line_count,
            glyph_count,
            overflow_evidence,
            baseline_origin_clip,
            tint,
            draw_key,
            sort_key,
        }
    }
}
