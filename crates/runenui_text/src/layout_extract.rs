use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use parley::{Brush, Layout, PositionedLayoutItem};
use runenui_core::{LogicalSize, ResourceKind, ResourceRef};

use crate::{
    FontSourceSnapshot, ShapedTextResource, TextArtifact, TextCluster, TextClusterFlag,
    TextClusterFlags, TextDirection, TextFontBinding, TextGlyph, TextLine, TextLineMetrics,
    TextRun,
};

pub(super) fn extract_layout<B: Brush>(
    layout: &Layout<B>,
    source_snapshot: FontSourceSnapshot,
    resources: &mut HashMap<ResourceRef, Weak<ShapedTextResource>>,
) -> Option<TextArtifact> {
    resources.retain(|_, resource| resource.strong_count() > 0);

    let size = LogicalSize::try_new(layout.width(), layout.height()).ok()?;
    let mut lines = Vec::with_capacity(layout.lines().count());

    for line in layout.lines() {
        let metrics = line.metrics();
        let metrics = TextLineMetrics::from_finite([
            metrics.line_height,
            metrics.baseline,
            metrics.offset,
            metrics.advance,
            metrics.trailing_whitespace,
            metrics.inline_min_coord,
            metrics.inline_max_coord,
            metrics.block_min_coord,
            metrics.block_max_coord,
        ])?;
        let mut runs = Vec::new();

        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                return None;
            };
            let run = glyph_run.run();
            let synthesis = run.synthesis();
            let font = run.font();
            let font = TextFontBinding::new(
                font.data.clone(),
                font.index,
                run.normalized_coords().to_vec(),
                synthesis.embolden(),
                synthesis.skew(),
            )?;

            let origin_x = glyph_run.offset();
            let origin_y = glyph_run.baseline();
            let glyphs = glyph_run
                .positioned_glyphs()
                .map(|glyph| {
                    TextGlyph::new(
                        glyph.id,
                        glyph.x - origin_x,
                        glyph.y - origin_y,
                        glyph.advance,
                    )
                })
                .collect::<Option<Vec<_>>>()?;

            let resource_ref = ResourceRef::new(ResourceKind::ShapedTextRun);
            let shaped = Arc::new(ShapedTextResource::new(
                resource_ref.clone(),
                font,
                run.font_size(),
                glyphs,
            )?);
            resources.insert(resource_ref, Arc::downgrade(&shaped));

            let clusters = run
                .clusters()
                .map(|cluster| {
                    let flags = TextClusterFlags::NONE
                        .with(TextClusterFlag::LigatureStart, cluster.is_ligature_start())
                        .with(
                            TextClusterFlag::LigatureContinuation,
                            cluster.is_ligature_continuation(),
                        )
                        .with(TextClusterFlag::WordBoundary, cluster.is_word_boundary())
                        .with(TextClusterFlag::SoftLineBreak, cluster.is_soft_line_break())
                        .with(TextClusterFlag::HardLineBreak, cluster.is_hard_line_break())
                        .with(TextClusterFlag::SpaceOrNbsp, cluster.is_space_or_nbsp())
                        .with(TextClusterFlag::Emoji, cluster.is_emoji());
                    TextCluster::new(
                        cluster.text_range(),
                        cluster.advance(),
                        TextDirection::from_rtl(cluster.is_rtl()),
                        flags,
                    )
                })
                .collect::<Option<Vec<_>>>()?;

            runs.push(TextRun::new(
                run.text_range(),
                origin_x,
                origin_y,
                glyph_run.advance(),
                TextDirection::from_rtl(run.is_rtl()),
                clusters,
                shaped,
            )?);
        }

        lines.push(TextLine::new(line.text_range(), metrics, runs));
    }

    Some(TextArtifact::new(size, source_snapshot, lines))
}
