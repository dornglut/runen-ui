//! File: domain/ui/ui_runtime/src/output/primitives.rs
//! Purpose: Shared primitive paint, draw-key, and sort-key helpers for UI frame emission.

use ui_render_data::{UiDrawKey, UiPaint, UiSortKey};
use ui_theme::UiColor;

pub(crate) fn brighten(color: UiColor, factor: f32) -> UiColor {
    UiColor::new(
        (color.r * factor).clamp(0.0, 1.0),
        (color.g * factor).clamp(0.0, 1.0),
        (color.b * factor).clamp(0.0, 1.0),
        color.a,
    )
}

pub(crate) fn darken(color: UiColor, factor: f32) -> UiColor {
    UiColor::new(
        (color.r * factor).clamp(0.0, 1.0),
        (color.g * factor).clamp(0.0, 1.0),
        (color.b * factor).clamp(0.0, 1.0),
        color.a,
    )
}

pub(crate) fn with_alpha(color: UiColor, alpha_mul: f32) -> UiColor {
    UiColor::new(
        color.r,
        color.g,
        color.b,
        (color.a * alpha_mul).clamp(0.0, 1.0),
    )
}

pub(crate) fn paint_from_color(color: UiColor) -> UiPaint {
    UiPaint::rgba(color.r, color.g, color.b, color.a)
}

pub(crate) fn default_draw_key() -> UiDrawKey {
    UiDrawKey::new(0, None)
}

pub(crate) fn sort_key(layer_order: u32, primitive_order: u32) -> UiSortKey {
    UiSortKey::new(0, layer_order, primitive_order)
}
