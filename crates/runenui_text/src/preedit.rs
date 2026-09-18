//! Transient IME preedit display projection.

use core::{error::Error, fmt};
use std::sync::Arc;

use runenui_core::{
    CompositionGeneration, CompositionRange, TextAffinity, TextDisplayPosition,
    TextDocumentSnapshot, TextPosition, TextPositionError, TextPreeditPosition, TextRange,
};

/// Failure while constructing or addressing a transient preedit projection.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextPreeditProjectionError {
    SnapshotMismatch,
    InvalidPreeditSelection,
    DisplayLengthOverflow,
    DisplayOffsetOutOfBounds,
    DisplayOffsetNotScalarBoundary,
    ForeignComposition,
    InvalidBoundaryAffinity,
    HiddenDocumentPosition,
    InvalidPosition(TextPositionError),
}

impl fmt::Display for TextPreeditProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SnapshotMismatch => "preedit projection addresses a different document snapshot",
            Self::InvalidPreeditSelection => {
                "preedit selection is invalid for the exact preedit string"
            }
            Self::DisplayLengthOverflow => "preedit display length overflowed",
            Self::DisplayOffsetOutOfBounds => "preedit display offset is out of bounds",
            Self::DisplayOffsetNotScalarBoundary => {
                "preedit display offset splits a Unicode scalar"
            }
            Self::ForeignComposition => "synthetic position belongs to another composition",
            Self::InvalidBoundaryAffinity => {
                "position affinity addresses the other side of a preedit boundary"
            }
            Self::HiddenDocumentPosition => {
                "document position is hidden by the active preedit replacement"
            }
            Self::InvalidPosition(_) => "preedit projection position is invalid",
        })
    }
}

impl Error for TextPreeditProjectionError {}

impl From<TextPositionError> for TextPreeditProjectionError {
    fn from(error: TextPositionError) -> Self {
        Self::InvalidPosition(error)
    }
}

/// Immutable transient display projection for one exact IME composition update.
///
/// The authoritative document is never mutated. `display_text` is the staged
/// rendering input formed by replacing `replacement` with `preedit`; mapping
/// methods preserve whether an offset is durable document text or synthetic
/// preedit text.
#[derive(Clone, Eq, PartialEq)]
pub struct TextPreeditProjection {
    snapshot: TextDocumentSnapshot,
    document: Arc<str>,
    replacement: TextRange,
    generation: CompositionGeneration,
    preedit: Arc<str>,
    selection: Option<CompositionRange>,
    display_text: Arc<str>,
}

impl fmt::Debug for TextPreeditProjection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextPreeditProjection")
            .field("snapshot", &self.snapshot)
            .field("replacement", &self.replacement)
            .field("generation", &self.generation)
            .field("preedit_len", &self.preedit.len())
            .field("selection", &self.selection)
            .field("display_len", &self.display_text.len())
            .finish_non_exhaustive()
    }
}

impl TextPreeditProjection {
    /// Builds one immutable display projection without mutating `document`.
    ///
    /// # Errors
    ///
    /// Rejects a foreign replacement snapshot, a selection invalid for this
    /// exact preedit string, or display length overflow.
    pub fn new(
        snapshot: TextDocumentSnapshot,
        document: impl Into<Arc<str>>,
        replacement: TextRange,
        generation: CompositionGeneration,
        preedit: impl Into<Arc<str>>,
        selection: Option<CompositionRange>,
    ) -> Result<Self, TextPreeditProjectionError> {
        if replacement.snapshot() != snapshot {
            return Err(TextPreeditProjectionError::SnapshotMismatch);
        }
        let document = document.into();
        let preedit = preedit.into();
        if let Some(selection) = selection {
            CompositionRange::new(&preedit, selection.start(), selection.end())
                .map_err(|_| TextPreeditProjectionError::InvalidPreeditSelection)?;
        }
        let prefix = document
            .get(..replacement.start())
            .ok_or(TextPreeditProjectionError::HiddenDocumentPosition)?;
        let suffix = document
            .get(replacement.end()..)
            .ok_or(TextPreeditProjectionError::HiddenDocumentPosition)?;
        let capacity = prefix
            .len()
            .checked_add(preedit.len())
            .and_then(|value| value.checked_add(suffix.len()))
            .ok_or(TextPreeditProjectionError::DisplayLengthOverflow)?;
        let mut display_text = String::with_capacity(capacity);
        display_text.push_str(prefix);
        display_text.push_str(&preedit);
        display_text.push_str(suffix);
        Ok(Self {
            snapshot,
            document,
            replacement,
            generation,
            preedit,
            selection,
            display_text: display_text.into(),
        })
    }

    #[must_use]
    pub const fn snapshot(&self) -> TextDocumentSnapshot {
        self.snapshot
    }

    #[must_use]
    pub const fn replacement(&self) -> TextRange {
        self.replacement
    }

