//! File: domain/ui/ui_runtime/src/state/runtime_state.rs
//! Purpose: Persistent runtime state across UI frames.

use crate::WidgetId;
use std::collections::BTreeMap;
use ui_input::FocusTargetId;
use ui_math::{Axis, UiPoint};

const SCROLLBAR_ACTIVITY_HOLD_FRAMES: u64 = 18;
const SCROLLBAR_ACTIVITY_FADE_FRAMES: u64 = 24;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarThumbDragState {
    pub scroll_widget: WidgetId,
    pub axis: Axis,
    pub pointer_grab_offset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarAxisTarget {
    pub widget_id: WidgetId,
    pub axis: Axis,
}

impl ScrollbarAxisTarget {
    pub fn new(widget_id: WidgetId, axis: Axis) -> Self {
        Self { widget_id, axis }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollbarAxisActivityFrames {
    pub horizontal: Option<u64>,
    pub vertical: Option<u64>,
}

impl ScrollbarAxisActivityFrames {
    pub fn for_axis(self, axis: Axis) -> Option<u64> {
        match axis {
            Axis::Horizontal => self.horizontal,
            Axis::Vertical => self.vertical,
        }
    }

    pub fn set_axis(&mut self, axis: Axis, frame: u64) {
        match axis {
            Axis::Horizontal => self.horizontal = Some(frame),
            Axis::Vertical => self.vertical = Some(frame),
        }
    }

    pub fn retain_recent(&mut self, current_frame: u64, max_age: u64) {
        if self
            .horizontal
            .is_some_and(|frame| current_frame.saturating_sub(frame) > max_age)
        {
            self.horizontal = None;
        }
        if self
            .vertical
            .is_some_and(|frame| current_frame.saturating_sub(frame) > max_age)
        {
            self.vertical = None;
        }
    }

    pub fn is_empty(self) -> bool {
        self.horizontal.is_none() && self.vertical.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollbarAxisOpacities {
    pub horizontal: f32,
    pub vertical: f32,
}

impl ScrollbarAxisOpacities {
    pub fn for_axis(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.horizontal,
            Axis::Vertical => self.vertical,
        }
    }

    pub fn set_axis(&mut self, axis: Axis, opacity: f32) {
        match axis {
            Axis::Horizontal => self.horizontal = opacity,
            Axis::Vertical => self.vertical = opacity,
        }
    }

    pub fn is_empty(self) -> bool {
        self.horizontal <= 0.0 && self.vertical <= 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollOffset {
    pub horizontal: f32,
    pub vertical: f32,
}

impl ScrollOffset {
    pub fn for_axis(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.horizontal,
            Axis::Vertical => self.vertical,
        }
    }

    pub fn with_axis(mut self, axis: Axis, offset: f32) -> Self {
        match axis {
            Axis::Horizontal => self.horizontal = offset.max(0.0),
            Axis::Vertical => self.vertical = offset.max(0.0),
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiRuntimeState {
    pub hovered_widget: Option<WidgetId>,
    pub pressed_widget: Option<WidgetId>,
    pub captured_widget: Option<WidgetId>,
    pub focused_target: Option<FocusTargetId>,
    pub scroll_offsets: BTreeMap<WidgetId, ScrollOffset>,
    pub middle_pan_anchor: Option<WidgetId>,
    pub middle_pan_last_position: Option<UiPoint>,
    pub scrollbar_thumb_drag: Option<ScrollbarThumbDragState>,
    pub hovered_scrollbar: Option<ScrollbarAxisTarget>,
    pub active_scrollbar: Option<ScrollbarAxisTarget>,
    pub frame_index: u64,
    pub scrollbar_activity_frames: BTreeMap<WidgetId, ScrollbarAxisActivityFrames>,
    pub graph_canvas_gestures: BTreeMap<WidgetId, ui_graph_editor::GraphCanvasGestureState>,
    pub graph_canvas_viewports: BTreeMap<WidgetId, ui_graph_editor::GraphViewport>,
}

impl UiRuntimeState {
    pub fn clear_pointer_state(&mut self) {
        self.hovered_widget = None;
        self.pressed_widget = None;
        self.captured_widget = None;
        self.middle_pan_anchor = None;
        self.middle_pan_last_position = None;
        self.scrollbar_thumb_drag = None;
        self.hovered_scrollbar = None;
        self.graph_canvas_gestures.clear();
    }

    pub fn scroll_offset(&self, widget_id: WidgetId) -> f32 {
        self.scroll_offset_for_axis(widget_id, Axis::Vertical)
    }

    pub fn set_scroll_offset(&mut self, widget_id: WidgetId, offset: f32) {
        self.set_scroll_offset_for_axis(widget_id, Axis::Vertical, offset);
    }

    pub fn scroll_offset_for_axis(&self, widget_id: WidgetId, axis: Axis) -> f32 {
        self.scroll_offsets
            .get(&widget_id)
            .copied()
            .unwrap_or_default()
            .for_axis(axis)
    }

    pub fn set_scroll_offset_for_axis(&mut self, widget_id: WidgetId, axis: Axis, offset: f32) {
        let current = self
            .scroll_offsets
            .get(&widget_id)
            .copied()
            .unwrap_or_default();
        self.scroll_offsets
            .insert(widget_id, current.with_axis(axis, offset));
    }

    pub fn mark_scrollbar_active(&mut self, widget_id: WidgetId, axis: Axis) {
        self.active_scrollbar = Some(ScrollbarAxisTarget::new(widget_id, axis));
        self.scrollbar_activity_frames
            .entry(widget_id)
            .or_default()
            .set_axis(axis, self.frame_index);
    }

    pub fn advance_frame(&mut self) {
        self.frame_index = self.frame_index.saturating_add(1);
        let current_frame = self.frame_index;
        let max_age = SCROLLBAR_ACTIVITY_HOLD_FRAMES + SCROLLBAR_ACTIVITY_FADE_FRAMES;
        self.scrollbar_activity_frames
            .values_mut()
            .for_each(|frames| frames.retain_recent(current_frame, max_age));
        self.scrollbar_activity_frames
            .retain(|_, frames| !frames.is_empty());
        if let Some(active_scrollbar) = self.active_scrollbar
            && self
                .scrollbar_activity_frames
                .get(&active_scrollbar.widget_id)
                .and_then(|frames| frames.for_axis(active_scrollbar.axis))
                .is_none()
            && self.scrollbar_thumb_drag.is_none_or(|drag| {
                drag.scroll_widget != active_scrollbar.widget_id
                    || drag.axis != active_scrollbar.axis
            })
        {
            self.active_scrollbar = None;
        }
    }

    pub fn scrollbar_opacity(&self, widget_id: WidgetId, axis: Axis) -> f32 {
        if self
            .scrollbar_thumb_drag
            .is_some_and(|drag| drag.scroll_widget == widget_id && drag.axis == axis)
        {
            return 1.0;
        }
        let Some(active_frame) = self
            .scrollbar_activity_frames
            .get(&widget_id)
            .and_then(|frames| frames.for_axis(axis))
        else {
            return 0.0;
        };
        let elapsed = self.frame_index.saturating_sub(active_frame);
        if elapsed <= SCROLLBAR_ACTIVITY_HOLD_FRAMES {
            return 1.0;
        }
        let fade_elapsed = elapsed - SCROLLBAR_ACTIVITY_HOLD_FRAMES;
        let fade_remaining = SCROLLBAR_ACTIVITY_FADE_FRAMES.saturating_sub(fade_elapsed) as f32;
        (fade_remaining / SCROLLBAR_ACTIVITY_FADE_FRAMES as f32).clamp(0.0, 1.0)
    }

    pub fn scrollbar_opacity_entries(&self) -> BTreeMap<WidgetId, ScrollbarAxisOpacities> {
        let mut opacities = self
            .scrollbar_activity_frames
            .keys()
            .filter_map(|widget_id| {
                let mut opacities = ScrollbarAxisOpacities::default();
                for axis in [Axis::Horizontal, Axis::Vertical] {
                    opacities.set_axis(axis, self.scrollbar_opacity(*widget_id, axis));
                }
                (!opacities.is_empty()).then_some((*widget_id, opacities))
            })
            .collect::<BTreeMap<_, _>>();
        if let Some(drag) = self.scrollbar_thumb_drag {
            opacities
                .entry(drag.scroll_widget)
                .or_default()
                .set_axis(drag.axis, 1.0);
        }
        opacities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollbar_activity_opacity_fades_after_recent_input() {
        let widget_id = WidgetId(42);
        let mut state = UiRuntimeState::default();
        state.mark_scrollbar_active(widget_id, Axis::Vertical);
        assert_eq!(state.scrollbar_opacity(widget_id, Axis::Vertical), 1.0);
        assert_eq!(state.scrollbar_opacity(widget_id, Axis::Horizontal), 0.0);

        for _ in 0..SCROLLBAR_ACTIVITY_HOLD_FRAMES {
            state.advance_frame();
        }
        assert_eq!(state.scrollbar_opacity(widget_id, Axis::Vertical), 1.0);

        state.advance_frame();
        let fading_opacity = state.scrollbar_opacity(widget_id, Axis::Vertical);
        assert!(
            fading_opacity > 0.0 && fading_opacity < 1.0,
            "scrollbar opacity should fade after the hold window"
        );

        for _ in 0..SCROLLBAR_ACTIVITY_FADE_FRAMES {
            state.advance_frame();
        }
        assert_eq!(state.scrollbar_opacity(widget_id, Axis::Vertical), 0.0);
    }

    #[test]
    fn scrollbar_activity_tracks_axes_independently() {
        let widget_id = WidgetId(42);
        let mut state = UiRuntimeState::default();
        state.mark_scrollbar_active(widget_id, Axis::Horizontal);
        assert_eq!(state.scrollbar_opacity(widget_id, Axis::Horizontal), 1.0);
        assert_eq!(state.scrollbar_opacity(widget_id, Axis::Vertical), 0.0);

        for _ in 0..(SCROLLBAR_ACTIVITY_HOLD_FRAMES + 1) {
            state.advance_frame();
        }
        let horizontal_opacity = state.scrollbar_opacity(widget_id, Axis::Horizontal);
        assert!(
            horizontal_opacity > 0.0 && horizontal_opacity < 1.0,
            "horizontal scrollbar should have started fading"
        );

        state.mark_scrollbar_active(widget_id, Axis::Vertical);
        assert_eq!(state.scrollbar_opacity(widget_id, Axis::Vertical), 1.0);
        assert!(
            state.scrollbar_opacity(widget_id, Axis::Horizontal) < 1.0,
            "refreshing vertical activity must not refresh horizontal opacity"
        );
    }

    #[test]
    fn scrollbar_thumb_drag_keeps_only_dragged_axis_visible() {
        let widget_id = WidgetId(42);
        let state = UiRuntimeState {
            scrollbar_thumb_drag: Some(ScrollbarThumbDragState {
                scroll_widget: widget_id,
                axis: Axis::Horizontal,
                pointer_grab_offset: 0.0,
            }),
            ..Default::default()
        };

        assert_eq!(state.scrollbar_opacity(widget_id, Axis::Horizontal), 1.0);
        assert_eq!(state.scrollbar_opacity(widget_id, Axis::Vertical), 0.0);
    }
}
