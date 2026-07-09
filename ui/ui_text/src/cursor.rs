//! File: domain/ui/ui_text/src/cursor.rs
//! Purpose: Caret positions in text buffers.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct TextCursor {
    pub char_index: usize,
}
