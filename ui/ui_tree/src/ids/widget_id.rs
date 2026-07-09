//! File: domain/ui/ui_runtime/src/ids/widget_id.rs
//! Purpose: Stable widget identifier.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetId(pub u64);
