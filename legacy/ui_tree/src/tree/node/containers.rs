//! Container and scrolling node contracts.

use ui_layout::SizePolicy;
use ui_math::{Axis, UiInsets, UiSize};
use ui_theme::ThemeTokens;

#[derive(Debug, Clone, PartialEq)]
pub struct PanelNode {
    pub padding: UiInsets,
    pub gap: f32,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
}

impl PanelNode {
    pub fn new(theme: ThemeTokens) -> Self {
        Self {
            padding: UiInsets::all(theme.spacing.xs),
            gap: theme.spacing.xs,
            min_size: UiSize::ZERO,
            theme,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScrollNode {
    pub axes: ScrollAxes,
    pub bar_thickness: f32,
    pub min_thumb_main_size: f32,
    pub input_policies: ScrollInputPolicies,
    pub theme: ThemeTokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxes {
    Horizontal,
    Vertical,
    Both,
}

impl ScrollAxes {
    pub const fn from_axis(axis: Axis) -> Self {
        match axis {
            Axis::Horizontal => Self::Horizontal,
            Axis::Vertical => Self::Vertical,
        }
    }

    pub const fn contains(self, axis: Axis) -> bool {
        matches!(
            (self, axis),
            (Self::Both, _)
                | (Self::Horizontal, Axis::Horizontal)
                | (Self::Vertical, Axis::Vertical)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollInputPolicy {
    WheelOnly,
    MiddleDragOnly,
    WheelAndMiddleDrag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollInputPolicies {
    pub horizontal: ScrollInputPolicy,
    pub vertical: ScrollInputPolicy,
}

impl ScrollInputPolicies {
    pub const fn new(horizontal: ScrollInputPolicy, vertical: ScrollInputPolicy) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    pub const fn for_axis(self, axis: Axis) -> ScrollInputPolicy {
        match axis {
            Axis::Horizontal => self.horizontal,
            Axis::Vertical => self.vertical,
        }
    }
}

impl Default for ScrollInputPolicies {
    fn default() -> Self {
        Self {
            horizontal: ScrollInputPolicy::WheelAndMiddleDrag,
            vertical: ScrollInputPolicy::WheelOnly,
        }
    }
}

impl ScrollNode {
    pub fn new(theme: ThemeTokens) -> Self {
        Self::vertical(theme)
    }

    pub fn vertical(theme: ThemeTokens) -> Self {
        Self {
            axes: ScrollAxes::Vertical,
            bar_thickness: (theme.spacing.xs * 1.5).clamp(6.0, 18.0),
            min_thumb_main_size: (theme.spacing.lg + theme.spacing.xs).max(18.0),
            input_policies: ScrollInputPolicies::default(),
            theme,
        }
    }

    pub fn horizontal(theme: ThemeTokens) -> Self {
        Self {
            axes: ScrollAxes::Horizontal,
            bar_thickness: (theme.spacing.xs * 1.5).clamp(6.0, 18.0),
            min_thumb_main_size: (theme.spacing.lg + theme.spacing.xs).max(18.0),
            input_policies: ScrollInputPolicies::default(),
            theme,
        }
    }

    pub fn both(theme: ThemeTokens) -> Self {
        Self {
            axes: ScrollAxes::Both,
            bar_thickness: (theme.spacing.xs * 1.5).clamp(6.0, 18.0),
            min_thumb_main_size: (theme.spacing.lg + theme.spacing.xs).max(18.0),
            input_policies: ScrollInputPolicies::default(),
            theme,
        }
    }

    pub fn with_input_policies(mut self, input_policies: ScrollInputPolicies) -> Self {
        self.input_policies = input_policies;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StackNode {
    pub axis: Axis,
    pub gap: f32,
    pub padding: UiInsets,
    pub child_main_policies: Vec<SizePolicy>,
}

impl StackNode {
    pub fn vertical(gap: f32) -> Self {
        Self {
            axis: Axis::Vertical,
            gap,
            padding: UiInsets::ZERO,
            child_main_policies: Vec::new(),
        }
    }

    pub fn horizontal(gap: f32) -> Self {
        Self {
            axis: Axis::Horizontal,
            gap,
            padding: UiInsets::ZERO,
            child_main_policies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SplitNode {
    pub axis: Axis,
    pub ratio: f32,
    pub gap: f32,
}

impl SplitNode {
    pub fn new(axis: Axis, ratio: f32, gap: f32) -> Self {
        Self { axis, ratio, gap }
    }
}
