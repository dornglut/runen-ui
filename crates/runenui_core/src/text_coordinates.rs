//! Host-neutral, revision-scoped text coordinate values.

use core::{error::Error, fmt};

use crate::CompositionGeneration;

/// Application-authored stable identity for one authoritative text document.
///
/// This value names application state; it does not confer mounted, editing-session,
/// or runtime authority.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TextDocumentId(u64);

impl TextDocumentId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Application-authored revision within one text document's history.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TextDocumentRevision(u64);

impl TextDocumentRevision {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Exact application document revision addressed by text coordinates.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextDocumentSnapshot {
    document: TextDocumentId,
    revision: TextDocumentRevision,
}

impl TextDocumentSnapshot {
    #[must_use]
    pub const fn new(document: TextDocumentId, revision: TextDocumentRevision) -> Self {
        Self { document, revision }
    }

    #[must_use]
    pub const fn document(self) -> TextDocumentId {
        self.document
    }

    #[must_use]
    pub const fn revision(self) -> TextDocumentRevision {
        self.revision
    }
}

/// Visual side of a text boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextAffinity {
    /// The caret is visually associated with content before the byte offset.
    Upstream,
    /// The caret is visually associated with content after the byte offset.
    Downstream,
}

/// Failure while constructing a scalar-aligned durable text position.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextPositionError {
    OutOfBounds,
    NotScalarBoundary,
    Utf16SplitScalar,
    Utf16OffsetOverflow,
}

impl fmt::Display for TextPositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OutOfBounds => "text position is outside the exact source snapshot",
            Self::NotScalarBoundary => "text position splits a UTF-8 scalar",
            Self::Utf16SplitScalar => "UTF-16 offset splits a surrogate pair",
            Self::Utf16OffsetOverflow => "UTF-16 offset cannot be represented safely",
        })
    }
}

impl Error for TextPositionError {}

/// Scalar-aligned UTF-8 byte position in one exact document revision.
///
/// Construction proves bounds and Unicode scalar alignment. A text layout's
/// caret map separately proves grapheme and shaping-valid caret alignment.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextPosition {
    snapshot: TextDocumentSnapshot,
    byte_offset: usize,
    affinity: TextAffinity,
}

impl TextPosition {
    /// Creates a checked UTF-8 position in `source`.
    ///
    /// # Errors
    ///
    /// Returns [`TextPositionError`] for an out-of-bounds or split-scalar offset.
    pub const fn new(
        snapshot: TextDocumentSnapshot,
        source: &str,
        byte_offset: usize,
        affinity: TextAffinity,
    ) -> Result<Self, TextPositionError> {
        if byte_offset > source.len() {
            return Err(TextPositionError::OutOfBounds);
        }
        if !source.is_char_boundary(byte_offset) {
            return Err(TextPositionError::NotScalarBoundary);
        }
        Ok(Self {
            snapshot,
            byte_offset,
            affinity,
        })
    }

    /// Converts an exact UTF-16 code-unit offset at a host boundary.
    ///
    /// # Errors
    ///
    /// Rejects offsets outside `source`, offsets inside a surrogate pair, and
    /// arithmetic overflow. The returned value is always a canonical UTF-8 byte
    /// coordinate rather than retaining a native code-unit index.
    pub fn from_utf16_offset(
        snapshot: TextDocumentSnapshot,
        source: &str,
        utf16_offset: u64,
        affinity: TextAffinity,
    ) -> Result<Self, TextPositionError> {
        let mut units = 0_u64;
        for (byte_offset, scalar) in source.char_indices() {
            if units == utf16_offset {
                return Self::new(snapshot, source, byte_offset, affinity);
            }
            let scalar_units = u64::try_from(scalar.len_utf16())
                .map_err(|_| TextPositionError::Utf16OffsetOverflow)?;
            let next = units
                .checked_add(scalar_units)
                .ok_or(TextPositionError::Utf16OffsetOverflow)?;
            if utf16_offset < next {
                return Err(TextPositionError::Utf16SplitScalar);
            }
            units = next;
        }
        if units == utf16_offset {
            Self::new(snapshot, source, source.len(), affinity)
        } else {
            Err(TextPositionError::OutOfBounds)
        }
    }

