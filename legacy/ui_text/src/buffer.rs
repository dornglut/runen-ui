//! File: domain/ui/ui_text/src/buffer.rs
//! Purpose: Editable text buffer primitives.

use crate::{TextCursor, TextSelection};

#[derive(Debug, Clone, Default)]
pub struct TextBuffer {
    text: String,
}

impl TextBuffer {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    pub fn insert_str(&mut self, cursor: TextCursor, value: &str) -> TextCursor {
        let byte_index = char_to_byte_index(&self.text, cursor.char_index);
        self.text.insert_str(byte_index, value);
        TextCursor {
            char_index: cursor.char_index + value.chars().count(),
        }
    }

    pub fn replace_selection(&mut self, selection: TextSelection, value: &str) -> TextCursor {
        let range = selection.normalized_range();
        let start = char_to_byte_index(&self.text, range.start);
        let end = char_to_byte_index(&self.text, range.end);
        self.text.replace_range(start..end, value);
        TextCursor {
            char_index: range.start + value.chars().count(),
        }
    }

    pub fn remove_backward(&mut self, cursor: TextCursor) -> TextCursor {
        if cursor.char_index == 0 {
            return cursor;
        }

        let start = char_to_byte_index(&self.text, cursor.char_index - 1);
        let end = char_to_byte_index(&self.text, cursor.char_index);
        self.text.replace_range(start..end, "");

        TextCursor {
            char_index: cursor.char_index - 1,
        }
    }
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}
