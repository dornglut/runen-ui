//! Lightweight substrate gallery harness.
//! Run with: `cargo run -p ui_runtime --example substrate_gallery`

use std::collections::HashMap;

use ui_math::UiRect;
use ui_render_data::{UiPrimitive, ViewportSurfaceEmbedSlotId};
use ui_runtime::{
    ButtonNode, InteractionVisualState, PanelNode, ScrollNode, StackNode, TextInputNode, UiNode,
    UiNodeKind, UiRuntimeState, UiTree, WidgetId, build_ui_frame, compute_tree_layout,
};
use ui_text::{FontAtlasSource, FontFaceMetrics, FontId, GlyphMetrics, MsdfFontAtlas};
use ui_theme::ThemeTokens;

fn main() {
    let atlas = GalleryAtlasSource {
        atlas: atlas_with_ascii(FontId(1)),
    };
    let theme = ThemeTokens::default();
    let style = theme.body_text_style(FontId(1));

    let scenarios = vec![
        (
            "panel_controls",
            UiTree::new(UiNode::with_children(
                WidgetId(1),
                UiNodeKind::Panel(PanelNode::new(theme.clone())),
                vec![UiNode::with_children(
                    WidgetId(2),
                    UiNodeKind::Stack(StackNode::vertical(theme.spacing.xs)),
                    vec![
                        UiNode::new(
                            WidgetId(3),
                            UiNodeKind::Button(ButtonNode::new(
                                "Apply",
                                style.clone(),
                                theme.clone(),
                            )),
                        ),
                        UiNode::new(
                            WidgetId(4),
                            UiNodeKind::TextInput(TextInputNode::new(
                                "",
                                "Name",
                                style.clone(),
                                theme.clone(),
                            )),
                        ),
                    ],
                )],
            )),
        ),
        (
            "scroll_column",
            UiTree::new(UiNode::with_children(
                WidgetId(10),
                UiNodeKind::Scroll(ScrollNode::new(theme.clone())),
                vec![UiNode::with_children(
                    WidgetId(11),
                    UiNodeKind::Stack(StackNode::vertical(theme.spacing.xs)),
                    (0..8)
                        .map(|index| {
                            UiNode::new(
                                WidgetId(12 + index),
                                UiNodeKind::Button(ButtonNode::new(
                                    format!("Row {index}"),
                                    style.clone(),
                                    theme.clone(),
                                )),
                            )
                        })
                        .collect(),
                )],
            )),
        ),
        (
            "viewport_embed_lifecycle",
            UiTree::new(UiNode::with_children(
                WidgetId(20),
                UiNodeKind::Panel(PanelNode::new(theme.clone())),
                vec![UiNode::with_children(
                    WidgetId(21),
                    UiNodeKind::Stack(StackNode::vertical(theme.spacing.xs)),
                    vec![
                        UiNode::new(
                            WidgetId(22),
                            UiNodeKind::ViewportSurfaceEmbed(
                                ui_runtime::ViewportSurfaceEmbedNode::new(
                                    1,
                                    ViewportSurfaceEmbedSlotId::new(1),
                                ),
                            ),
                        ),
                        UiNode::new(
                            WidgetId(23),
                            UiNodeKind::ViewportSurfaceEmbed(
                                ui_runtime::ViewportSurfaceEmbedNode::new(
                                    1,
                                    ViewportSurfaceEmbedSlotId::new(2),
                                ),
                            ),
                        ),
                        UiNode::new(
                            WidgetId(24),
                            UiNodeKind::ViewportSurfaceEmbed(
                                ui_runtime::ViewportSurfaceEmbedNode::new(
                                    1,
                                    ViewportSurfaceEmbedSlotId::new(3),
                                ),
                            ),
                        ),
                    ],
                )],
            )),
        ),
        (
            "multi_surface_embeds",
            UiTree::new(UiNode::with_children(
                WidgetId(30),
                UiNodeKind::Stack(StackNode::horizontal(theme.spacing.sm)),
                vec![
                    UiNode::new(
                        WidgetId(31),
                        UiNodeKind::ViewportSurfaceEmbed(
                            ui_runtime::ViewportSurfaceEmbedNode::new(
                                1,
                                ViewportSurfaceEmbedSlotId::new(1),
                            ),
                        ),
                    ),
                    UiNode::new(
                        WidgetId(32),
                        UiNodeKind::ViewportSurfaceEmbed(
                            ui_runtime::ViewportSurfaceEmbedNode::new(
                                2,
                                ViewportSurfaceEmbedSlotId::new(1),
                            ),
                        ),
                    ),
                ],
            )),
        ),
    ];

    for (name, tree) in scenarios {
        let bounds = UiRect::new(0.0, 0.0, 360.0, 220.0);
        let layouts = compute_tree_layout(&tree, bounds, &UiRuntimeState::default());
        let frame = build_ui_frame(
            &tree,
            &layouts,
            bounds.size(),
            InteractionVisualState::default(),
            &atlas,
        );
        let viewport_slots = viewport_embed_slot_ids(&frame);
        println!(
            "scenario={name} surfaces={} primitives={} viewport_embed_slots={:?}",
            frame.surfaces.len(),
            primitive_count(&frame),
            viewport_slots
        );
    }
}

fn primitive_count(frame: &ui_render_data::UiFrame) -> usize {
    frame
        .surfaces
        .iter()
        .map(|surface| {
            surface
                .layers
                .iter()
                .map(|layer| layer.primitives.len())
                .sum::<usize>()
        })
        .sum()
}

fn viewport_embed_slot_ids(frame: &ui_render_data::UiFrame) -> Vec<u16> {
    frame
        .surfaces
        .iter()
        .flat_map(|surface| surface.layers.iter())
        .flat_map(|layer| layer.primitives.iter())
        .filter_map(|primitive| match primitive {
            UiPrimitive::ViewportSurfaceEmbed(embed) => Some(embed.slot.raw()),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone)]
struct GalleryAtlasSource {
    atlas: MsdfFontAtlas,
}

impl FontAtlasSource for GalleryAtlasSource {
    fn atlas(&self, font_id: FontId) -> Option<&MsdfFontAtlas> {
        (self.atlas.font_id == font_id).then_some(&self.atlas)
    }
}

fn atlas_with_ascii(font_id: FontId) -> MsdfFontAtlas {
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
