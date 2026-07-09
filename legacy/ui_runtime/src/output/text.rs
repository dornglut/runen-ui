//! File: domain/ui/ui_runtime/src/output/text.rs
//! Purpose: Runtime text output helpers for UI frame emission.

use ui_text::{
    TextDirectionPolicy, TextEllipsisPlacement, TextHorizontalAlign, TextLayoutPolicy,
    TextOverflowPolicy, TextVerticalAlign, TextWhitespacePolicy, TextWrapPolicy,
};

pub(crate) fn single_line_text_layout(
    horizontal_align: TextHorizontalAlign,
    vertical_align: TextVerticalAlign,
    overflow: TextOverflowPolicy,
) -> TextLayoutPolicy {
    TextLayoutPolicy {
        wrap: TextWrapPolicy::NoWrap,
        whitespace: TextWhitespacePolicy::Preserve,
        horizontal_align,
        vertical_align,
        overflow,
        max_lines: Some(1),
        text_direction: TextDirectionPolicy::Ltr,
        ..TextLayoutPolicy::default()
    }
}

pub(crate) fn clipped_text_layout(horizontal_align: TextHorizontalAlign) -> TextLayoutPolicy {
    single_line_text_layout(
        horizontal_align,
        TextVerticalAlign::Center,
        TextOverflowPolicy::Clip,
    )
}

pub(crate) fn ellipsis_text_layout(horizontal_align: TextHorizontalAlign) -> TextLayoutPolicy {
    single_line_text_layout(
        horizontal_align,
        TextVerticalAlign::Center,
        TextOverflowPolicy::Ellipsis(TextEllipsisPlacement::End),
    )
}
