//! File: domain/ui/ui_text/src/proof_layout.rs
//! Purpose: Deterministic renderer-neutral proof text layout.

use ui_math::{UiPoint, UiRect, UiSize};

use crate::{
    FontAtlasSource, FontId, GlyphMetrics, TextBlock, TextBlockLayoutRequest,
    TextBlockLayoutResult, TextCluster, TextClusterRange, TextDirectionPolicy,
    TextEllipsisPlacement, TextFallbackEvidence, TextGlyph, TextHorizontalAlign,
    TextLayoutDiagnostic, TextLineMetrics, TextOverflowEvidence, TextOverflowPolicy, TextRunId,
    TextSourceRange, TextStyle, TextVisualRun, TextWhitespacePolicy, TextWrapPolicy,
};

#[derive(Clone)]
struct ProofGlyph {
    cluster: TextCluster,
    glyph_key: String,
    advance: f32,
    ascent: f32,
    descent: f32,
    line_height: f32,
    replacement: bool,
}

#[derive(Clone, Default)]
struct ProofLine {
    glyphs: Vec<ProofGlyph>,
    wrapped_from_previous: bool,
    explicit_break: bool,
}

pub fn layout_text_block(
    atlas_source: &dyn FontAtlasSource,
    request: TextBlockLayoutRequest,
) -> TextBlockLayoutResult {
    let block = request.block;
    let mut diagnostics = Vec::new();
    if block.runs.is_empty() {
        diagnostics.push(TextLayoutDiagnostic::error(
            "ui.text.block.empty",
            "text block must contain at least one source run",
        ));
        return empty_result(block, diagnostics);
    }
    if block.layout.text_direction == TextDirectionPolicy::Auto {
        diagnostics.push(TextLayoutDiagnostic::warning(
            "ui.text.direction.auto_uses_ltr_proof",
            "auto direction is modeled and resolved as ltr in deterministic proof layout",
        ));
    }
    if block.layout.text_direction == TextDirectionPolicy::Rtl {
        diagnostics.push(TextLayoutDiagnostic::warning(
            "ui.text.direction.rtl_unsupported",
            "rtl direction is modeled but deterministic proof layout emits ltr visual order",
        ));
    }
    if matches!(
        block.layout.overflow,
        TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::Start | TextEllipsisPlacement::Middle)
    ) {
        diagnostics.push(TextLayoutDiagnostic::warning(
            "ui.text.ellipsis.unsupported_placement",
            "start and middle ellipsis are modeled but deterministic proof layout only applies end ellipsis",
        ));
    }

    let mut fallback = TextFallbackEvidence::none(block.base_style.font_id);
    let mut source = lower_source(&block, atlas_source, &mut fallback, &mut diagnostics);
    if block.layout.whitespace == TextWhitespacePolicy::CollapseRuns {
        collapse_horizontal_spaces(&mut source);
    }
    let width_limit = block.layout.width_constraint.limit();
    let mut lines = break_lines(&block, &source, width_limit, &mut diagnostics);
    if block.layout.whitespace == TextWhitespacePolicy::TrimEdges {
        trim_line_edges(&mut lines);
    }
    let max_lines_applied = block
        .layout
        .max_lines
        .is_some_and(|max_lines| lines.len() > max_lines as usize);
    if let Some(max_lines) = block.layout.max_lines {
        if max_lines == 0 {
            diagnostics.push(TextLayoutDiagnostic::error(
                "ui.text.max_lines.zero",
                "max_lines must be absent or greater than zero",
            ));
        } else if lines.len() > max_lines as usize {
            lines.truncate(max_lines as usize);
        }
    }

    let content_height = lines.iter().map(line_height).sum::<f32>();
    let horizontal_overflow = width_limit.is_some_and(|limit| {
        lines
            .iter()
            .any(|line| line_width(&line.glyphs) > limit + f32::EPSILON)
    });
    let height_overflow = block
        .layout
        .height_constraint
        .limit()
        .is_some_and(|limit| content_height > limit + f32::EPSILON);
    let mut overflow = TextOverflowEvidence::none();
    overflow.horizontal_overflow = horizontal_overflow;
    overflow.vertical_overflow = max_lines_applied || height_overflow;
    overflow.max_lines_applied = max_lines_applied;
    overflow.omitted_cluster_count = source
        .len()
        .saturating_sub(lines.iter().map(|line| line.glyphs.len()).sum())
        as u32;
    overflow.omitted_run_count = omitted_run_count(&block, &lines);

    let end_ellipsis = matches!(
        block.layout.overflow,
        TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::End)
    );
    if end_ellipsis && (horizontal_overflow || max_lines_applied) {
        if let Some(last_line) = lines.last_mut() {
            let ellipsis_limit = width_limit.unwrap_or_else(|| line_width(&last_line.glyphs));
            apply_end_ellipsis(&mut last_line.glyphs, ellipsis_limit, atlas_source);
        }
        overflow.ellipsized = true;
        overflow.ellipsis_placement = Some(TextEllipsisPlacement::End);
    } else if matches!(block.layout.overflow, TextOverflowPolicy::Clip)
        && (horizontal_overflow || max_lines_applied || height_overflow)
    {
        overflow.clipped = true;
    }
    if height_overflow {
        overflow.clipped = true;
    }
    overflow.visible_source_range = visible_range(&lines);

    let resolved_width = block.layout.width_constraint.resolved_width(
        lines
            .iter()
            .map(|line| line_width(&line.glyphs))
            .fold(0.0, f32::max),
    );
    let line_metrics = line_metrics(&block, &lines, resolved_width, &overflow);
    let visual_runs = visual_runs(&block, &lines, &line_metrics);
    let glyph_count = visual_runs
        .iter()
        .map(|run| run.glyphs.len())
        .sum::<usize>();

    TextBlockLayoutResult {
        block_id: block.text_block_id,
        input_run_count: block.runs.len(),
        resolved_run_count: resolved_run_count(&source),
        resolved_cluster_count: source.len(),
        line_count: line_metrics.len(),
        glyph_run_count: visual_runs.len(),
        glyph_count,
        measured_size: UiSize::new(
            resolved_width,
            block
                .layout
                .height_constraint
                .resolved_height(content_height),
        ),
        content_bounds: UiRect::new(0.0, 0.0, resolved_width, content_height),
        ink_bounds: UiRect::new(0.0, 0.0, resolved_width, content_height),
        line_metrics,
        visual_runs,
        clusters: source.into_iter().map(|glyph| glyph.cluster).collect(),
        overflow_evidence: overflow,
        fallback_evidence: vec![fallback],
        diagnostics,
    }
}

