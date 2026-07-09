//! File: domain/ui/ui_render_data/src/frame/composition.rs
//! Purpose: Backend-neutral composition helpers for UI frame fragments.

use ui_math::{UiPoint, UiRect, UiSize};

use crate::{
    BorderPrimitive, ClipPrimitive, GlyphRunPrimitive, ImagePrimitive, ProductSurfacePrimitive,
    RectPrimitive, StrokePrimitive, UiFrame, UiLayer, UiLayerId, UiPrimitive, UiSortKey, UiSurface,
    UiSurfaceId, ViewportSurfaceEmbedPrimitive,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiFramePlacement {
    pub origin: UiPoint,
    pub clip: UiRect,
    pub surface_order: u32,
}

impl UiFramePlacement {
    pub const fn new(origin: UiPoint, clip: UiRect, surface_order: u32) -> Self {
        Self {
            origin,
            clip,
            surface_order,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiFrameFragment<'a> {
    pub frame: &'a UiFrame,
    pub placement: UiFramePlacement,
}

impl<'a> UiFrameFragment<'a> {
    pub const fn new(frame: &'a UiFrame, placement: UiFramePlacement) -> Self {
        Self { frame, placement }
    }
}

pub fn compose_frame_fragments<'a>(
    output_size: UiSize,
    fragments: impl IntoIterator<Item = UiFrameFragment<'a>>,
) -> UiFrame {
    let mut output_layers = Vec::new();
    let mut output_layer_index = 0_u64;

    for fragment in fragments {
        for surface in &fragment.frame.surfaces {
            for layer in &surface.layers {
                if layer.primitives.is_empty() {
                    continue;
                }
                let mut primitives = Vec::with_capacity(layer.primitives.len() + 2);
                primitives.push(UiPrimitive::Clip(ClipPrimitive::Push {
                    rect: fragment.placement.clip,
                    sort_key: UiSortKey::new(fragment.placement.surface_order, 0, 0),
                }));
                primitives.extend(layer.primitives.iter().cloned().map(|primitive| {
                    translate_primitive(
                        primitive,
                        fragment.placement.origin,
                        fragment.placement.surface_order,
                    )
                }));
                primitives.push(UiPrimitive::Clip(ClipPrimitive::Pop {
                    sort_key: UiSortKey::new(fragment.placement.surface_order, u32::MAX, u32::MAX),
                }));
                output_layers.push(UiLayer::with_primitives(
                    UiLayerId(output_layer_index),
                    primitives,
                ));
                output_layer_index = output_layer_index.saturating_add(1);
            }
        }
    }

    UiFrame::with_surfaces(vec![UiSurface::with_layers(
        UiSurfaceId(0),
        output_size,
        output_layers,
    )])
}

fn translate_primitive(primitive: UiPrimitive, origin: UiPoint, surface_order: u32) -> UiPrimitive {
    match primitive {
        UiPrimitive::Rect(value) => UiPrimitive::Rect(RectPrimitive {
            rect: translate_rect(value.rect, origin),
            sort_key: sort_key_with_surface(value.sort_key, surface_order),
            ..value
        }),
        UiPrimitive::Border(value) => UiPrimitive::Border(BorderPrimitive {
            rect: translate_rect(value.rect, origin),
            sort_key: sort_key_with_surface(value.sort_key, surface_order),
            ..value
        }),
        UiPrimitive::GlyphRun(value) => {
            let mut visual_runs = value.visual_runs.clone();
            for visual_run in &mut visual_runs {
                visual_run.bounds = translate_rect(visual_run.bounds, origin);
                for glyph in &mut visual_run.glyphs {
                    glyph.origin = translate_point(glyph.origin, origin);
                    glyph.bounds = translate_rect(glyph.bounds, origin);
                }
            }
            UiPrimitive::GlyphRun(GlyphRunPrimitive {
                visual_runs,
                baseline_origin_clip: value
                    .baseline_origin_clip
                    .map(|clip| translate_rect(clip, origin)),
                sort_key: sort_key_with_surface(value.sort_key, surface_order),
                ..value
            })
        }
        UiPrimitive::Image(value) => UiPrimitive::Image(ImagePrimitive {
            rect: translate_rect(value.rect, origin),
            sort_key: sort_key_with_surface(value.sort_key, surface_order),
            ..value
        }),
        UiPrimitive::Stroke(value) => UiPrimitive::Stroke(StrokePrimitive {
            points: value
                .points
                .into_iter()
                .map(|point| translate_point(point, origin))
                .collect(),
            clip: value.clip.map(|clip| translate_rect(clip, origin)),
            sort_key: sort_key_with_surface(value.sort_key, surface_order),
            ..value
        }),
        UiPrimitive::ViewportSurfaceEmbed(value) => {
            UiPrimitive::ViewportSurfaceEmbed(ViewportSurfaceEmbedPrimitive {
                rect: translate_rect(value.rect, origin),
                sort_key: sort_key_with_surface(value.sort_key, surface_order),
                ..value
            })
        }
        UiPrimitive::ProductSurface(value) => {
            UiPrimitive::ProductSurface(ProductSurfacePrimitive {
                rect: translate_rect(value.rect, origin),
                sort_key: sort_key_with_surface(value.sort_key, surface_order),
                ..value
            })
        }
        UiPrimitive::Clip(ClipPrimitive::Push { rect, sort_key }) => {
            UiPrimitive::Clip(ClipPrimitive::Push {
                rect: translate_rect(rect, origin),
                sort_key: sort_key_with_surface(sort_key, surface_order),
            })
        }
        UiPrimitive::Clip(ClipPrimitive::Pop { sort_key }) => {
            UiPrimitive::Clip(ClipPrimitive::Pop {
                sort_key: sort_key_with_surface(sort_key, surface_order),
            })
        }
    }
}

fn translate_point(point: UiPoint, origin: UiPoint) -> UiPoint {
    UiPoint::new(point.x + origin.x, point.y + origin.y)
}

fn translate_rect(rect: UiRect, origin: UiPoint) -> UiRect {
    UiRect::new(
        rect.x + origin.x,
        rect.y + origin.y,
        rect.width,
        rect.height,
    )
}

fn sort_key_with_surface(sort_key: UiSortKey, surface_order: u32) -> UiSortKey {
    UiSortKey::new(
        surface_order,
        sort_key.layer_order,
        sort_key.primitive_order,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ProductSurfaceAlphaMode, ProductSurfaceTextureBindingSource, UiDrawKey, UiPaint,
        ViewportSurfaceEmbedSlotId,
    };
    use ui_text::{
        FontId, TextBlockId, TextClusterRange, TextDirectionPolicy, TextGlyph,
        TextOverflowEvidence, TextRunId, TextStyle, TextVisualRun,
    };

    #[test]
    fn composition_translates_all_current_primitive_shapes_and_rewrites_surface_order() {
        let paint = UiPaint::rgba(0.1, 0.2, 0.3, 1.0);
        let draw_key = UiDrawKey::new(1, Some(7));
        let source = UiFrame::with_surfaces(vec![UiSurface::with_layers(
            UiSurfaceId(99),
            UiSize::new(40.0, 20.0),
            vec![UiLayer::with_primitives(
                UiLayerId(3),
                vec![
                    UiPrimitive::Rect(RectPrimitive::new(
                        UiRect::new(1.0, 2.0, 3.0, 4.0),
                        2.0,
                        paint,
                        draw_key,
                        UiSortKey::new(0, 5, 1),
                    )),
                    UiPrimitive::Border(BorderPrimitive::new(
                        UiRect::new(2.0, 3.0, 4.0, 5.0),
                        3.0,
                        2.0,
                        paint,
                        draw_key,
                        UiSortKey::new(0, 5, 2),
                    )),
                    UiPrimitive::GlyphRun(GlyphRunPrimitive::new(
                        test_text_layout(),
                        Some(UiRect::new(3.0, 4.0, 20.0, 10.0)),
                        paint,
                        draw_key,
                        UiSortKey::new(0, 5, 3),
                    )),
                    UiPrimitive::Image(ImagePrimitive::new(
                        UiRect::new(4.0, 5.0, 6.0, 7.0),
                        UiRect::new(0.0, 0.0, 1.0, 1.0),
                        paint,
                        draw_key,
                        UiSortKey::new(0, 5, 4),
                    )),
                    UiPrimitive::Stroke(
                        StrokePrimitive::new(
                            [UiPoint::new(5.0, 6.0), UiPoint::new(7.0, 8.0)],
                            2.0,
                            paint,
                            draw_key,
                            UiSortKey::new(0, 5, 5),
                        )
                        .with_clip(UiRect::new(0.0, 0.0, 12.0, 12.0)),
                    ),
                    UiPrimitive::ViewportSurfaceEmbed(ViewportSurfaceEmbedPrimitive::new(
                        10,
                        ViewportSurfaceEmbedSlotId::new(2),
                        UiRect::new(8.0, 9.0, 10.0, 11.0),
                        UiRect::new(0.0, 0.0, 1.0, 1.0),
                        paint,
                        UiSortKey::new(0, 5, 6),
                    )),
                    UiPrimitive::ProductSurface(ProductSurfacePrimitive::new(
                        ProductSurfaceTextureBindingSource::dynamic_texture("ns", "target"),
                        UiRect::new(9.0, 10.0, 11.0, 12.0),
                        UiRect::new(0.0, 0.0, 1.0, 1.0),
                        paint,
                        ProductSurfaceAlphaMode::Straight,
                        UiSortKey::new(0, 5, 7),
                    )),
                    UiPrimitive::Clip(ClipPrimitive::Push {
                        rect: UiRect::new(0.0, 0.0, 9.0, 9.0),
                        sort_key: UiSortKey::new(0, 5, 8),
                    }),
                    UiPrimitive::Clip(ClipPrimitive::Pop {
                        sort_key: UiSortKey::new(0, 5, 9),
                    }),
                ],
            )],
        )]);

        let composed = compose_frame_fragments(
            UiSize::new(200.0, 100.0),
            [UiFrameFragment::new(
                &source,
                UiFramePlacement::new(
                    UiPoint::new(20.0, 30.0),
                    UiRect::new(16.0, 24.0, 80.0, 40.0),
                    4,
                ),
            )],
        );

        assert_eq!(composed.surfaces.len(), 1);
        assert_eq!(composed.surfaces[0].id, UiSurfaceId(0));
        assert_eq!(composed.surfaces[0].size, UiSize::new(200.0, 100.0));
        let primitives = &composed.surfaces[0].layers[0].primitives;
        assert_eq!(primitives.len(), 11);
        assert_eq!(
            primitives.first(),
            Some(&UiPrimitive::Clip(ClipPrimitive::Push {
                rect: UiRect::new(16.0, 24.0, 80.0, 40.0),
                sort_key: UiSortKey::new(4, 0, 0),
            }))
        );

        let UiPrimitive::Rect(rect) = &primitives[1] else {
            panic!("expected translated rect");
        };
        assert_eq!(rect.rect, UiRect::new(21.0, 32.0, 3.0, 4.0));
        assert_eq!(rect.sort_key, UiSortKey::new(4, 5, 1));

        let UiPrimitive::Border(border) = &primitives[2] else {
            panic!("expected translated border");
        };
        assert_eq!(border.rect, UiRect::new(22.0, 33.0, 4.0, 5.0));
        assert_eq!(border.width, 2.0);
        assert_eq!(border.sort_key, UiSortKey::new(4, 5, 2));

        let UiPrimitive::GlyphRun(run) = &primitives[3] else {
            panic!("expected translated glyph run");
        };
        assert_eq!(
            run.visual_runs[0].glyphs[0].origin,
            UiPoint::new(24.0, 35.0)
        );
        assert_eq!(
            run.baseline_origin_clip,
            Some(UiRect::new(23.0, 34.0, 20.0, 10.0))
        );
        assert_eq!(run.sort_key, UiSortKey::new(4, 5, 3));

        let UiPrimitive::Stroke(stroke) = &primitives[5] else {
            panic!("expected translated stroke");
        };
        assert_eq!(stroke.points[0], UiPoint::new(25.0, 36.0));
        assert_eq!(stroke.points[1], UiPoint::new(27.0, 38.0));
        assert_eq!(stroke.clip, Some(UiRect::new(20.0, 30.0, 12.0, 12.0)));
        assert_eq!(stroke.sort_key, UiSortKey::new(4, 5, 5));

        let UiPrimitive::ViewportSurfaceEmbed(embed) = &primitives[6] else {
            panic!("expected translated viewport embed");
        };
        assert_eq!(embed.rect, UiRect::new(28.0, 39.0, 10.0, 11.0));
        assert_eq!(embed.sort_key, UiSortKey::new(4, 5, 6));

        let UiPrimitive::ProductSurface(product) = &primitives[7] else {
            panic!("expected translated product surface");
        };
        assert_eq!(product.rect, UiRect::new(29.0, 40.0, 11.0, 12.0));
        assert_eq!(product.sort_key, UiSortKey::new(4, 5, 7));

        assert_eq!(
            primitives.last(),
            Some(&UiPrimitive::Clip(ClipPrimitive::Pop {
                sort_key: UiSortKey::new(4, u32::MAX, u32::MAX),
            }))
        );
    }

    #[test]
    fn composition_skips_empty_source_layers() {
        let source = UiFrame::with_surfaces(vec![UiSurface::with_layers(
            UiSurfaceId(1),
            UiSize::new(10.0, 10.0),
            vec![UiLayer::new(UiLayerId(0))],
        )]);

        let composed = compose_frame_fragments(
            UiSize::new(20.0, 20.0),
            [UiFrameFragment::new(
                &source,
                UiFramePlacement::new(UiPoint::ZERO, UiRect::new(0.0, 0.0, 20.0, 20.0), 0),
            )],
        );

        assert_eq!(composed.surfaces.len(), 1);
        assert!(composed.surfaces[0].layers.is_empty());
        assert!(composed.is_empty());
    }

    fn test_text_layout() -> ui_text::TextBlockLayoutResult {
        ui_text::TextBlockLayoutResult {
            block_id: TextBlockId(7),
            input_run_count: 1,
            resolved_run_count: 1,
            resolved_cluster_count: 1,
            line_count: 1,
            glyph_run_count: 1,
            glyph_count: 1,
            measured_size: UiSize::new(7.0, 8.0),
            content_bounds: UiRect::new(0.0, 0.0, 7.0, 8.0),
            ink_bounds: UiRect::new(0.0, 0.0, 7.0, 8.0),
            line_metrics: Vec::new(),
            visual_runs: vec![TextVisualRun {
                visual_run_id: 0,
                line_index: 0,
                run_id: TextRunId(1),
                span_id: None,
                font_id: FontId(7),
                style: TextStyle {
                    font_id: FontId(7),
                    font_size: 12.0,
                    ..TextStyle::default()
                },
                direction: TextDirectionPolicy::Ltr,
                glyphs: vec![TextGlyph {
                    draw_order: 0,
                    line_index: 0,
                    run_id: TextRunId(1),
                    span_id: None,
                    font_id: FontId(7),
                    glyph_key: "char:A".to_owned(),
                    cluster_range: TextClusterRange::new(0, 1),
                    origin: UiPoint::new(4.0, 5.0),
                    advance: 6.0,
                    bounds: UiRect::new(4.0, -7.0, 6.0, 12.0),
                    source_text_preview: "A".to_owned(),
                    replacement: false,
                }],
                bounds: UiRect::new(0.0, 0.0, 7.0, 8.0),
            }],
            clusters: Vec::new(),
            overflow_evidence: TextOverflowEvidence::none(),
            fallback_evidence: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}