    #[must_use]
    pub const fn snapshot(self) -> TextDocumentSnapshot {
        self.snapshot
    }

    #[must_use]
    pub const fn byte_offset(self) -> usize {
        self.byte_offset
    }

    #[must_use]
    pub const fn affinity(self) -> TextAffinity {
        self.affinity
    }
}

/// Failure while constructing a document range.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextRangeError {
    Reversed,
    Endpoint(TextPositionError),
}

impl fmt::Display for TextRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reversed => formatter.write_str("text range is reversed"),
            Self::Endpoint(error) => write!(formatter, "invalid text range endpoint: {error}"),
        }
    }
}

impl Error for TextRangeError {}

/// Ordered half-open UTF-8 byte range in one exact document revision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextRange {
    snapshot: TextDocumentSnapshot,
    start: usize,
    end: usize,
}

impl TextRange {
    /// Creates a scalar-aligned ordered range.
    ///
    /// # Errors
    ///
    /// Rejects reversed, out-of-bounds, or split-scalar endpoints.
    pub const fn new(
        snapshot: TextDocumentSnapshot,
        source: &str,
        start: usize,
        end: usize,
    ) -> Result<Self, TextRangeError> {
        if start > end {
            return Err(TextRangeError::Reversed);
        }
        if start > source.len() || end > source.len() {
            return Err(TextRangeError::Endpoint(TextPositionError::OutOfBounds));
        }
        if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
            return Err(TextRangeError::Endpoint(
                TextPositionError::NotScalarBoundary,
            ));
        }
        Ok(Self {
            snapshot,
            start,
            end,
        })
    }

    #[must_use]
    pub const fn snapshot(self) -> TextDocumentSnapshot {
        self.snapshot
    }

    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Failure while combining directional selection endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextSelectionSnapshotMismatch;

impl fmt::Display for TextSelectionSnapshotMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("selection endpoints address different document revisions")
    }
}

impl Error for TextSelectionSnapshotMismatch {}

/// Logical direction retained by a selection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextSelectionDirection {
    Forward,
    Backward,
}

/// Directional selection in one exact document revision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextSelection {
    anchor: TextPosition,
    active: TextPosition,
}

impl TextSelection {
    /// Creates a selection without discarding direction or endpoint affinity.
    ///
    /// # Errors
    ///
    /// Rejects endpoints from different document snapshots.
    pub fn new(
        anchor: TextPosition,
        active: TextPosition,
    ) -> Result<Self, TextSelectionSnapshotMismatch> {
        if anchor.snapshot != active.snapshot {
            return Err(TextSelectionSnapshotMismatch);
        }
        Ok(Self { anchor, active })
    }

    #[must_use]
    pub const fn collapsed(position: TextPosition) -> Self {
        Self {
            anchor: position,
            active: position,
        }
    }

    #[must_use]
    pub const fn anchor(self) -> TextPosition {
        self.anchor
    }

    #[must_use]
    pub const fn active(self) -> TextPosition {
        self.active
    }

    #[must_use]
    pub const fn is_collapsed(self) -> bool {
        self.anchor.byte_offset == self.active.byte_offset
    }

    #[must_use]
    pub const fn direction(self) -> TextSelectionDirection {
        if self.active.byte_offset < self.anchor.byte_offset {
            TextSelectionDirection::Backward
        } else {
            TextSelectionDirection::Forward
        }
    }
}

/// Checked synthetic offset in one transient composition preedit.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TextPreeditPosition {
    snapshot: TextDocumentSnapshot,
    generation: CompositionGeneration,
    byte_offset: usize,
    affinity: TextAffinity,
}