fn empty_result(block: TextBlock, diagnostics: Vec<TextLayoutDiagnostic>) -> TextBlockLayoutResult {
    TextBlockLayoutResult {
        block_id: block.text_block_id,
        input_run_count: 0,
        resolved_run_count: 0,
        resolved_cluster_count: 0,
        line_count: 0,
        glyph_run_count: 0,
        glyph_count: 0,
        measured_size: UiSize::ZERO,
        content_bounds: UiRect::ZERO,
        ink_bounds: UiRect::ZERO,
        line_metrics: Vec::new(),
        visual_runs: Vec::new(),
        clusters: Vec::new(),
        overflow_evidence: TextOverflowEvidence::none(),
        fallback_evidence: Vec::new(),
        diagnostics,
    }
}

fn lower_source(
    block: &TextBlock,
    atlas_source: &dyn FontAtlasSource,
    fallback: &mut TextFallbackEvidence,
    diagnostics: &mut Vec<TextLayoutDiagnostic>,
) -> Vec<ProofGlyph> {
    let mut source = Vec::new();
    let mut index = 0_u32;
    for run in &block.runs {
        if run.text.is_empty() {
            diagnostics.push(TextLayoutDiagnostic::warning(
                "ui.text.run.empty",
                format!("source run {} is empty", run.run_id.0),
            ));
            continue;
        }
        let run_style = block.base_style.merged_with(&run.style);
        for (local_index, ch) in run.text.chars().enumerate() {
            let span = run.spans.iter().find(|span| {
                let local = local_index as u32;
                local >= span.source_range.start_cluster && local < span.source_range.end_cluster
            });
            let style = span.map_or(run_style.clone(), |span| run_style.merged_with(&span.style));
            let metrics = metrics(atlas_source, style.font_id, style.font_size, ch, fallback);
            source.push(ProofGlyph {
                cluster: TextCluster {
                    cluster_index: index,
                    run_id: run.run_id,
                    span_id: span.map(|span| span.span_id),
                    source_range: TextSourceRange::new(index, index + 1),
                    text_preview: ch.to_string(),
                    style: style.clone(),
                    semantic_role: span
                        .and_then(|span| span.semantic_role)
                        .or(run.semantic_role)
                        .or(block.semantic_role),
                    is_whitespace: ch.is_whitespace() && ch != '\n',
                    is_newline: ch == '\n',
                },
                glyph_key: if metrics.5 {
                    "replacement:?".to_owned()
                } else {
                    format!("char:{ch}")
                },
                advance: metrics.0.advance.max(0.0) * metrics.4
                    + style.letter_spacing.resolve(style.font_size),
                ascent: metrics.1 * metrics.4,
                descent: metrics.2.abs() * metrics.4,
                line_height: style.line_height_or_default(metrics.3 * metrics.4),
                replacement: metrics.5,
            });
            index += 1;
        }
    }
    source
}

