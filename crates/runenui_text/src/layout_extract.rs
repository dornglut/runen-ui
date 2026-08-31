use std::{collections::HashMap, sync::Arc};

use parley::{Layout, PositionedLayoutItem};
use runenui_core::{LogicalSize, ResourceKind, ResourceRef};

use crate::{
    FontSourceRevision, ShapedTextResource, TextArtifact, TextCluster, TextFontBinding, TextGlyph,
    TextLine, TextLineMetrics, TextRun,
};

pub(crate) fn extract_layout(
    layout: &Layout<()>,
    source_revision: FontSourceRevision,
    resources: &mut HashMap<ResourceRef, Arc<ShapedTextResource>>,
) -> Option<TextArtifact> {
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
                Arc::<[u8]>::from(font.data.as_ref()),
                font.index,
                run.normalized_coords().to_vec(),
                synthesis.embolden(),
                synthesis.skew(),
            )?;

            let glyphs = glyph_run
                .glyphs()
                .map(|glyph| TextGlyph::new(glyph.id, glyph.x, glyph.y, glyph.advance))
                .collect::<Option<Vec<_>>>()?;

            let resource_ref = ResourceRef::new(ResourceKind::ShapedTextRun);
            let shaped = Arc::new(ShapedTextResource::new(
                resource_ref.clone(),
                font,
                run.font_size(),
                glyphs,
            )?);
            resources.insert(resource_ref, Arc::clone(&shaped));

            let clusters = run
                .clusters()
                .map(|cluster| {
                    TextCluster::new(
                        cluster.text_range(),
                        cluster.advance(),
                        cluster.is_rtl(),
                        cluster.is_ligature_start(),
                        cluster.is_ligature_continuation(),
                        cluster.is_word_boundary(),
                        cluster.is_soft_line_break(),
                        cluster.is_hard_line_break(),
                        cluster.is_space_or_nbsp(),
                        cluster.is_emoji(),
                    )
                })
                .collect::<Option<Vec<_>>>()?;

            runs.push(TextRun::new(
                run.text_range(),
                glyph_run.offset(),
                glyph_run.baseline(),
                glyph_run.advance(),
                run.is_rtl(),
                clusters,
                shaped,
            )?);
        }

        lines.push(TextLine::new(line.text_range(), metrics, runs));
    }

    Some(TextArtifact::new(size, source_revision, lines))
}
