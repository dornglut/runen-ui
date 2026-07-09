//! File: domain/ui/ui_render_data/src/primitives/clip.rs
//! Purpose: Clip stack primitives.

use ui_math::UiRect;

use crate::UiSortKey;

#[derive(Debug, Clone, PartialEq)]
pub enum ClipPrimitive {
    Push { rect: UiRect, sort_key: UiSortKey },
    Pop { sort_key: UiSortKey },
}