impl TextPreeditPosition {
    /// Creates a scalar-aligned position relative to `preedit`.
    ///
    /// # Errors
    ///
    /// Rejects out-of-bounds and split-scalar offsets.
    pub fn new(
        snapshot: TextDocumentSnapshot,
        generation: CompositionGeneration,
        preedit: &str,
        byte_offset: usize,
        affinity: TextAffinity,
    ) -> Result<Self, TextPositionError> {
        let checked = TextPosition::new(snapshot, preedit, byte_offset, affinity)?;
        Ok(Self {
            snapshot,
            generation,
            byte_offset: checked.byte_offset,
            affinity,
        })
    }

    #[must_use]
    pub const fn snapshot(&self) -> TextDocumentSnapshot {
        self.snapshot
    }

    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        &self.generation
    }

    #[must_use]
    pub const fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    #[must_use]
    pub const fn affinity(&self) -> TextAffinity {
        self.affinity
    }
}

/// Position in a staged display, explicitly distinguishing durable document
/// coordinates from synthetic preedit coordinates.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TextDisplayPosition {
    Document(TextPosition),
    Preedit(TextPreeditPosition),
}

impl TextDisplayPosition {
    #[must_use]
    pub const fn snapshot(&self) -> TextDocumentSnapshot {
        match self {
            Self::Document(position) => position.snapshot(),
            Self::Preedit(position) => position.snapshot(),
        }
    }

    #[must_use]
    pub const fn affinity(&self) -> TextAffinity {
        match self {
            Self::Document(position) => position.affinity(),
            Self::Preedit(position) => position.affinity(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(revision: u64) -> TextDocumentSnapshot {
        TextDocumentSnapshot::new(TextDocumentId::new(7), TextDocumentRevision::new(revision))
    }

    #[test]
    fn utf8_positions_and_ranges_reject_split_or_unordered_coordinates() {
        let source = "aé";
        assert_eq!(
            TextPosition::new(snapshot(1), source, 2, TextAffinity::Downstream),
            Err(TextPositionError::NotScalarBoundary)
        );
        assert_eq!(
            TextRange::new(snapshot(1), source, 3, 1),
            Err(TextRangeError::Reversed)
        );
        assert_eq!(
            TextRange::new(snapshot(1), source, 0, 4),
            Err(TextRangeError::Endpoint(TextPositionError::OutOfBounds))
        );
    }

    #[test]
    fn utf16_conversion_rejects_surrogate_splits_and_returns_utf8_offsets() {
        let source = "a🙂z";
        assert_eq!(
            TextPosition::from_utf16_offset(snapshot(2), source, 2, TextAffinity::Downstream),
            Err(TextPositionError::Utf16SplitScalar)
        );
        let position =
            TextPosition::from_utf16_offset(snapshot(2), source, 3, TextAffinity::Upstream)
                .unwrap_or_else(|_| unreachable!("fixture coordinate is valid"));
        assert_eq!(position.byte_offset(), 5);
        assert_eq!(position.affinity(), TextAffinity::Upstream);
    }

    #[test]
    fn selection_preserves_direction_affinity_and_exact_snapshot() {
        let source = "abc";
        let anchor = TextPosition::new(snapshot(3), source, 3, TextAffinity::Upstream)
            .unwrap_or_else(|_| unreachable!("fixture coordinate is valid"));
        let active = TextPosition::new(snapshot(3), source, 0, TextAffinity::Downstream)
            .unwrap_or_else(|_| unreachable!("fixture coordinate is valid"));
        let selection = TextSelection::new(anchor, active)
            .unwrap_or_else(|_| unreachable!("fixture snapshots match"));
        assert_eq!(selection.direction(), TextSelectionDirection::Backward);
        assert_eq!(selection.anchor().affinity(), TextAffinity::Upstream);
        assert_eq!(
            TextSelection::new(
                anchor,
                TextPosition::new(snapshot(4), source, 0, TextAffinity::Downstream)
                    .unwrap_or_else(|_| unreachable!("fixture coordinate is valid"))
            ),
            Err(TextSelectionSnapshotMismatch)
        );
    }
}
