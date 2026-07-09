//! Overlay, popup, and radial-menu node contracts.

use ui_math::{UiInsets, UiSize};
use ui_theme::ThemeTokens;

use crate::WidgetId;

#[derive(Debug, Clone, PartialEq)]
pub struct RadialMenuNode {
    pub anchor: RadialMenuAnchor,
    pub layer_order: u32,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub item_size: UiSize,
    pub start_angle_radians: f32,
    pub theme: ThemeTokens,
}

impl RadialMenuNode {
    pub fn anchored_to(anchor: WidgetId, theme: ThemeTokens) -> Self {
        Self {
            anchor: RadialMenuAnchor::Widget(anchor),
            layer_order: 2,
            inner_radius: 16.0,
            outer_radius: 72.0,
            item_size: UiSize::new(42.0, 30.0),
            start_angle_radians: -std::f32::consts::FRAC_PI_2,
            theme,
        }
    }

    pub fn anchored_at(point: ui_math::UiPoint, theme: ThemeTokens) -> Self {
        Self {
            anchor: RadialMenuAnchor::Point(point),
            layer_order: 2,
            inner_radius: 16.0,
            outer_radius: 72.0,
            item_size: UiSize::new(42.0, 30.0),
            start_angle_radians: -std::f32::consts::FRAC_PI_2,
            theme,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RadialMenuAnchor {
    Widget(WidgetId),
    Point(ui_math::UiPoint),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayAdornmentNode {
    pub anchor: WidgetId,
    pub placement: PopupPlacement,
    pub offset: f32,
    pub min_size: UiSize,
    pub stretch_child: bool,
}

impl OverlayAdornmentNode {
    pub fn anchored_inside_top_start(anchor: WidgetId, offset: f32) -> Self {
        Self {
            anchor,
            placement: PopupPlacement::TopStart,
            offset,
            min_size: UiSize::ZERO,
            stretch_child: false,
        }
    }

    pub fn anchored_inside_top_end(anchor: WidgetId, offset: f32) -> Self {
        Self {
            anchor,
            placement: PopupPlacement::InsideTopEnd,
            offset,
            min_size: UiSize::ZERO,
            stretch_child: false,
        }
    }

    pub fn anchored_inside_edge(anchor: WidgetId, side: PopupSide, thickness: f32) -> Self {
        let placement = match side {
            PopupSide::Left => PopupPlacement::InsideLeft,
            PopupSide::Right => PopupPlacement::InsideRight,
            PopupSide::Top => PopupPlacement::InsideTop,
            PopupSide::Bottom => PopupPlacement::InsideBottom,
        };
        let min_size = match side {
            PopupSide::Left | PopupSide::Right => UiSize::new(thickness.max(0.0), 0.0),
            PopupSide::Top | PopupSide::Bottom => UiSize::new(0.0, thickness.max(0.0)),
        };
        Self {
            anchor,
            placement,
            offset: 0.0,
            min_size,
            stretch_child: true,
        }
    }

    pub fn anchored_inside_center(anchor: WidgetId) -> Self {
        Self {
            anchor,
            placement: PopupPlacement::InsideCenter,
            offset: 0.0,
            min_size: UiSize::ZERO,
            stretch_child: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopupNode {
    pub anchor: WidgetId,
    pub placement: PopupPlacement,
    pub dismiss_policy: PopupDismissPolicy,
    pub layer_order: u32,
    pub padding: UiInsets,
    pub gap: f32,
    pub offset: f32,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupPlacement {
    BottomStart,
    RightStart,
    InsideTopEnd,
    InsideBottomStart,
    TopStart,
    InsideLeft,
    InsideRight,
    InsideTop,
    InsideBottom,
    InsideCenter,
    Outside {
        preferred_side: PopupSide,
        align: PopupAlign,
        flip_policy: PopupFlipPolicy,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupSide {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupAlign {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupFlipPolicy {
    None,
    FlipToFit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupDismissPolicy {
    None,
    OutsidePointerDown,
}

impl PopupNode {
    pub fn anchored_outside(
        anchor: WidgetId,
        preferred_side: PopupSide,
        align: PopupAlign,
        flip_policy: PopupFlipPolicy,
        theme: ThemeTokens,
    ) -> Self {
        Self {
            anchor,
            placement: PopupPlacement::Outside {
                preferred_side,
                align,
                flip_policy,
            },
            dismiss_policy: PopupDismissPolicy::OutsidePointerDown,
            layer_order: 2,
            padding: UiInsets::all(theme.spacing.xs),
            gap: theme.spacing.xs,
            offset: theme.spacing.xs,
            min_size: UiSize::new(120.0, 0.0),
            theme,
        }
    }

    pub fn anchored_bottom_start(anchor: WidgetId, theme: ThemeTokens) -> Self {
        Self {
            anchor,
            placement: PopupPlacement::BottomStart,
            dismiss_policy: PopupDismissPolicy::OutsidePointerDown,
            layer_order: 2,
            padding: UiInsets::all(theme.spacing.xs),
            gap: theme.spacing.xs,
            offset: theme.spacing.xs,
            min_size: UiSize::new(120.0, 0.0),
            theme,
        }
    }

    pub fn anchored_top_start(anchor: WidgetId, theme: ThemeTokens) -> Self {
        Self {
            anchor,
            placement: PopupPlacement::TopStart,
            dismiss_policy: PopupDismissPolicy::OutsidePointerDown,
            layer_order: 1,
            padding: UiInsets::all(theme.spacing.xs),
            gap: theme.spacing.xs,
            offset: 0.0,
            min_size: UiSize::new(120.0, 0.0),
            theme,
        }
    }

    pub fn anchored_right_start(anchor: WidgetId, theme: ThemeTokens) -> Self {
        Self {
            anchor,
            placement: PopupPlacement::RightStart,
            dismiss_policy: PopupDismissPolicy::OutsidePointerDown,
            layer_order: 2,
            padding: UiInsets::all(theme.spacing.xs),
            gap: theme.spacing.xs,
            offset: theme.spacing.xs,
            min_size: UiSize::new(120.0, 0.0),
            theme,
        }
    }

    pub fn anchored_inside_top_end(anchor: WidgetId, theme: ThemeTokens) -> Self {
        Self {
            anchor,
            placement: PopupPlacement::InsideTopEnd,
            dismiss_policy: PopupDismissPolicy::OutsidePointerDown,
            layer_order: 0,
            padding: UiInsets::all(0.0),
            gap: 0.0,
            offset: 0.0,
            min_size: UiSize::ZERO,
            theme,
        }
    }

    pub fn anchored_inside_bottom_start(anchor: WidgetId, theme: ThemeTokens) -> Self {
        Self {
            anchor,
            placement: PopupPlacement::InsideBottomStart,
            dismiss_policy: PopupDismissPolicy::OutsidePointerDown,
            layer_order: 1,
            padding: UiInsets::all(theme.spacing.xs),
            gap: theme.spacing.xs,
            offset: theme.spacing.xs,
            min_size: UiSize::new(120.0, 0.0),
            theme,
        }
    }

    pub fn with_dismiss_policy(mut self, dismiss_policy: PopupDismissPolicy) -> Self {
        self.dismiss_policy = dismiss_policy;
        self
    }
}
