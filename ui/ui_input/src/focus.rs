//! File: domain/ui/ui_input/src/focus.rs
//! Purpose: Stable focus target identifiers and focus movement contracts.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FocusTargetId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusChange {
    None,
    Set(FocusTargetId),
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    Next,
    Previous,
    Up,
    Down,
    Left,
    Right,
}