fn metrics(
    atlas_source: &dyn FontAtlasSource,
    font_id: FontId,
    font_size: f32,
    ch: char,
    fallback: &mut TextFallbackEvidence,
) -> (GlyphMetrics, f32, f32, f32, f32, bool) {
    let atlas = atlas_source.atlas(font_id);
    let replacement = atlas
        .and_then(|atlas| atlas.glyphs.get(&'?').copied())
        .unwrap_or(GlyphMetrics {
            advance: 8.0,
            plane_left: 0.0,
            plane_top: 8.0,
            plane_right: 8.0,
            plane_bottom: -2.0,
            atlas_left: 0.0,
            atlas_top: 0.0,
            atlas_right: 0.0,
            atlas_bottom: 0.0,
        });
    let (glyph, missing) = atlas
        .and_then(|atlas| atlas.glyphs.get(&ch).copied())
        .map(|glyph| (glyph, false))
        .unwrap_or((replacement, true));
    if missing {
        fallback.missing_glyph_count += 1;
        fallback.fallback_glyph_count += 1;
        fallback.replacement_glyph_count += 1;
    }
    let base_size = atlas.map_or(12.0, |atlas| atlas.metrics.base_size.max(1.0));
    (
        glyph,
        atlas.map_or(9.0, |atlas| atlas.metrics.ascender),
        atlas.map_or(-3.0, |atlas| atlas.metrics.descender),
        atlas.map_or(12.0, |atlas| atlas.metrics.line_height),
        font_size / base_size,
        missing,
    )
}

fn collapse_horizontal_spaces(source: &mut Vec<ProofGlyph>) {
    let mut collapsed = Vec::new();
    let mut last_was_space = false;
    for glyph in source.drain(..) {
        if glyph.cluster.is_whitespace && last_was_space {
            continue;
        }
        last_was_space = glyph.cluster.is_whitespace;
        if !glyph.cluster.is_whitespace {
            last_was_space = false;
        }
        collapsed.push(glyph);
    }
    *source = collapsed;
}

