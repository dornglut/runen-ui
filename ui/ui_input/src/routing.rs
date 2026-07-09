//! File: domain/ui/ui_input/src/routing.rs
//! Purpose: Input propagation and capture contracts.

use crate::FocusChange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPropagation {
    Continue,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerCapture {
    None,
    CaptureSelf,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputResponse {
    pub propagation: EventPropagation,
    pub capture: PointerCapture,
    pub focus_change: FocusChange,
    pub repaint: bool,
    pub relayout: bool,
}

impl InputResponse {
    pub const fn ignored() -> Self {
        Self {
            propagation: EventPropagation::Continue,
            capture: PointerCapture::None,
            focus_change: FocusChange::None,
            repaint: false,
            relayout: false,
        }
    }

    pub const fn handled() -> Self {
        Self {
            propagation: EventPropagation::Stop,
            capture: PointerCapture::None,
            focus_change: FocusChange::None,
            repaint: false,
            relayout: false,
        }
    }
}
