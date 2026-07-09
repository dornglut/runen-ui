//! File: domain/ui/ui_render_data/src/lib.rs
//! Crate: ui_render_data

pub mod batching;
pub mod colors;
pub mod frame;
pub mod primitives;

pub use batching::*;
pub use colors::*;
pub use frame::*;
pub use primitives::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_surface_binding_registry_is_slot_scoped() {
        let mut registry = ViewportSurfaceBindingRegistry::default();
        let primary_slot = ViewportSurfaceEmbedSlotId::new(1);
        let overlay_slot = ViewportSurfaceEmbedSlotId::new(3);
        registry.bind(
            7,
            primary_slot,
            ViewportSurfaceBinding::dynamic_texture("viewport", "target.primary"),
        );
        registry.bind(
            7,
            overlay_slot,
            ViewportSurfaceBinding::dynamic_texture("viewport", "target.overlay"),
        );

        let primary_binding = registry
            .get(7, primary_slot)
            .expect("primary slot binding should exist");
        let ViewportSurfaceBindingSource::DynamicTexture {
            namespace,
            target_id,
        } = &primary_binding.source;
        assert_eq!(namespace, "viewport");
        assert_eq!(target_id, "target.primary");

        let overlay_binding = registry
            .get(7, overlay_slot)
            .expect("overlay slot binding should exist");
        let ViewportSurfaceBindingSource::DynamicTexture {
            namespace,
            target_id,
        } = &overlay_binding.source;
        assert_eq!(namespace, "viewport");
        assert_eq!(target_id, "target.overlay");
    }

    #[test]
    fn stroke_primitive_constructs_and_exports_as_ui_primitive() {
        let primitive = StrokePrimitive::new(
            [
                ui_math::UiPoint::new(1.0, 2.0),
                ui_math::UiPoint::new(5.0, 7.0),
            ],
            3.0,
            UiPaint::rgba(0.1, 0.2, 0.3, 1.0),
            UiDrawKey::new(9, None),
            UiSortKey::new(0, 2, 4),
        )
        .with_clip(ui_math::UiRect::new(0.0, 0.0, 10.0, 10.0));

        let UiPrimitive::Stroke(stroke) = UiPrimitive::from(primitive) else {
            panic!("stroke primitive should convert into UiPrimitive::Stroke");
        };
        assert_eq!(stroke.points.len(), 2);
        assert_eq!(stroke.width, 3.0);
        assert_eq!(stroke.draw_key.material_id, 9);
        assert!(stroke.clip.is_some());
    }

    #[test]
    fn graph_canvas_emits_primitives() {
        let mut batch = GraphCanvasPrimitiveBatch::new();
        let draw_key = UiDrawKey::new(0, None);
        let paint = UiPaint::rgba(0.2, 0.3, 0.4, 1.0);
        batch.push(
            GraphCanvasPrimitiveRole::NodeBox,
            RectPrimitive::new(
                ui_math::UiRect::new(8.0, 8.0, 96.0, 48.0),
                4.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 0),
            ),
        );
        batch.push(
            GraphCanvasPrimitiveRole::Port,
            RectPrimitive::new(
                ui_math::UiRect::new(4.0, 20.0, 8.0, 8.0),
                4.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 1),
            ),
        );
        batch.push(
            GraphCanvasPrimitiveRole::Edge,
            StrokePrimitive::new(
                [
                    ui_math::UiPoint::new(20.0, 20.0),
                    ui_math::UiPoint::new(80.0, 30.0),
                ],
                2.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 2),
            ),
        );
        batch.push(
            GraphCanvasPrimitiveRole::Label,
            RectPrimitive::new(
                ui_math::UiRect::new(12.0, 12.0, 24.0, 8.0),
                0.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 3),
            ),
        );
        batch.push(
            GraphCanvasPrimitiveRole::SelectionOutline,
            BorderPrimitive::new(
                ui_math::UiRect::new(6.0, 6.0, 100.0, 52.0),
                4.0,
                1.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 4),
            ),
        );
        batch.push(
            GraphCanvasPrimitiveRole::ConnectionPreview,
            StrokePrimitive::new(
                [
                    ui_math::UiPoint::new(20.0, 20.0),
                    ui_math::UiPoint::new(120.0, 60.0),
                ],
                1.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 5),
            ),
        );
        batch.push(
            GraphCanvasPrimitiveRole::Overlay,
            BorderPrimitive::new(
                ui_math::UiRect::new(96.0, 4.0, 20.0, 20.0),
                10.0,
                1.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 6),
            ),
        );

        for role in [
            GraphCanvasPrimitiveRole::NodeBox,
            GraphCanvasPrimitiveRole::Port,
            GraphCanvasPrimitiveRole::Edge,
            GraphCanvasPrimitiveRole::Label,
            GraphCanvasPrimitiveRole::SelectionOutline,
            GraphCanvasPrimitiveRole::ConnectionPreview,
            GraphCanvasPrimitiveRole::Overlay,
        ] {
            assert_eq!(batch.count_role(role), 1);
        }
        assert_eq!(batch.into_ui_primitives().len(), 7);
    }

    #[test]
    fn graph_canvas_orders_primitives_by_role() {
        let mut batch = GraphCanvasPrimitiveBatch::new();
        let draw_key = UiDrawKey::new(0, None);
        let paint = UiPaint::rgba(0.2, 0.3, 0.4, 1.0);
        let rect = |order| {
            RectPrimitive::new(
                ui_math::UiRect::new(0.0, 0.0, 1.0, 1.0),
                0.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, order),
            )
        };

        batch.push(GraphCanvasPrimitiveRole::Label, rect(0));
        batch.push(GraphCanvasPrimitiveRole::Overlay, rect(1));
        batch.push(GraphCanvasPrimitiveRole::NodeBox, rect(2));
        batch.push(GraphCanvasPrimitiveRole::Edge, rect(3));
        batch.push(GraphCanvasPrimitiveRole::SelectionOutline, rect(4));
        batch.push(GraphCanvasPrimitiveRole::ConnectionPreview, rect(5));
        batch.push(GraphCanvasPrimitiveRole::Port, rect(6));

        let roles = batch
            .into_render_primitives()
            .into_iter()
            .map(|primitive| primitive.role)
            .collect::<Vec<_>>();

        assert_eq!(
            roles,
            vec![
                GraphCanvasPrimitiveRole::Edge,
                GraphCanvasPrimitiveRole::ConnectionPreview,
                GraphCanvasPrimitiveRole::NodeBox,
                GraphCanvasPrimitiveRole::Port,
                GraphCanvasPrimitiveRole::SelectionOutline,
                GraphCanvasPrimitiveRole::Overlay,
                GraphCanvasPrimitiveRole::Label,
            ]
        );
    }
}
