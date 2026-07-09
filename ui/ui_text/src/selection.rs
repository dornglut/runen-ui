//! File: domain/ui/ui_text/src/selection.rs
//! Purpose: Selection range model.

use crate::TextCursor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: TextCursor,
    pub caret: TextCursor,
}

impl TextSelection {
    pub fn collapsed(cursor: TextCursor) -> Self {
        Self {
            anchor: cursor,
            caret: cursor,
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.caret
    }

    pub fn normalized_range(&self) -> std::ops::Range<usize> {
        let start = self.anchor.char_index.min(self.caret.char_index);
        let end = self.anchor.char_index.max(self.caret.char_index);
        start..end
    }
}
