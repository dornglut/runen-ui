//! File: domain/ui/ui_runtime/src/layout/computed_layout.rs
//! Purpose: Per-widget arranged layout records.

use std::collections::BTreeMap;

use ui_math::{UiRect, UiSize};

use crate::WidgetId;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputedLayout {
    pub bounds: UiRect,
    pub content_bounds: UiRect,
    pub measured_size: UiSize,
}

impl ComputedLayout {
    pub fn new(bounds: UiRect, content_bounds: UiRect, measured_size: UiSize) -> Self {
        Self {
            bounds,
            content_bounds,
            measured_size,
        }
    }
}

pub type ComputedLayoutMap = BTreeMap<WidgetId, ComputedLayout>;
