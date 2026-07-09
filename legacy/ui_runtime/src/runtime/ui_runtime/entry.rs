//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/entry.rs
//! Purpose: Public retained UI runtime entrypoint.

use std::collections::BTreeSet;
use ui_input::{
    EventPropagation, FocusChange, FocusTargetId, InputResponse, Key, KeyState, PointerCapture,
    UiInputEvent,
};
use ui_math::{Axis, UiRect, UiSize};
use ui_render_data::UiFrame;
use ui_text::FontAtlasSource;

use crate::{
    ComputedLayoutMap, UiInputOutcome, UiInteraction, UiInteractionResults, UiNodeKind,
    UiRuntimeState, UiTree, WidgetId, build_ui_frame, compute_tree_layout, dispatch_pointer_event,
};

use super::{
    focus::{focusable_widgets, focused_widget_captures_viewport_shortcuts, next_focus_target},
    graph_canvas::{graph_canvas_shortcut_action, is_graph_canvas_widget},
    helpers::outcome,
    popup::topmost_popup_scope,
};

#[derive(Debug, Default)]
pub struct UiRuntime {
    state: UiRuntimeState,
}

impl UiRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &UiRuntimeState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut UiRuntimeState {
        &mut self.state
    }

    pub fn set_focused_widget(&mut self, widget_id: Option<WidgetId>) {
        self.state.focused_target = widget_id.map(|value| FocusTargetId(value.0));
    }

    pub fn focused_widget_captures_viewport_shortcuts(&self, tree: &UiTree) -> bool {
        let Some(widget_id) = self.focused_widget_in_tree(tree) else {
            return false;
        };
        focused_widget_captures_viewport_shortcuts(tree, widget_id)
    }

    pub fn scroll_offset(&self, widget_id: WidgetId) -> f32 {
        self.state.scroll_offset(widget_id)
    }

    pub fn set_scroll_offset(&mut self, widget_id: WidgetId, offset: f32) {
        self.state.set_scroll_offset(widget_id, offset);
    }

    pub fn scroll_offset_for_axis(&self, widget_id: WidgetId, axis: Axis) -> f32 {
        self.state.scroll_offset_for_axis(widget_id, axis)
    }

    pub fn set_scroll_offset_for_axis(&mut self, widget_id: WidgetId, axis: Axis, offset: f32) {
        self.state
            .set_scroll_offset_for_axis(widget_id, axis, offset);
    }

    pub fn begin_frame(&mut self) {
        self.state.hovered_widget = None;
    }

    pub fn retain_state_for_tree(&mut self, tree: &UiTree) {
        let mounted_widgets = tree.walk().map(|node| node.id).collect::<BTreeSet<_>>();
        let mounted_graph_canvases = tree
            .walk()
            .filter(|node| matches!(node.kind, UiNodeKind::GraphCanvas(_)))
            .map(|node| node.id)
            .collect::<BTreeSet<_>>();

        self.state
            .graph_canvas_gestures
            .retain(|widget_id, _| mounted_graph_canvases.contains(widget_id));
        self.state
            .graph_canvas_viewports
            .retain(|widget_id, _| mounted_graph_canvases.contains(widget_id));

        self.state.hovered_widget = self
            .state
            .hovered_widget
            .filter(|widget_id| mounted_widgets.contains(widget_id));
        self.state.pressed_widget = self
            .state
            .pressed_widget
            .filter(|widget_id| mounted_widgets.contains(widget_id));
        self.state.captured_widget = self
            .state
            .captured_widget
            .filter(|widget_id| mounted_widgets.contains(widget_id));
        self.state.focused_target = self
            .state
            .focused_target
            .filter(|target| mounted_widgets.contains(&WidgetId(target.0)));
    }

    pub fn compute_layout(&self, tree: &UiTree, bounds: UiRect) -> ComputedLayoutMap {
        compute_tree_layout(tree, bounds, &self.state)
    }

    pub fn dispatch_input(
        &mut self,
        tree: &UiTree,
        layouts: &ComputedLayoutMap,
        event: &UiInputEvent,
    ) -> UiInputOutcome {
        self.retain_state_for_tree(tree);
        match event {
            UiInputEvent::Pointer(pointer) => {
                dispatch_pointer_event(tree, layouts, &mut self.state, pointer)
            }
            UiInputEvent::Keyboard(keyboard) => {
                self.dispatch_keyboard_event(tree, layouts, keyboard)
            }
            // Semantic actions are resolved by the owning interaction domain because
            // retained widgets cannot infer move, resize, or composition policy.
            UiInputEvent::Semantic(_) => UiInputOutcome::ignored(),
            UiInputEvent::Text(text) => self.dispatch_text_event(tree, text),
        }
    }

    pub fn build_frame(
        &self,
        tree: &UiTree,
        bounds: UiRect,
        atlas_source: &dyn FontAtlasSource,
    ) -> UiFrame {
        let layouts = self.compute_layout(tree, bounds);
        let interaction_state = crate::InteractionVisualState {
            hovered_widget: self.state.hovered_widget,
            pressed_widget: self.state.pressed_widget,
            focused_widget: self.state.focused_target.map(|value| WidgetId(value.0)),
            hovered_scrollbar: self.state.hovered_scrollbar,
            active_scrollbar: self
                .state
                .scrollbar_thumb_drag
                .map(|drag| crate::ScrollbarAxisTarget::new(drag.scroll_widget, drag.axis)),
            scrollbar_opacity_by_widget_id: self.state.scrollbar_opacity_entries(),
            graph_canvas_gestures: self.state.graph_canvas_gestures.clone(),
            graph_canvas_viewports: self.state.graph_canvas_viewports.clone(),
        };
        build_ui_frame(
            tree,
            &layouts,
            UiSize::new(bounds.width, bounds.height),
            interaction_state,
            atlas_source,
        )
    }

    fn dispatch_keyboard_event(
        &mut self,
        tree: &UiTree,
        layouts: &ComputedLayoutMap,
        event: &ui_input::KeyboardEvent,
    ) -> UiInputOutcome {
        if matches!(event.key, Key::Escape)
            && matches!(event.state, KeyState::Pressed | KeyState::Repeated)
            && let Some(outcome) = self.dispatch_graph_canvas_cancel(tree)
        {
            return outcome;
        }

        if matches!(event.key, Key::Escape)
            && matches!(event.state, KeyState::Pressed | KeyState::Repeated)
            && let Some(outcome) = self.dispatch_popup_escape_dismiss(tree, layouts)
        {
            return outcome;
        }

        if matches!(event.key, Key::Tab)
            && matches!(event.state, KeyState::Pressed | KeyState::Repeated)
        {
            return self.dispatch_focus_traversal(tree, event.modifiers.shift);
        }

        let Some(target) = self.focused_widget_in_tree(tree) else {
            return UiInputOutcome::ignored();
        };

        if matches!(event.state, KeyState::Pressed | KeyState::Repeated)
            && is_graph_canvas_widget(tree, target)
            && let Some(shortcut_action) = graph_canvas_shortcut_action(event)
        {
            let mut interactions = UiInteractionResults::new();
            interactions.push(UiInteraction::GraphCanvasAction {
                target,
                action: match shortcut_action {
                    ui_graph_editor::GraphShortcutAction::DeleteSelection => {
                        ui_graph_editor::GraphCanvasAction::KeyboardDeleteSelection
                    }
                    action => ui_graph_editor::GraphCanvasAction::KeyboardShortcut(action),
                },
            });
            return outcome(
                Some(target),
                InputResponse {
                    propagation: EventPropagation::Stop,
                    capture: PointerCapture::None,
                    focus_change: FocusChange::None,
                    repaint: true,
                    relayout: false,
                },
                interactions,
            );
        }

        let mut interactions = UiInteractionResults::new();
        interactions.push(UiInteraction::KeyboardInput {
            target,
            event: event.clone(),
        });

        let response = InputResponse {
            propagation: EventPropagation::Stop,
            capture: PointerCapture::None,
            focus_change: FocusChange::None,
            repaint: false,
            relayout: false,
        };

        outcome(Some(target), response, interactions)
    }

    fn dispatch_popup_escape_dismiss(
        &mut self,
        tree: &UiTree,
        layouts: &ComputedLayoutMap,
    ) -> Option<UiInputOutcome> {
        let (popup, anchor) = topmost_popup_scope(tree, layouts)?;
        let focus_return = tree.walk().any(|node| node.id == anchor).then_some(anchor);
        let focus_change = self.set_focus(focus_return);

        let mut interactions = UiInteractionResults::new();
        interactions.push(UiInteraction::PopupDismissRequested {
            popup,
            focus_return,
        });
        if !matches!(focus_change, FocusChange::None) {
            interactions.push(UiInteraction::FocusChanged(focus_change));
        }

        let response = InputResponse {
            propagation: EventPropagation::Stop,
            capture: PointerCapture::None,
            focus_change,
            repaint: !matches!(focus_change, FocusChange::None),
            relayout: false,
        };

        Some(outcome(Some(popup), response, interactions))
    }

    fn dispatch_graph_canvas_cancel(&mut self, tree: &UiTree) -> Option<UiInputOutcome> {
        let target = self.focused_widget_in_tree(tree)?;
        if !is_graph_canvas_widget(tree, target) {
            return None;
        }
        let action = self
            .state
            .graph_canvas_gestures
            .entry(target)
            .or_default()
            .cancel()?;
        if self.state.captured_widget == Some(target) {
            self.state.captured_widget = None;
        }
        if self.state.pressed_widget == Some(target) {
            self.state.pressed_widget = None;
        }

        let mut interactions = UiInteractionResults::new();
        interactions.push(UiInteraction::GraphCanvasAction { target, action });

        Some(outcome(
            Some(target),
            InputResponse {
                propagation: EventPropagation::Stop,
                capture: PointerCapture::Release,
                focus_change: FocusChange::None,
                repaint: true,
                relayout: false,
            },
            interactions,
        ))
    }

    fn dispatch_text_event(
        &mut self,
        tree: &UiTree,
        event: &ui_input::TextInputEvent,
    ) -> UiInputOutcome {
        let Some(target) = self.focused_widget_in_tree(tree) else {
            return UiInputOutcome::ignored();
        };

        let mut interactions = UiInteractionResults::new();
        interactions.push(UiInteraction::TextInput {
            target,
            event: event.clone(),
        });

        let response = InputResponse {
            propagation: EventPropagation::Stop,
            capture: PointerCapture::None,
            focus_change: FocusChange::None,
            repaint: true,
            relayout: true,
        };

        outcome(Some(target), response, interactions)
    }

    fn dispatch_focus_traversal(&mut self, tree: &UiTree, reverse: bool) -> UiInputOutcome {
        let focusable = focusable_widgets(tree);
        let Some(next_focus) = next_focus_target(
            &focusable,
            self.state.focused_target.map(|target| WidgetId(target.0)),
            reverse,
        ) else {
            return UiInputOutcome::ignored();
        };

        let focus_change = self.set_focus(Some(next_focus));
        let mut interactions = UiInteractionResults::new();
        if !matches!(focus_change, FocusChange::None) {
            interactions.push(UiInteraction::FocusChanged(focus_change));
        }

        let response = InputResponse {
            propagation: EventPropagation::Stop,
            capture: PointerCapture::None,
            focus_change,
            repaint: !matches!(focus_change, FocusChange::None),
            relayout: false,
        };

        outcome(Some(next_focus), response, interactions)
    }

    fn focused_widget_in_tree(&self, tree: &UiTree) -> Option<WidgetId> {
        let target = self.state.focused_target?;
        let widget_id = WidgetId(target.0);
        if tree.walk().any(|node| node.id == widget_id) {
            Some(widget_id)
        } else {
            None
        }
    }

    fn set_focus(&mut self, widget_id: Option<WidgetId>) -> FocusChange {
        let next = widget_id.map(|value| FocusTargetId(value.0));
        if self.state.focused_target == next {
            return FocusChange::None;
        }
        self.state.focused_target = next;
        match next {
            Some(target) => FocusChange::Set(target),
            None => FocusChange::Clear,
        }
    }
}