fn break_lines(
    block: &TextBlock,
    source: &[ProofGlyph],
    width_limit: Option<f32>,
    diagnostics: &mut Vec<TextLayoutDiagnostic>,
) -> Vec<ProofLine> {
    let mut lines = Vec::new();
    let mut line = ProofLine::default();
    for glyph in source {
        if glyph.cluster.is_newline {
            line.explicit_break = true;
            lines.push(line);
            line = ProofLine::default();
            continue;
        }
        let wraps = block.layout.wrap != TextWrapPolicy::NoWrap
            && width_limit.is_some_and(|limit| {
                !line.glyphs.is_empty() && line_width(&line.glyphs) + glyph.advance > limit
            });
        if wraps {
            match block.layout.wrap {
                TextWrapPolicy::Word => wrap_word_line(&mut lines, &mut line, diagnostics),
                TextWrapPolicy::Character => wrap_character_line(&mut lines, &mut line),
                TextWrapPolicy::NoWrap => {}
            }
        }
        line.glyphs.push(glyph.clone());
    }
    lines.push(line);
    lines
}

fn wrap_word_line(
    lines: &mut Vec<ProofLine>,
    line: &mut ProofLine,
    diagnostics: &mut Vec<TextLayoutDiagnostic>,
) {
    if let Some(boundary) = line
        .glyphs
        .iter()
        .rposition(|glyph| glyph.cluster.is_whitespace)
    {
        let mut remainder = line.glyphs.split_off(boundary + 1);
        while line
            .glyphs
            .last()
            .is_some_and(|glyph| glyph.cluster.is_whitespace)
        {
            line.glyphs.pop();
        }
        while remainder
            .first()
            .is_some_and(|glyph| glyph.cluster.is_whitespace)
        {
            remainder.remove(0);
        }
        let pushed = core::mem::replace(
            line,
            ProofLine {
                glyphs: remainder,
                wrapped_from_previous: true,
                explicit_break: false,
            },
        );
        lines.push(ProofLine {
            explicit_break: false,
            ..pushed
        });
    } else {
        diagnostics.push(TextLayoutDiagnostic::warning(
            "ui.text.wrap.word_fell_back_to_character",
            "word wrap found no whitespace boundary and fell back to cluster wrapping",
        ));
        wrap_character_line(lines, line);
    }
}

fn wrap_character_line(lines: &mut Vec<ProofLine>, line: &mut ProofLine) {
    let pushed = core::mem::take(line);
    lines.push(ProofLine {
        explicit_break: false,
        ..pushed
    });
    line.wrapped_from_previous = true;
}

fn trim_line_edges(lines: &mut [ProofLine]) {
    for line in lines {
        while line
            .glyphs
            .first()
            .is_some_and(|glyph| glyph.cluster.is_whitespace)
        {
            line.glyphs.remove(0);
        }
        while line
            .glyphs
            .last()
            .is_some_and(|glyph| glyph.cluster.is_whitespace)
        {
            line.glyphs.pop();
        }
    }
}

fn apply_end_ellipsis(line: &mut Vec<ProofGlyph>, limit: f32, atlas_source: &dyn FontAtlasSource) {
    let Some(template) = line.last().cloned() else {
        return;
    };
    let mut fallback = TextFallbackEvidence::none(template.cluster.style.font_id);
    let dot = metrics(
        atlas_source,
        template.cluster.style.font_id,
        template.cluster.style.font_size,
        '.',
        &mut fallback,
    );
    let dot_width = dot.0.advance.max(0.0) * dot.4;
    while !line.is_empty() && line_width(line) + dot_width * 3.0 > limit + f32::EPSILON {
        line.pop();
    }
    let start = line
        .last()
        .map(|glyph| glyph.cluster.cluster_index + 1)
        .unwrap_or(template.cluster.cluster_index);
    for offset in 0..3 {
        let mut next = template.clone();
        next.cluster.cluster_index = start + offset;
        next.cluster.text_preview = ".".to_owned();
        next.cluster.is_whitespace = false;
        next.cluster.is_newline = false;
        next.glyph_key = "ellipsis:.".to_owned();
        next.advance = dot_width;
        line.push(next);
    }
}