    /// Returns the exact immutable authoritative source used to build this projection.
    #[must_use]
    pub fn document_text(&self) -> &str {
        &self.document
    }

    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        &self.generation
    }

    #[must_use]
    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    #[must_use]
    pub const fn selection(&self) -> Option<CompositionRange> {
        self.selection
    }

    #[must_use]
    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    #[must_use]
    pub const fn display_preedit_start(&self) -> usize {
        self.replacement.start()
    }

    #[must_use]
    pub fn display_preedit_end(&self) -> usize {
        self.display_preedit_start() + self.preedit.len()
    }

    /// Maps one checked display byte boundary into durable or synthetic space.
    ///
    /// At the projection boundaries, affinity disambiguates the document side
    /// from the preedit side.
    ///
    /// # Errors
    ///
    /// Rejects out-of-bounds or split-scalar display offsets and unrepresentable
    /// mapped document/preedit positions.
    pub fn position_from_display_offset(
        &self,
        display_offset: usize,
        affinity: TextAffinity,
    ) -> Result<TextDisplayPosition, TextPreeditProjectionError> {
        if display_offset > self.display_text.len() {
            return Err(TextPreeditProjectionError::DisplayOffsetOutOfBounds);
        }
        if !self.display_text.is_char_boundary(display_offset) {
            return Err(TextPreeditProjectionError::DisplayOffsetNotScalarBoundary);
        }
        let display_start = self.display_preedit_start();
        let display_end = self.display_preedit_end();
        let replacement = self.replacement;

        if display_offset < display_start {
            return self.document_position(display_offset, affinity);
        }
        if display_offset == display_start && affinity == TextAffinity::Upstream {
            return self.document_position(replacement.start(), affinity);
        }
        if display_offset < display_end
            || (display_offset == display_end
                && !self.preedit.is_empty()
                && affinity == TextAffinity::Upstream)
        {
            let relative = display_offset - display_start;
            return Ok(TextDisplayPosition::Preedit(TextPreeditPosition::new(
                self.snapshot,
                self.generation.clone(),
                &self.preedit,
                relative,
                affinity,
            )?));
        }
        if display_offset == display_start && self.preedit.is_empty() {
            let document_offset = if affinity == TextAffinity::Upstream {
                replacement.start()
            } else {
                replacement.end()
            };
            return self.document_position(document_offset, affinity);
        }

        let suffix_offset = display_offset - display_end;
        let document_offset = replacement
            .end()
            .checked_add(suffix_offset)
            .ok_or(TextPreeditProjectionError::DisplayLengthOverflow)?;
        self.document_position(document_offset, affinity)
    }

    /// Maps a durable or synthetic position into this projection's display bytes.
    ///
    /// # Errors
    ///
    /// Rejects stale snapshots, foreign composition generations, and document
    /// positions hidden by the replaced range.
    pub fn display_offset_for_position(
        &self,
        position: &TextDisplayPosition,
    ) -> Result<usize, TextPreeditProjectionError> {
        if position.snapshot() != self.snapshot {
            return Err(TextPreeditProjectionError::SnapshotMismatch);
        }
        match position {
            TextDisplayPosition::Preedit(position) => {
                if position.generation() != &self.generation {
                    return Err(TextPreeditProjectionError::ForeignComposition);
                }
                TextPreeditPosition::new(
                    self.snapshot,
                    self.generation.clone(),
                    &self.preedit,
                    position.byte_offset(),
                    position.affinity(),
                )?;
                if self.preedit.is_empty()
                    || position.byte_offset() == 0
                        && position.affinity() != TextAffinity::Downstream
                    || position.byte_offset() == self.preedit.len()
                        && position.affinity() != TextAffinity::Upstream
                {
                    return Err(TextPreeditProjectionError::InvalidBoundaryAffinity);
                }
                self.display_preedit_start()
                    .checked_add(position.byte_offset())
                    .ok_or(TextPreeditProjectionError::DisplayLengthOverflow)
            }
            TextDisplayPosition::Document(position) => {
                TextPosition::new(
                    self.snapshot,
                    &self.document,
                    position.byte_offset(),
                    position.affinity(),
                )?;
                let offset = position.byte_offset();
                let start = self.replacement.start();
                let end = self.replacement.end();
                if offset < start {
                    return Ok(offset);
                }
                if start == end && offset == start {
                    return Ok(if position.affinity() == TextAffinity::Upstream {
                        start
                    } else {
                        self.display_preedit_end()
                    });
                }
                if offset == start && position.affinity() == TextAffinity::Upstream {
                    return Ok(start);
                }
                if offset == end && position.affinity() == TextAffinity::Downstream {
                    return Ok(self.display_preedit_end());
                }
                if offset <= end {
                    return Err(TextPreeditProjectionError::HiddenDocumentPosition);
                }
                self.display_preedit_end()
                    .checked_add(offset - end)
                    .ok_or(TextPreeditProjectionError::DisplayLengthOverflow)
            }
        }
    }

    fn document_position(
        &self,
        byte_offset: usize,
        affinity: TextAffinity,
    ) -> Result<TextDisplayPosition, TextPreeditProjectionError> {
        Ok(TextDisplayPosition::Document(TextPosition::new(
            self.snapshot,
            &self.document,
            byte_offset,
            affinity,
        )?))
    }
}
