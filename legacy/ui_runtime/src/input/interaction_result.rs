//! File: domain/ui/ui_runtime/src/input/interaction_result.rs
//! Purpose: Semantic UI interaction outcomes produced by runtime input handling.

use crate::WidgetId;
use ui_input::{FocusChange, KeyboardEvent, TextInputEvent};
use ui_math::Axis;

#[derive(Debug, Clone, PartialEq)]
pub enum UiInteraction {
    Activated(WidgetId),
    HoveredChanged {
        previous: Option<WidgetId>,
        current: Option<WidgetId>,
    },
    PressedChanged {
        previous: Option<WidgetId>,
        current: Option<WidgetId>,
    },
    FocusChanged(FocusChange),
    PopupDismissRequested {
        popup: WidgetId,
        focus_return: Option<WidgetId>,
    },
    ScrollInputOwned {
        owner: WidgetId,
        axis: Axis,
        changed: bool,
        at_boundary: bool,
    },
    KeyboardInput {
        target: WidgetId,
        event: KeyboardEvent,
    },
    GraphCanvasAction {
        target: WidgetId,
        action: ui_graph_editor::GraphCanvasAction,
    },
    TextInput {
        target: WidgetId,
        event: TextInputEvent,
    },
    Toggled {
        target: WidgetId,
        checked: bool,
    },
    NumericStepped {
        target: WidgetId,
        value: f64,
    },
    TabSelected {
        target: WidgetId,
        index: usize,
    },
    SelectChanged {
        target: WidgetId,
        index: usize,
    },
    TableRowSelected {
        target: WidgetId,
        row_index: usize,
    },
    TreeRowSelected {
        target: WidgetId,
        row_index: usize,
    },
    TreeRowToggled {
        target: WidgetId,
        row_index: usize,
        expanded: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiInteractionResults {
    pub items: Vec<UiInteraction>,
}

impl UiInteractionResults {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, interaction: UiInteraction) {
        self.items.push(interaction);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