fn line_width(line: &[ProofGlyph]) -> f32 {
    line.iter().map(|glyph| glyph.advance).sum()
}

fn line_dimensions(line: &ProofLine) -> (f32, f32, f32) {
    let ascent = line
        .glyphs
        .iter()
        .map(|glyph| glyph.ascent)
        .fold(0.0, f32::max);
    let descent = line
        .glyphs
        .iter()
        .map(|glyph| glyph.descent)
        .fold(0.0, f32::max);
    let height = line
        .glyphs
        .iter()
        .map(|glyph| glyph.line_height)
        .fold((ascent + descent).max(1.0), f32::max);
    (ascent, descent, height)
}

fn line_height(line: &ProofLine) -> f32 {
    line_dimensions(line).2
}

fn resolved_run_count(source: &[ProofGlyph]) -> usize {
    source
        .iter()
        .map(|glyph| glyph.cluster.run_id)
        .collect::<std::collections::BTreeSet<TextRunId>>()
        .len()
}

#[derive(Clone)]
struct PendingVisualRun {
    line_index: u32,
    run_id: TextRunId,
    span_id: Option<crate::TextSpanId>,
    font_id: FontId,
    style: TextStyle,
    direction: TextDirectionPolicy,
    glyphs: Vec<TextGlyph>,
}

impl PendingVisualRun {
    fn new(line_index: u32, glyph: &ProofGlyph, direction: TextDirectionPolicy) -> Self {
        Self {
            line_index,
            run_id: glyph.cluster.run_id,
            span_id: glyph.cluster.span_id,
            font_id: glyph.cluster.style.font_id,
            style: glyph.cluster.style.clone(),
            direction,
            glyphs: Vec::new(),
        }
    }

    fn matches(&self, glyph: &ProofGlyph, direction: TextDirectionPolicy) -> bool {
        self.run_id == glyph.cluster.run_id
            && self.span_id == glyph.cluster.span_id
            && self.font_id == glyph.cluster.style.font_id
            && self.style == glyph.cluster.style
            && self.direction == direction
    }

    fn push(&mut self, glyph: TextGlyph) {
        self.glyphs.push(glyph);
    }

    fn finish(self, visual_run_id: u32) -> TextVisualRun {
        TextVisualRun {
            visual_run_id,
            line_index: self.line_index,
            run_id: self.run_id,
            span_id: self.span_id,
            font_id: self.font_id,
            style: self.style,
            direction: self.direction,
            bounds: glyph_bounds(&self.glyphs),
            glyphs: self.glyphs,
        }
    }
}

fn line_metrics(
    block: &TextBlock,
    lines: &[ProofLine],
    resolved_width: f32,
    overflow: &TextOverflowEvidence,
) -> Vec<TextLineMetrics> {
    let mut y = 0.0;
    let mut metrics = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        let width = line_width(&line.glyphs);
        let (ascent, descent, height) = line_dimensions(line);
        let x = match block.layout.horizontal_align {
            TextHorizontalAlign::Start => 0.0,
            TextHorizontalAlign::Center => ((resolved_width - width) * 0.5).max(0.0),
            TextHorizontalAlign::End => (resolved_width - width).max(0.0),
        };
        let line_box = UiRect::new(x, y, width, height);
        metrics.push(TextLineMetrics {
            line_index: line_index as u32,
            source_range: visible_range_for_line(line),
            visual_order: line_index as u32,
            origin: UiPoint::new(x, y),
            baseline_y: y + ascent,
            ascent,
            descent,
            line_gap: (height - ascent - descent).max(0.0),
            line_height: height,
            line_box,
            content_width: width,
            ink_bounds: line_box,
            horizontal_align: block.layout.horizontal_align,
            is_wrapped: line.wrapped_from_previous,
            is_explicit_break: line.explicit_break,
            is_truncated: line_index + 1 == lines.len()
                && (overflow.clipped || overflow.ellipsized || overflow.max_lines_applied),
        });
        y += height;
    }
    metrics
}

