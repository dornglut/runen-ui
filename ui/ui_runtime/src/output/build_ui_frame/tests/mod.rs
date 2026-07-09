//! File: domain/ui/ui_runtime/src/output/build_ui_frame/tests.rs
//! Purpose: Build UI frame behavior tests.

use super::super::test_support::{
    TestAtlasSource, atlas_with_ascii, first_border_paint, first_glyph, first_rect_paint,
    first_text_glyph, frame_signature, has_rect_primitive, primitive_sort_key, rect_approx_eq,
};
use super::*;
use crate::{
    ButtonNode, LabelNode, PanelNode, PopupNode, ScrollbarAxisTarget, UiNode, UiNodeKind,
    UiRuntimeState, UiTree, ViewportSurfaceEmbedNode, WidgetId, compute_tree_layout,
};
use ui_math::{Axis, UiRect, UiSize};
use ui_render_data::{ClipPrimitive, UiDrawKey, UiPaint, UiPrimitive, ViewportSurfaceEmbedSlotId};
use ui_text::{FontId, TextEllipsisPlacement, TextLineHeightPolicy, TextOverflowPolicy, TextStyle};
use ui_theme::ThemeTokens;

mod layering;
mod scrollbars;
mod snapshot;
mod visual_states;
