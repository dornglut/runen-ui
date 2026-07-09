//! File: domain/ui/ui_runtime/src/input/pointer/hover.rs
//! Purpose: Hover interaction state helpers.

use crate::{UiInteraction, UiInteractionResults, WidgetId};

pub(super) fn push_hover_change_if_needed(
    interactions: &mut UiInteractionResults,
    previous: Option<WidgetId>,
    current: Option<WidgetId>,
) {
    if previous != current {
        interactions.push(UiInteraction::HoveredChanged { previous, current });
    }
}
