use std::collections::HashMap;

use ui_math::UiRect;
use ui_render_data::{ClipPrimitive, GlyphRunPrimitive, UiFrame, UiPaint, UiPrimitive, UiSortKey};
use ui_text::{FontAtlasSource, FontFaceMetrics, FontId, GlyphMetrics, MsdfFontAtlas};

#[derive(Debug, Clone)]
pub(crate) struct TestAtlasSource {
    pub(crate) atlas: MsdfFontAtlas,
}

impl FontAtlasSource for TestAtlasSource {
    fn atlas(&self, font_id: FontId) -> Option<&MsdfFontAtlas> {
        (self.atlas.font_id == font_id).then_some(&self.atlas)
    }
}

pub(crate) fn frame_signature(frame: &UiFrame) -> String {
    let layer = &frame.surfaces[0].layers[0];
    layer
        .primitives
        .iter()
        .map(|primitive| match primitive {
            UiPrimitive::Rect(value) => format!(
                "Rect(x={:.1} y={:.1} w={:.1} h={:.1})",
                value.rect.x, value.rect.y, value.rect.width, value.rect.height
            ),
            UiPrimitive::Border(value) => format!(
                "Border(x={:.1} y={:.1} w={:.1} h={:.1})",
                value.rect.x, value.rect.y, value.rect.width, value.rect.height
            ),
            UiPrimitive::Clip(ClipPrimitive::Push { rect, .. }) => format!(
                "ClipPush(x={:.1} y={:.1} w={:.1} h={:.1})",
                rect.x, rect.y, rect.width, rect.height
            ),
            UiPrimitive::Clip(ClipPrimitive::Pop { .. }) => "ClipPop".to_string(),
            UiPrimitive::GlyphRun(value) => {
                let text = crate::text_emission::text_from_primitive(value);
                format!(
                    "GlyphRun(text=\"{}\" clip={})",
                    text,
                    value.baseline_origin_clip.is_some()
                )
            }
            UiPrimitive::ViewportSurfaceEmbed(value) => format!(
                "ViewportSurfaceEmbed(viewport={} slot={:?})",
                value.viewport_id, value.slot
            ),
            UiPrimitive::Image(value) => format!(
                "Image(x={:.1} y={:.1} w={:.1} h={:.1})",
                value.rect.x, value.rect.y, value.rect.width, value.rect.height
            ),
            UiPrimitive::ProductSurface(value) => format!(
                "ProductSurface(x={:.1} y={:.1} w={:.1} h={:.1})",
                value.rect.x, value.rect.y, value.rect.width, value.rect.height
            ),
            UiPrimitive::Stroke(value) => {
                format!(
                    "Stroke(points={} width={:.1})",
                    value.points.len(),
                    value.width
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn primitive_sort_key(primitive: &UiPrimitive) -> UiSortKey {
    match primitive {
        UiPrimitive::Rect(value) => value.sort_key,
        UiPrimitive::Border(value) => value.sort_key,
        UiPrimitive::GlyphRun(value) => value.sort_key,
        UiPrimitive::Image(value) => value.sort_key,
        UiPrimitive::Stroke(value) => value.sort_key,
        UiPrimitive::ViewportSurfaceEmbed(value) => value.sort_key,
        UiPrimitive::ProductSurface(value) => value.sort_key,
        UiPrimitive::Clip(ClipPrimitive::Push { sort_key, .. }) => *sort_key,
        UiPrimitive::Clip(ClipPrimitive::Pop { sort_key }) => *sort_key,
    }
}

pub(crate) fn has_rect_primitive(frame: &UiFrame, expected_rect: UiRect) -> bool {
    frame.surfaces.iter().any(|surface| {
        surface.layers.iter().any(|layer| {
            layer.primitives.iter().any(|primitive| match primitive {
                UiPrimitive::Rect(value) => rect_approx_eq(value.rect, expected_rect),
                _ => false,
            })
        })
    })
}

pub(crate) fn first_rect_paint(frame: &UiFrame) -> Option<UiPaint> {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .find_map(|primitive| match primitive {
            UiPrimitive::Rect(value) => Some(value.paint),
            _ => None,
        })
}

pub(crate) fn first_border_paint(frame: &UiFrame) -> Option<UiPaint> {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .find_map(|primitive| match primitive {
            UiPrimitive::Border(value) => Some(value.paint),
            _ => None,
        })
}

pub(crate) fn first_glyph(frame: &UiFrame) -> Option<&ui_text::TextGlyph> {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .find_map(|primitive| match primitive {
            UiPrimitive::GlyphRun(run) => first_text_glyph(run),
            _ => None,
        })
}

pub(crate) fn first_text_glyph(run: &GlyphRunPrimitive) -> Option<&ui_text::TextGlyph> {
    run.visual_runs
        .iter()
        .flat_map(|visual_run| visual_run.glyphs.iter())
        .next()
}

pub(crate) fn rect_approx_eq(left: UiRect, right: UiRect) -> bool {
    (left.x - right.x).abs() <= 0.001
        && (left.y - right.y).abs() <= 0.001
        && (left.width - right.width).abs() <= 0.001
        && (left.height - right.height).abs() <= 0.001
}

pub(crate) fn atlas_with_ascii(font_id: FontId) -> MsdfFontAtlas {
    let mut glyphs = HashMap::new();
    for ch in 32_u8..=126_u8 {
        glyphs.insert(
            char::from(ch),
            GlyphMetrics {
                advance: 10.0,
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
    MsdfFontAtlas {
        font_id,
        texture_width: 256,
        texture_height: 256,
        metrics: FontFaceMetrics {
            ascender: 9.0,
            descender: -3.0,
            line_height: 12.0,
            base_size: 12.0,
        },
        glyphs,
    }
}
