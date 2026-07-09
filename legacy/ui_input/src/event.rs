//! File: domain/ui/ui_input/src/event.rs
//! Purpose: High-level UI input events.

use crate::{
    Key, KeyState, Modifiers, PointerButton, PointerDelta, PointerEventKind, PointerPacket,
    PointerPosition, SemanticActionEvent,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerEventKind,
    pub position: PointerPosition,
    pub delta: PointerDelta,
    pub button: Option<PointerButton>,
    pub modifiers: Modifiers,
    pub click_count: u8,
    pub packet: PointerPacket,
}

impl PointerEvent {
    pub fn new(
        kind: PointerEventKind,
        position: PointerPosition,
        delta: PointerDelta,
        button: Option<PointerButton>,
        modifiers: Modifiers,
        click_count: u8,
    ) -> Self {
        Self {
            kind,
            position,
            delta,
            button,
            modifiers,
            click_count,
            packet: PointerPacket::default(),
        }
    }

    pub fn with_packet(mut self, packet: PointerPacket) -> Self {
        self.packet = packet;
        self
    }
}

impl Default for PointerEvent {
    fn default() -> Self {
        Self {
            kind: PointerEventKind::Move,
            position: PointerPosition::ZERO,
            delta: PointerDelta::ZERO,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            packet: PointerPacket::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardEvent {
    pub key: Key,
    pub state: KeyState,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInputEvent {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiInputEvent {
    Pointer(PointerEvent),
    Keyboard(KeyboardEvent),
    Semantic(SemanticActionEvent),
    Text(TextInputEvent),
}
