//! File: domain/ui/ui_widgets/src/lib.rs
//! Crate: ui_widgets
//! Purpose: First widget constructors and helpers over ui_tree node contracts.

pub use ::ui_tree::*;

mod button;
mod color_picker;
mod divider;
mod image;
mod label;
mod numeric_input;
mod panel;
mod product_surface;
mod scenario;
mod scroll;
mod search_field;
mod select;
mod spacer;
mod split;
mod stack;
mod table;
mod tabs;
mod text_input;
mod toggle;
mod tree_widget;
mod vector_input;
mod viewport_surface_embed;

pub use button::{button, button_selected};
pub use color_picker::{
    ColorPickerControl, ColorPickerControlPackage, ColorPickerDeferredFeature,
    ColorPickerDiagnostic, ColorPickerEventPacket, ColorPickerEventPayload, ColorPickerProperties,
    ColorPickerSchema, ColorPickerState, ColorPickerTrianglePoint, color_picker,
    color_picker_committed_packet, color_picker_control_package, color_picker_diagnostics,
    color_picker_fixture, color_picker_preview_changed_packet, color_picker_rgb_cube_status,
};
pub use divider::{divider, hdivider, vdivider};
pub use image::image;
pub use label::label;
pub use numeric_input::{NumericInputConfig, numeric_input};
pub use panel::panel;
pub use product_surface::product_surface;
pub use scenario::{
    PrimitiveWidgetScenario, PrimitiveWidgetScenarioKind, primitive_widget_scenarios,
};
pub use scroll::{hscroll, scroll, vscroll, xy_scroll};
pub use search_field::search_field;
pub use select::select;
pub use spacer::spacer;
pub use split::split;
pub use stack::{hstack, hstack_with_policies, stack, vstack, vstack_with_policies};
pub use table::table;
pub use tabs::tabs;
pub use text_input::text_input;
pub use toggle::toggle;
pub use tree_widget::tree;
pub use vector_input::vector3_input;
pub use viewport_surface_embed::viewport_surface_embed;

#[cfg(test)]
mod tests {
    use super::*;
    use ui_text::TextStyle;
    use ui_theme::ThemeTokens;

    #[test]
    fn constructors_emit_expected_node_kinds() {
        let style = TextStyle::default();
        let theme = ThemeTokens::default();
        assert!(matches!(
            button(WidgetId(1), "Apply", style.clone(), theme.clone()).kind,
            UiNodeKind::Button(_)
        ));
        assert!(matches!(
            text_input(
                WidgetId(2),
                "value",
                "placeholder",
                style.clone(),
                theme.clone()
            )
            .kind,
            UiNodeKind::TextInput(_)
        ));
        assert!(matches!(
            toggle(WidgetId(3), "Snap", true, style.clone(), theme.clone()).kind,
            UiNodeKind::Toggle(_)
        ));
        assert!(matches!(
            numeric_input(
                WidgetId(4),
                NumericInputConfig::new(1.0, 0.1, Some(0.0), Some(2.0), 2),
                style.clone(),
                theme.clone()
            )
            .kind,
            UiNodeKind::NumericInput(_)
        ));
        assert!(matches!(
            tabs(WidgetId(5), ["A", "B"], 0, style, theme.clone()).kind,
            UiNodeKind::Tabs(_)
        ));
        assert!(matches!(
            select(
                WidgetId(50),
                ["A", "B"],
                Some(1),
                "Choose",
                TextStyle::default(),
                theme.clone(),
            )
            .kind,
            UiNodeKind::Select(_)
        ));
        assert!(matches!(
            table(
                WidgetId(51),
                [TableColumn::new("Name", 80.0)],
                [TableRow::new(["Player"])],
                TextStyle::default(),
                TextStyle::default(),
                theme.clone(),
            )
            .kind,
            UiNodeKind::Table(_)
        ));
        assert!(matches!(
            tree(
                WidgetId(52),
                [TreeRow::new("Root", 0, true)],
                TextStyle::default(),
                theme.clone(),
            )
            .kind,
            UiNodeKind::Tree(_)
        ));
        assert!(matches!(
            search_field(WidgetId(53), "player", TextStyle::default(), theme.clone(),).kind,
            UiNodeKind::TextInput(_)
        ));
        assert!(matches!(
            hscroll(WidgetId(6), ThemeTokens::default(), Vec::new()).kind,
            UiNodeKind::Scroll(_)
        ));
        assert!(matches!(
            vscroll(WidgetId(7), ThemeTokens::default(), Vec::new()).kind,
            UiNodeKind::Scroll(_)
        ));
        assert!(matches!(
            spacer(WidgetId(8), ui_math::UiSize::new(12.0, 4.0)).kind,
            UiNodeKind::Spacer(_)
        ));
        assert!(matches!(
            hdivider(WidgetId(9), 1.0, ui_layout::SizePolicy::Auto, theme.border).kind,
            UiNodeKind::Divider(_)
        ));
        assert!(matches!(
            image(
                WidgetId(10),
                ui_render_data::UiDrawKey::new(2, Some(3)),
                ui_math::UiRect::new(0.0, 0.0, 1.0, 1.0),
                ui_render_data::UiPaint::WHITE,
                ui_math::UiSize::new(16.0, 16.0),
            )
            .kind,
            UiNodeKind::Image(_)
        ));
        assert!(matches!(
            product_surface(
                WidgetId(11),
                ui_render_data::ProductSurfaceTextureBindingSource::dynamic_texture(
                    "test", "target"
                ),
                ui_math::UiSize::new(64.0, 64.0),
            )
            .kind,
            UiNodeKind::ProductSurface(_)
        ));
    }
}