fn visual_runs(
    block: &TextBlock,
    lines: &[ProofLine],
    metrics: &[TextLineMetrics],
) -> Vec<TextVisualRun> {
    let mut runs = Vec::new();
    let mut draw_order = 0_u32;
    for (line_index, line) in lines.iter().enumerate() {
        let Some(line_metrics) = metrics.get(line_index) else {
            continue;
        };
        let mut pen_x = line_metrics.origin.x;
        let mut pending: Option<PendingVisualRun> = None;
        for glyph in &line.glyphs {
            let origin = UiPoint::new(pen_x, line_metrics.baseline_y);
            let bounds = UiRect::new(
                origin.x,
                line_metrics.baseline_y - glyph.ascent,
                glyph.advance,
                line_metrics.line_height,
            );
            let text_glyph = TextGlyph {
                draw_order,
                line_index: line_index as u32,
                run_id: glyph.cluster.run_id,
                span_id: glyph.cluster.span_id,
                font_id: glyph.cluster.style.font_id,
                glyph_key: glyph.glyph_key.clone(),
                cluster_range: TextClusterRange::new(
                    glyph.cluster.cluster_index,
                    glyph.cluster.cluster_index + 1,
                ),
                origin,
                advance: glyph.advance,
                bounds,
                source_text_preview: glyph.cluster.text_preview.clone(),
                replacement: glyph.replacement,
            };
            let direction = block.layout.text_direction;
            if pending
                .as_ref()
                .is_some_and(|segment| !segment.matches(glyph, direction))
            {
                if let Some(segment) = pending.take() {
                    runs.push(segment.finish(runs.len() as u32));
                }
            }
            if pending.is_none() {
                pending = Some(PendingVisualRun::new(line_index as u32, glyph, direction));
            }
            pending
                .as_mut()
                .expect("pending visual run")
                .push(text_glyph);
            draw_order += 1;
            pen_x += glyph.advance;
        }
        if let Some(segment) = pending.take() {
            runs.push(segment.finish(runs.len() as u32));
        }
    }
    runs
}

fn glyph_bounds(glyphs: &[TextGlyph]) -> UiRect {
    let Some(first) = glyphs.first() else {
        return UiRect::ZERO;
    };
    let mut min_x = first.bounds.x;
    let mut min_y = first.bounds.y;
    let mut max_x = first.bounds.x + first.bounds.width;
    let mut max_y = first.bounds.y + first.bounds.height;
    for glyph in glyphs.iter().skip(1) {
        min_x = min_x.min(glyph.bounds.x);
        min_y = min_y.min(glyph.bounds.y);
        max_x = max_x.max(glyph.bounds.x + glyph.bounds.width);
        max_y = max_y.max(glyph.bounds.y + glyph.bounds.height);
    }
    UiRect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn visible_range(lines: &[ProofLine]) -> TextClusterRange {
    let start = lines
        .first()
        .map(visible_range_for_line)
        .map(|range| range.start)
        .unwrap_or(0);
    let end = lines
        .last()
        .map(visible_range_for_line)
        .map(|range| range.end)
        .unwrap_or(start);
    TextClusterRange::new(start, end)
}

fn visible_range_for_line(line: &ProofLine) -> TextClusterRange {
    let start = line
        .glyphs
        .first()
        .map(|glyph| glyph.cluster.cluster_index)
        .unwrap_or(0);
    let end = line
        .glyphs
        .last()
        .map(|glyph| glyph.cluster.cluster_index + 1)
        .unwrap_or(start);
    TextClusterRange::new(start, end)
}

fn omitted_run_count(block: &TextBlock, lines: &[ProofLine]) -> u32 {
    block
        .runs
        .iter()
        .filter(|run| {
            !lines
                .iter()
                .flat_map(|line| line.glyphs.iter())
                .any(|glyph| glyph.cluster.run_id == run.run_id)
        })
        .count() as u32
}
