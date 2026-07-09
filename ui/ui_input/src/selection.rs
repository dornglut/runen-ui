//! Normalized text selection facts.
//!
//! Positions are domain-shaped so public input vocabulary does not expose raw
//! Rust string byte offsets.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextPositionUnit {
    Opaque,
    Grapheme,
    LineColumn,
}

impl TextPositionUnit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Opaque => "opaque",
            Self::Grapheme => "grapheme",
            Self::LineColumn => "line-column",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextPosition {
    pub unit: TextPositionUnit,
    pub ordinal: u32,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl TextPosition {
    pub const fn opaque(ordinal: u32) -> Self {
        Self {
            unit: TextPositionUnit::Opaque,
            ordinal,
            line: None,
            column: None,
        }
    }

    pub const fn grapheme(ordinal: u32) -> Self {
        Self {
            unit: TextPositionUnit::Grapheme,
            ordinal,
            line: None,
            column: None,
        }
    }

    pub const fn line_column(line: u32, column: u32) -> Self {
        Self {
            unit: TextPositionUnit::LineColumn,
            ordinal: column,
            line: Some(line),
            column: Some(column),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextRange {
    pub anchor: TextPosition,
    pub extent: TextPosition,
}

impl TextRange {
    pub const fn collapsed(position: TextPosition) -> Self {
        Self {
            anchor: position,
            extent: position,
        }
    }

    pub const fn new(anchor: TextPosition, extent: TextPosition) -> Self {
        Self { anchor, extent }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.extent
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSelectionFact {
    pub target_id: Option<String>,
    pub range: TextRange,
    pub reason: TextSelectionReason,
}

impl TextSelectionFact {
    pub fn caret(position: TextPosition) -> Self {
        Self {
            target_id: None,
            range: TextRange::collapsed(position),
            reason: TextSelectionReason::CaretMove,
        }
    }

    pub fn range(range: TextRange) -> Self {
        Self {
            target_id: None,
            range,
            reason: TextSelectionReason::RangeSelection,
        }
    }

    pub fn with_target(mut self, target_id: impl Into<String>) -> Self {
        self.target_id = Some(target_id.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextSelectionReason {
    FocusPlacement,
    CaretMove,
    RangeSelection,
    SelectionCollapse,
}

impl TextSelectionReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FocusPlacement => "focus-placement",
            Self::CaretMove => "caret-move",
            Self::RangeSelection => "range-selection",
            Self::SelectionCollapse => "selection-collapse",
        }
    }
}
