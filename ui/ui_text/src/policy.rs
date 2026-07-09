//! File: domain/ui/ui_text/src/policy.rs
//! Purpose: Text layout policy contracts separated from appearance style.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextWidthConstraint {
    Unconstrained,
    Exact(f32),
    Max(f32),
}

impl TextWidthConstraint {
    pub fn limit(self) -> Option<f32> {
        match self {
            Self::Unconstrained => None,
            Self::Exact(value) | Self::Max(value) => Some(value.max(0.0)),
        }
    }

    pub fn resolved_width(self, content_width: f32) -> f32 {
        match self {
            Self::Unconstrained => content_width,
            Self::Exact(value) => value.max(0.0),
            Self::Max(value) => content_width.min(value.max(0.0)),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unconstrained => "unconstrained",
            Self::Exact(_) => "exact",
            Self::Max(_) => "max",
        }
    }
}
impl Default for TextWidthConstraint {
    fn default() -> Self {
        Self::Unconstrained
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextHeightConstraint {
    Unconstrained,
    Exact(f32),
    Max(f32),
}
impl TextHeightConstraint {
    pub fn limit(self) -> Option<f32> {
        match self {
            Self::Unconstrained => None,
            Self::Exact(value) | Self::Max(value) => Some(value.max(0.0)),
        }
    }
    pub fn resolved_height(self, content_height: f32) -> f32 {
        match self {
            Self::Unconstrained => content_height,
            Self::Exact(value) => value.max(0.0),
            Self::Max(value) => content_height.min(value.max(0.0)),
        }
    }
}
impl Default for TextHeightConstraint {
    fn default() -> Self {
        Self::Unconstrained
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextWrapPolicy {
    NoWrap,
    Word,
    Character,
}
impl TextWrapPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoWrap => "no-wrap",
            Self::Word => "word",
            Self::Character => "character",
        }
    }
}
impl Default for TextWrapPolicy {
    fn default() -> Self {
        Self::NoWrap
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextWhitespacePolicy {
    Preserve,
    CollapseRuns,
    TrimEdges,
}
impl TextWhitespacePolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preserve => "preserve",
            Self::CollapseRuns => "collapse-runs",
            Self::TrimEdges => "trim-edges",
        }
    }
}
impl Default for TextWhitespacePolicy {
    fn default() -> Self {
        Self::Preserve
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextHorizontalAlign {
    Start,
    Center,
    End,
}
impl TextHorizontalAlign {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        }
    }
}
impl Default for TextHorizontalAlign {
    fn default() -> Self {
        Self::Start
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextVerticalAlign {
    Start,
    Center,
    End,
    Baseline,
}
impl TextVerticalAlign {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
            Self::Baseline => "baseline",
        }
    }
}
impl Default for TextVerticalAlign {
    fn default() -> Self {
        Self::Start
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextOverflowPolicy {
    Clip,
    Ellipsis(TextEllipsisPlacement),
}
impl TextOverflowPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clip => "clip",
            Self::Ellipsis(_) => "ellipsis",
        }
    }
}
impl Default for TextOverflowPolicy {
    fn default() -> Self {
        Self::Clip
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextEllipsisPlacement {
    Start,
    Middle,
    End,
}
impl TextEllipsisPlacement {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Middle => "middle",
            Self::End => "end",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextDirectionPolicy {
    Auto,
    Ltr,
    Rtl,
}
impl TextDirectionPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}
impl Default for TextDirectionPolicy {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextLayoutPolicy {
    pub width_constraint: TextWidthConstraint,
    pub height_constraint: TextHeightConstraint,
    pub wrap: TextWrapPolicy,
    pub whitespace: TextWhitespacePolicy,
    pub horizontal_align: TextHorizontalAlign,
    pub vertical_align: TextVerticalAlign,
    pub overflow: TextOverflowPolicy,
    pub max_lines: Option<u32>,
    pub text_direction: TextDirectionPolicy,
}

impl TextLayoutPolicy {
    pub const fn single_line(
        horizontal_align: TextHorizontalAlign,
        vertical_align: TextVerticalAlign,
        overflow: TextOverflowPolicy,
    ) -> Self {
        Self {
            width_constraint: TextWidthConstraint::Unconstrained,
            height_constraint: TextHeightConstraint::Unconstrained,
            wrap: TextWrapPolicy::NoWrap,
            whitespace: TextWhitespacePolicy::Preserve,
            horizontal_align,
            vertical_align,
            overflow,
            max_lines: Some(1),
            text_direction: TextDirectionPolicy::Auto,
        }
    }

    pub const fn single_line_start() -> Self {
        Self::single_line(
            TextHorizontalAlign::Start,
            TextVerticalAlign::Start,
            TextOverflowPolicy::Clip,
        )
    }

    pub const fn single_line_centered() -> Self {
        Self::single_line(
            TextHorizontalAlign::Center,
            TextVerticalAlign::Center,
            TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::End),
        )
    }

    pub const fn single_line_end() -> Self {
        Self::single_line(
            TextHorizontalAlign::End,
            TextVerticalAlign::Center,
            TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::End),
        )
    }

    pub const fn no_wrap_label() -> Self {
        Self {
            max_lines: None,
            text_direction: TextDirectionPolicy::Auto,
            ..Self::single_line_start()
        }
    }

    pub const fn button_label() -> Self {
        Self::single_line_centered()
    }

    pub const fn wrapping_body(max_width: f32) -> Self {
        Self {
            width_constraint: TextWidthConstraint::Max(max_width),
            height_constraint: TextHeightConstraint::Unconstrained,
            wrap: TextWrapPolicy::Word,
            whitespace: TextWhitespacePolicy::CollapseRuns,
            horizontal_align: TextHorizontalAlign::Start,
            vertical_align: TextVerticalAlign::Start,
            overflow: TextOverflowPolicy::Clip,
            max_lines: None,
            text_direction: TextDirectionPolicy::Auto,
        }
    }

    pub const fn helper_text(max_width: f32, max_lines: u32) -> Self {
        Self {
            width_constraint: TextWidthConstraint::Max(max_width),
            height_constraint: TextHeightConstraint::Unconstrained,
            wrap: TextWrapPolicy::Word,
            whitespace: TextWhitespacePolicy::CollapseRuns,
            horizontal_align: TextHorizontalAlign::Start,
            vertical_align: TextVerticalAlign::Start,
            overflow: TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::End),
            max_lines: Some(max_lines),
            text_direction: TextDirectionPolicy::Auto,
        }
    }

    pub const fn badge(max_width: f32) -> Self {
        Self {
            width_constraint: TextWidthConstraint::Max(max_width),
            whitespace: TextWhitespacePolicy::TrimEdges,
            ..Self::single_line_centered()
        }
    }

    pub const fn tab_label(max_width: f32) -> Self {
        Self {
            width_constraint: TextWidthConstraint::Max(max_width),
            whitespace: TextWhitespacePolicy::TrimEdges,
            ..Self::single_line_centered()
        }
    }

    pub const fn inspector_row_value(max_width: f32) -> Self {
        Self {
            width_constraint: TextWidthConstraint::Max(max_width),
            height_constraint: TextHeightConstraint::Unconstrained,
            wrap: TextWrapPolicy::Word,
            whitespace: TextWhitespacePolicy::CollapseRuns,
            horizontal_align: TextHorizontalAlign::Start,
            vertical_align: TextVerticalAlign::Baseline,
            overflow: TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::End),
            max_lines: Some(1),
            text_direction: TextDirectionPolicy::Auto,
        }
    }
}

impl Default for TextLayoutPolicy {
    fn default() -> Self {
        Self::no_wrap_label()
    }
}
