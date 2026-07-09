//! File: domain/ui/ui_runtime/src/output/interaction_visual.rs
//! Purpose: Runtime interaction state used by frame emission.

use std::collections::BTreeMap;

use crate::{ScrollbarAxisOpacities, ScrollbarAxisTarget, WidgetId};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct InteractionVisualState {
    pub hovered_widget: Option<WidgetId>,
    pub pressed_widget: Option<WidgetId>,
    pub focused_widget: Option<WidgetId>,
    pub hovered_scrollbar: Option<ScrollbarAxisTarget>,
    pub active_scrollbar: Option<ScrollbarAxisTarget>,
    pub scrollbar_opacity_by_widget_id: BTreeMap<WidgetId, ScrollbarAxisOpacities>,
    pub graph_canvas_gestures: BTreeMap<WidgetId, ui_graph_editor::GraphCanvasGestureState>,
    pub graph_canvas_viewports: BTreeMap<WidgetId, ui_graph_editor::GraphViewport>,
}
