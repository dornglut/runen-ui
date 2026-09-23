//! Immutable caret, selection, and navigation mapping over one retained layout.

use core::{error::Error, fmt};
use std::{collections::HashSet, sync::Arc};

use parley::{
    editing::{Cursor, Selection},
    layout::Affinity,
};
use runenui_core::{
    LogicalLength, LogicalPoint, LogicalRect, LogicalTransform, TextAffinity, TextDisplayPosition,
    TextDocumentSnapshot, TextPosition, TextPositionError, TextSelection,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    TextArtifact, TextPreeditProjection, TextPreeditProjectionError, layout_state::CachedTextLayout,
};

/// Failure while using an exact retained-layout caret map.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextCaretMapError {
    MissingLayout,
    SnapshotMismatch,
    DisplayTextMismatch,
    ForeignComposition,
    OutOfBounds,
    NotScalarBoundary,
    NotCaretStop,
    InvalidAffinity,
    HiddenDocumentPosition,
    UnsupportedInlineBoxes,
    NonInvertibleTransform,
    TransformOverflow,
    InvalidGeometry,
}

impl fmt::Display for TextCaretMapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingLayout => "text layout state has no retained layout",
            Self::SnapshotMismatch => "text coordinate addresses a different document revision",
            Self::DisplayTextMismatch => {
                "preedit display text does not match the retained text layout"
            }
            Self::ForeignComposition => "text coordinate belongs to another composition",
            Self::OutOfBounds => "text coordinate is outside the retained display",
            Self::NotScalarBoundary => "text coordinate splits a Unicode scalar",
            Self::NotCaretStop => "text coordinate is not a shaping-valid caret stop",
            Self::InvalidAffinity => "text affinity is not valid at this caret stop",
            Self::HiddenDocumentPosition => {
                "document position is hidden by the retained preedit projection"
            }
            Self::UnsupportedInlineBoxes => {
                "editable caret mapping does not support inline layout boxes"
            }
            Self::NonInvertibleTransform => "displayed text transform is not invertible",
            Self::TransformOverflow => "displayed text inverse transform overflowed",
            Self::InvalidGeometry => "text layout produced invalid caret geometry",
        })
    }
}

impl Error for TextCaretMapError {}

impl From<TextPreeditProjectionError> for TextCaretMapError {
    fn from(error: TextPreeditProjectionError) -> Self {
        match error {
            TextPreeditProjectionError::SnapshotMismatch => Self::SnapshotMismatch,
            TextPreeditProjectionError::ForeignComposition => Self::ForeignComposition,
            TextPreeditProjectionError::InvalidBoundaryAffinity => Self::InvalidAffinity,
            TextPreeditProjectionError::HiddenDocumentPosition => Self::HiddenDocumentPosition,
            TextPreeditProjectionError::DisplayOffsetOutOfBounds => Self::OutOfBounds,
            TextPreeditProjectionError::InvalidPosition(TextPositionError::OutOfBounds) => {
                Self::OutOfBounds
            }
            TextPreeditProjectionError::DisplayOffsetNotScalarBoundary
            | TextPreeditProjectionError::InvalidPosition(_) => Self::NotScalarBoundary,
            TextPreeditProjectionError::InvalidPreeditSelection
            | TextPreeditProjectionError::DisplayLengthOverflow => Self::DisplayTextMismatch,
        }
    }
}

/// Directional selection in one staged display coordinate space.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextDisplaySelection {
    anchor: TextDisplayPosition,
    active: TextDisplayPosition,
}

impl TextDisplaySelection {
    #[must_use]
    pub const fn new(anchor: TextDisplayPosition, active: TextDisplayPosition) -> Self {
        Self { anchor, active }
    }

    #[must_use]
    pub const fn from_document(selection: TextSelection) -> Self {
        Self::new(
            TextDisplayPosition::Document(selection.anchor()),
            TextDisplayPosition::Document(selection.active()),
        )
    }

    #[must_use]
    pub const fn anchor(&self) -> &TextDisplayPosition {
        &self.anchor
    }

    #[must_use]
    pub const fn active(&self) -> &TextDisplayPosition {
        &self.active
    }
}

/// One exact selection rectangle and its retained layout line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectionRect {
    rect: LogicalRect,
    line_index: usize,
}

impl TextSelectionRect {
    #[must_use]
    pub const fn rect(self) -> LogicalRect {
        self.rect
    }

    #[must_use]
    pub const fn line_index(self) -> usize {
        self.line_index
    }
}

/// Host-neutral navigation operation over shaping-valid caret stops.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextNavigation {
    PreviousLogical,
    NextLogical,
    PreviousVisual,
    NextVisual,
    PreviousLogicalWord,
    NextLogicalWord,
    PreviousVisualWord,
    NextVisualWord,
    PreviousLine,
    NextLine,
    LineStart,
    LineEnd,
    HardLineStart,
    HardLineEnd,
}

/// Whether navigation collapses or extends the existing selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextNavigationMode {
    Move,
    Extend,
}

/// Finite retained inline coordinate used to preserve vertical movement intent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextPreferredInline(f32);

impl TextPreferredInline {
    /// Creates a finite preferred inline coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`TextCaretMapError::InvalidGeometry`] for a non-finite value.
    pub fn new(value: f32) -> Result<Self, TextCaretMapError> {
        value
            .is_finite()
            .then_some(Self(if value == 0.0 { 0.0 } else { value }))
            .ok_or(TextCaretMapError::InvalidGeometry)
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Result of one navigation step plus vertical inline-position continuity.
#[derive(Clone, Debug, PartialEq)]
pub struct TextNavigationResult {
    selection: TextDisplaySelection,
    preferred_inline: Option<TextPreferredInline>,
}

impl TextNavigationResult {
    #[must_use]
    pub const fn selection(&self) -> &TextDisplaySelection {
        &self.selection
    }

    #[must_use]
    pub const fn preferred_inline(&self) -> Option<TextPreferredInline> {
        self.preferred_inline
    }
}

#[derive(Clone)]
enum CoordinateSpace {
    Document(TextDocumentSnapshot),
    Preedit(Arc<TextPreeditProjection>),
}

/// Immutable mapping derived from the exact retained layout and artifact used
/// for measurement and paint.
///
/// Clones retain that layout lineage. A later width-only re-linebreak uses
/// copy-on-write and cannot mutate an already issued map.
#[derive(Clone)]
pub struct TextCaretMap {
    cached: Arc<CachedTextLayout>,
    coordinates: CoordinateSpace,
    grapheme_boundaries: Arc<[usize]>,
}

impl TextCaretMap {
    pub(super) fn document(
        cached: Arc<CachedTextLayout>,
        snapshot: TextDocumentSnapshot,
    ) -> Result<Self, TextCaretMapError> {
        Self::validate_layout(&cached)?;
        #[cfg(any(test, feature = "internal-test-seams"))
        let profile_started = std::time::Instant::now();
        let grapheme_boundaries = grapheme_boundaries(cached.request.text());
        #[cfg(any(test, feature = "internal-test-seams"))
        crate::test_profile::record_graphemes(profile_started.elapsed(), grapheme_boundaries.len());
        Ok(Self {
            grapheme_boundaries,
            cached,
            coordinates: CoordinateSpace::Document(snapshot),
        })
    }

    pub(super) fn preedit(
        cached: Arc<CachedTextLayout>,
        projection: Arc<TextPreeditProjection>,
    ) -> Result<Self, TextCaretMapError> {
        Self::validate_layout(&cached)?;
        if cached.request.text() != projection.display_text() {
            return Err(TextCaretMapError::DisplayTextMismatch);
        }
        #[cfg(any(test, feature = "internal-test-seams"))
        let profile_started = std::time::Instant::now();
        let grapheme_boundaries = grapheme_boundaries(cached.request.text());
        #[cfg(any(test, feature = "internal-test-seams"))
        crate::test_profile::record_graphemes(profile_started.elapsed(), grapheme_boundaries.len());
        Ok(Self {
            grapheme_boundaries,
            cached,
            coordinates: CoordinateSpace::Preedit(projection),
        })
    }

    fn validate_layout(cached: &CachedTextLayout) -> Result<(), TextCaretMapError> {
        if cached.layout.inline_boxes().is_empty() {
            Ok(())
        } else {
            Err(TextCaretMapError::UnsupportedInlineBoxes)
        }
    }

    /// Returns the exact document revision addressed by this map.
    #[must_use]
    pub fn snapshot(&self) -> TextDocumentSnapshot {
        match &self.coordinates {
            CoordinateSpace::Document(snapshot) => *snapshot,
            CoordinateSpace::Preedit(projection) => projection.snapshot(),
        }
    }

    /// Returns the exact correlated measurement/paint artifact.
    #[must_use]
    pub fn artifact(&self) -> &TextArtifact {
        &self.cached.artifact
    }

    /// Returns whether `artifact` is the exact immutable measurement/paint
    /// artifact from which this map was derived.
    #[must_use]
    pub fn is_correlated_with(&self, artifact: &TextArtifact) -> bool {
        self.cached.artifact.shares_layout_with(artifact)
    }

    /// Returns whether two maps retain the exact same private layout lineage.
    #[must_use]
    pub fn shares_layout_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cached, &other.cached)
    }

    /// Returns the exact retained display text addressed by this map.
    #[must_use]
    pub fn display_text(&self) -> &str {
        self.cached.request.text()
    }

    /// Returns the transient projection when this map addresses staged preedit.
    #[must_use]
    pub fn preedit_projection(&self) -> Option<&TextPreeditProjection> {
        match &self.coordinates {
            CoordinateSpace::Document(_) => None,
            CoordinateSpace::Preedit(projection) => Some(projection),
        }
    }

    /// Returns the exact checked preedit-relative selection in this map's display space.
    ///
    /// # Errors
    ///
    /// Returns an error when the retained preedit selection is not a grapheme- and
    /// shaping-valid selection in the correlated display layout.
    pub fn preedit_selection(&self) -> Result<Option<TextDisplaySelection>, TextCaretMapError> {
        let CoordinateSpace::Preedit(projection) = &self.coordinates else {
            return Ok(None);
        };
        let Some(range) = projection.selection() else {
            return Ok(None);
        };
        if projection.preedit().is_empty() {
            let cursor = Cursor::from_byte_index(
                &self.cached.layout,
                projection.display_preedit_start(),
                Affinity::Downstream,
            );
            let position = self.position_for_cursor(cursor)?;
            return Ok(Some(TextDisplaySelection::new(position.clone(), position)));
        }
        let collapsed_affinity = if range.start() == projection.preedit().len() {
            TextAffinity::Upstream
        } else {
            TextAffinity::Downstream
        };
        let (anchor_affinity, active_affinity) = if range.start() == range.end() {
            (collapsed_affinity, collapsed_affinity)
        } else {
            (TextAffinity::Downstream, TextAffinity::Upstream)
        };
        let anchor = TextDisplayPosition::Preedit(
            runenui_core::TextPreeditPosition::new(
                projection.snapshot(),
                projection.generation().clone(),
                projection.preedit(),
                range.start(),
                anchor_affinity,
            )
            .map_err(TextPreeditProjectionError::from)?,
        );
        let active = TextDisplayPosition::Preedit(
            runenui_core::TextPreeditPosition::new(
                projection.snapshot(),
                projection.generation().clone(),
                projection.preedit(),
                range.end(),
                active_affinity,
            )
            .map_err(TextPreeditProjectionError::from)?,
        );
        self.cursor_for_position(&anchor)?;
        self.cursor_for_position(&active)?;
        Ok(Some(TextDisplaySelection::new(anchor, active)))
    }

    /// Validates scalar, snapshot, composition, grapheme and shaping caret facts.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the position is stale, foreign, out of
    /// bounds, hidden by preedit, or not a legal caret stop.
    pub fn validate_position(
        &self,
        position: &TextDisplayPosition,
    ) -> Result<(), TextCaretMapError> {
        self.cursor_for_position(position).map(|_| ())
    }

    /// Returns every legal leading/trailing stop in logical byte order.
    #[must_use]
    pub fn legal_positions(&self) -> Vec<TextDisplayPosition> {
        let mut positions = Vec::new();
        let mut seen = HashSet::new();
        for &byte_offset in self.grapheme_boundaries.iter() {
            for affinity in [TextAffinity::Upstream, TextAffinity::Downstream] {
                if let Ok(cursor) = self.cursor_at(byte_offset, affinity)
                    && let Ok(position) = self.position_for_cursor(cursor)
                    && seen.insert(position.clone())
                {
                    positions.push(position);
                }
            }
        }
        positions
    }

    /// Returns the ordered UTF-8 offsets that have at least one shaping-valid caret affinity.
    ///
    /// This is the compact projection needed by semantic text ranges. Unlike
    /// [`Self::legal_positions`], it does not allocate a public position for each affinity or
    /// retain duplicate offsets when both affinities are legal at one boundary.
    #[must_use]
    #[allow(
        clippy::let_and_return,
        reason = "test-only profiling observes collected offsets before returning them"
    )]
    pub fn legal_byte_offsets(&self) -> Vec<usize> {
        #[cfg(any(test, feature = "internal-test-seams"))
        let profile_started = std::time::Instant::now();
        let offsets = self
            .grapheme_boundaries
            .iter()
            .copied()
            .filter(|&byte_offset| {
                [TextAffinity::Upstream, TextAffinity::Downstream]
                    .into_iter()
                    .any(|affinity| self.cursor_at(byte_offset, affinity).is_ok())
            })
            .collect::<Vec<_>>();
        #[cfg(any(test, feature = "internal-test-seams"))
        crate::test_profile::record_legal_offsets(profile_started.elapsed(), offsets.len());
        offsets
    }

    /// Converts a displayed surface point into a shaping-valid position.
    ///
    /// `displayed_snapshot` must identify the exact published document revision.
    /// `eligible_surface_bounds` is checked before the exact published transform
    /// is inverted and applied. A singular/overflowing transform fails closed;
    /// Parley's nearest-cluster behavior never receives an ineligible point.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale snapshot, a non-invertible or overflowing
    /// transform, or an invalid retained layout result.
    pub fn hit_test(
        &self,
        displayed_snapshot: TextDocumentSnapshot,
        surface_point: LogicalPoint,
        eligible_surface_bounds: LogicalRect,
        layout_to_surface: LogicalTransform,
    ) -> Result<Option<TextDisplayPosition>, TextCaretMapError> {
        if displayed_snapshot != self.snapshot() {
            return Err(TextCaretMapError::SnapshotMismatch);
        }
        if !eligible_surface_bounds.contains(surface_point) {
            return Ok(None);
        }
        let surface_to_layout = layout_to_surface
            .inverse()
            .ok_or(TextCaretMapError::NonInvertibleTransform)?;
        let local_point = surface_to_layout
            .transform_point(surface_point)
            .ok_or(TextCaretMapError::TransformOverflow)?;
        let cursor = Cursor::from_point(&self.cached.layout, local_point.x(), local_point.y());
        let canonical =
            Cursor::from_byte_index(&self.cached.layout, cursor.index(), cursor.affinity());
        self.position_for_cursor(canonical).map(Some)
    }

    /// Maps a point through the exact displayed transform to the nearest legal
    /// caret in this retained layout, without applying initial-hit eligibility.
    ///
    /// This is intended for an already admitted and captured text-selection
    /// gesture. Initial pointer admission must continue to use [`Self::hit_test`]
    /// so viewport bounds and publication clipping remain authoritative.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale snapshot, a non-invertible or overflowing
    /// transform, or an invalid retained-layout result.
    pub fn nearest_position(
        &self,
        displayed_snapshot: TextDocumentSnapshot,
        surface_point: LogicalPoint,
        layout_to_surface: LogicalTransform,
    ) -> Result<TextDisplayPosition, TextCaretMapError> {
        if displayed_snapshot != self.snapshot() {
            return Err(TextCaretMapError::SnapshotMismatch);
        }
        let surface_to_layout = layout_to_surface
            .inverse()
            .ok_or(TextCaretMapError::NonInvertibleTransform)?;
        let local_point = surface_to_layout
            .transform_point(surface_point)
            .ok_or(TextCaretMapError::TransformOverflow)?;
        let cursor = Cursor::from_point(&self.cached.layout, local_point.x(), local_point.y());
        let canonical =
            Cursor::from_byte_index(&self.cached.layout, cursor.index(), cursor.affinity());
        self.position_for_cursor(canonical)
    }

    /// Returns logical caret geometry for one legal display position.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid position or unrepresentable geometry.
    pub fn caret_rect(
        &self,
        position: &TextDisplayPosition,
        width: LogicalLength,
    ) -> Result<LogicalRect, TextCaretMapError> {
        let cursor = self.cursor_for_position(position)?;
        Self::rect_from_box(cursor.geometry(&self.cached.layout, width.get()))
    }

    /// Returns zero-width candidate geometry for the active IME caret.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid position or unrepresentable geometry.
    pub fn candidate_rect(
        &self,
        position: &TextDisplayPosition,
    ) -> Result<LogicalRect, TextCaretMapError> {
        self.caret_rect(position, LogicalLength::ZERO)
    }

    /// Derives RunenUI-defined selection geometry from visual clusters.
    ///
    /// Explicit newlines with zero advance receive no guessed whitespace width;
    /// bidi-discontiguous visual segments remain separate rectangles.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid endpoints or unrepresentable geometry.
    pub fn selection_rects(
        &self,
        selection: &TextDisplaySelection,
    ) -> Result<Vec<TextSelectionRect>, TextCaretMapError> {
        let anchor = self.cursor_for_position(selection.anchor())?;
        let active = self.cursor_for_position(selection.active())?;
        let start = anchor.index().min(active.index());
        let end = anchor.index().max(active.index());
        if start == end {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        for (line_index, line) in self.cached.layout.lines().enumerate() {
            let metrics = line.metrics();
            let mut x = metrics.offset + metrics.inline_min_coord;
            let mut segment_start = None;
            for run in line.runs() {
                for cluster in run.visual_clusters() {
                    let cluster_range = cluster.text_range();
                    let selected = cluster_range.start >= start && cluster_range.end <= end;
                    if selected && segment_start.is_none() {
                        segment_start = Some(x);
                    } else if !selected {
                        Self::flush_selection_segment(
                            &mut result,
                            &mut segment_start,
                            x,
                            metrics.block_min_coord,
                            metrics.block_max_coord,
                            line_index,
                        )?;
                    }
                    x += cluster.advance();
                }
            }
            Self::flush_selection_segment(
                &mut result,
                &mut segment_start,
                x,
                metrics.block_min_coord,
                metrics.block_max_coord,
                line_index,
            )?;
        }
        Ok(result)
    }

    /// Returns whether both endpoints resolve to the same byte in this exact display map.
    ///
    /// # Errors
    ///
    /// Returns an error for stale, foreign, hidden, or invalid endpoints.
    pub fn is_selection_collapsed(
        &self,
        selection: &TextDisplaySelection,
    ) -> Result<bool, TextCaretMapError> {
        Ok(self.cursor_for_position(selection.anchor())?.index()
            == self.cursor_for_position(selection.active())?.index())
    }

    /// Applies one logical/visual/word/line operation to an exact display selection.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid endpoints, stale projection facts, or invalid
    /// vertical-navigation geometry.
    pub fn navigate(
        &self,
        selection: &TextDisplaySelection,
        operation: TextNavigation,
        mode: TextNavigationMode,
        preferred_inline: Option<TextPreferredInline>,
    ) -> Result<TextNavigationResult, TextCaretMapError> {
        let parley = self.parley_selection(selection)?;
        let extend = mode == TextNavigationMode::Extend;
        let (next, next_preferred) = match operation {
            TextNavigation::PreviousVisual => {
                (parley.previous_visual(&self.cached.layout, extend), None)
            }
            TextNavigation::NextVisual => (parley.next_visual(&self.cached.layout, extend), None),
            TextNavigation::PreviousVisualWord => (
                parley.previous_visual_word(&self.cached.layout, extend),
                None,
            ),
            TextNavigation::NextVisualWord => {
                (parley.next_visual_word(&self.cached.layout, extend), None)
            }
            TextNavigation::LineStart => (parley.line_start(&self.cached.layout, extend), None),
            TextNavigation::LineEnd => (parley.line_end(&self.cached.layout, extend), None),
            TextNavigation::HardLineStart => {
                (parley.hard_line_start(&self.cached.layout, extend), None)
            }
            TextNavigation::HardLineEnd => {
                (parley.hard_line_end(&self.cached.layout, extend), None)
            }
            TextNavigation::PreviousLogical => (self.move_logical(parley, false, extend), None),
            TextNavigation::NextLogical => (self.move_logical(parley, true, extend), None),
            TextNavigation::PreviousLogicalWord => {
                (self.move_logical_word(parley, false, extend), None)
            }
            TextNavigation::NextLogicalWord => (self.move_logical_word(parley, true, extend), None),
            TextNavigation::PreviousLine => self.move_line(parley, -1, extend, preferred_inline)?,
            TextNavigation::NextLine => self.move_line(parley, 1, extend, preferred_inline)?,
        };
        Ok(TextNavigationResult {
            selection: self.selection_from_parley(next)?,
            preferred_inline: next_preferred,
        })
    }

    fn display_offset(&self, position: &TextDisplayPosition) -> Result<usize, TextCaretMapError> {
        if position.snapshot() != self.snapshot() {
            return Err(TextCaretMapError::SnapshotMismatch);
        }
        match &self.coordinates {
            CoordinateSpace::Document(_) => match position {
                TextDisplayPosition::Document(position) => Ok(position.byte_offset()),
                TextDisplayPosition::Preedit(_) => Err(TextCaretMapError::ForeignComposition),
            },
            CoordinateSpace::Preedit(projection) => projection
                .display_offset_for_position(position)
                .map_err(Into::into),
        }
    }

    fn position_for_cursor(
        &self,
        cursor: Cursor,
    ) -> Result<TextDisplayPosition, TextCaretMapError> {
        let cursor = self.canonical_caret_cursor(cursor)?;
        let affinity = affinity_from_parley(cursor.affinity());
        match &self.coordinates {
            CoordinateSpace::Document(snapshot) => Ok(TextDisplayPosition::Document(
                TextPosition::new(*snapshot, self.display_text(), cursor.index(), affinity)
                    .map_err(|_| TextCaretMapError::NotScalarBoundary)?,
            )),
            CoordinateSpace::Preedit(projection) => projection
                .position_from_display_offset(cursor.index(), affinity)
                .map_err(Into::into),
        }
    }

    fn cursor_for_position(
        &self,
        position: &TextDisplayPosition,
    ) -> Result<Cursor, TextCaretMapError> {
        let offset = self.display_offset(position)?;
        let cursor = self.cursor_at(offset, position.affinity())?;
        if self.position_for_cursor(cursor)? != *position {
            return Err(TextCaretMapError::InvalidAffinity);
        }
        Ok(cursor)
    }

    fn cursor_at(
        &self,
        byte_offset: usize,
        affinity: TextAffinity,
    ) -> Result<Cursor, TextCaretMapError> {
        let source = self.display_text();
        if byte_offset > source.len() {
            return Err(TextCaretMapError::OutOfBounds);
        }
        if !source.is_char_boundary(byte_offset) {
            return Err(TextCaretMapError::NotScalarBoundary);
        }
        if self
            .grapheme_boundaries
            .binary_search(&byte_offset)
            .is_err()
        {
            return Err(TextCaretMapError::NotCaretStop);
        }
        let cursor = Cursor::from_byte_index(
            &self.cached.layout,
            byte_offset,
            affinity_to_parley(affinity),
        );
        if cursor.index() != byte_offset {
            return Err(TextCaretMapError::NotCaretStop);
        }
        if affinity_from_parley(cursor.affinity()) != affinity {
            return Err(TextCaretMapError::InvalidAffinity);
        }
        Ok(cursor)
    }

    fn canonical_caret_cursor(&self, cursor: Cursor) -> Result<Cursor, TextCaretMapError> {
        let affinity = affinity_from_parley(cursor.affinity());
        if let Ok(cursor) = self.cursor_at(cursor.index(), affinity) {
            return Ok(cursor);
        }
        if cursor.index() > self.display_text().len()
            || !self.display_text().is_char_boundary(cursor.index())
        {
            return Err(TextCaretMapError::NotScalarBoundary);
        }
        let boundary_index = self
            .grapheme_boundaries
            .partition_point(|boundary| *boundary < cursor.index());
        let byte_offset = match affinity {
            TextAffinity::Downstream => boundary_index
                .checked_sub(1)
                .and_then(|index| self.grapheme_boundaries.get(index))
                .copied(),
            TextAffinity::Upstream => self.grapheme_boundaries.get(boundary_index).copied(),
        }
        .ok_or(TextCaretMapError::NotCaretStop)?;
        self.cursor_at(byte_offset, affinity)
    }

    fn parley_selection(
        &self,
        selection: &TextDisplaySelection,
    ) -> Result<Selection, TextCaretMapError> {
        Ok(Selection::new(
            self.cursor_for_position(selection.anchor())?,
            self.cursor_for_position(selection.active())?,
        ))
    }

    fn selection_from_parley(
        &self,
        selection: Selection,
    ) -> Result<TextDisplaySelection, TextCaretMapError> {
        Ok(TextDisplaySelection::new(
            self.position_for_cursor(selection.anchor())?,
            self.position_for_cursor(selection.focus())?,
        ))
    }

    fn move_logical(&self, selection: Selection, next: bool, extend: bool) -> Selection {
        let focus = if !selection.is_collapsed() && !extend {
            let anchor = selection.anchor();
            let focus = selection.focus();
            if next {
                if anchor.index() >= focus.index() {
                    anchor
                } else {
                    focus
                }
            } else if anchor.index() <= focus.index() {
                anchor
            } else {
                focus
            }
        } else {
            let focus = selection.focus();
            let [upstream, downstream] = focus.logical_clusters(&self.cached.layout);
            if next {
                downstream.map_or(focus, |cluster| {
                    Cursor::from_byte_index(
                        &self.cached.layout,
                        cluster.text_range().end,
                        Affinity::Upstream,
                    )
                })
            } else {
                upstream.map_or(focus, |cluster| {
                    Cursor::from_byte_index(
                        &self.cached.layout,
                        cluster.text_range().start,
                        Affinity::Downstream,
                    )
                })
            }
        };
        if extend {
            Selection::new(selection.anchor(), focus)
        } else {
            Selection::new(focus, focus)
        }
    }

    fn move_logical_word(&self, selection: Selection, next: bool, extend: bool) -> Selection {
        let focus = if next {
            selection.focus().next_logical_word(&self.cached.layout)
        } else {
            selection.focus().previous_logical_word(&self.cached.layout)
        };
        if extend {
            Selection::new(selection.anchor(), focus)
        } else {
            Selection::new(focus, focus)
        }
    }

    fn move_line(
        &self,
        selection: Selection,
        delta: isize,
        extend: bool,
        preferred_inline: Option<TextPreferredInline>,
    ) -> Result<(Selection, Option<TextPreferredInline>), TextCaretMapError> {
        let focus_rect = selection.focus().geometry(&self.cached.layout, 0.0);
        let current_line = self
            .cached
            .layout
            .lines()
            .position(|line| {
                let metrics = line.metrics();
                (f64::from(metrics.block_min_coord)..f64::from(metrics.block_max_coord))
                    .contains(&focus_rect.y0)
            })
            .unwrap_or_else(|| self.cached.layout.len().saturating_sub(1));
        let target_index = current_line.saturating_add_signed(delta);
        let target_index = target_index.min(self.cached.layout.len().saturating_sub(1));
        let Some(target_line) = self.cached.layout.get(target_index) else {
            return Ok((selection, preferred_inline));
        };
        let default_inline = f64_to_f32(focus_rect.x0)?;
        let inline = preferred_inline.map_or(default_inline, TextPreferredInline::get);
        let preferred = TextPreferredInline::new(inline)?;
        if delta < 0 && current_line == 0 {
            return Ok((
                selection.line_start(&self.cached.layout, extend),
                Some(preferred),
            ));
        }
        if delta > 0 && current_line + 1 >= self.cached.layout.len() {
            return Ok((
                selection.line_end(&self.cached.layout, extend),
                Some(preferred),
            ));
        }
        let metrics = target_line.metrics();
        let y = f32::midpoint(metrics.block_min_coord, metrics.block_max_coord);
        let focus = Cursor::from_point(&self.cached.layout, inline, y);
        let next = if extend {
            Selection::new(selection.anchor(), focus)
        } else {
            Selection::new(focus, focus)
        };
        Ok((next, Some(preferred)))
    }

    fn flush_selection_segment(
        result: &mut Vec<TextSelectionRect>,
        segment_start: &mut Option<f32>,
        end: f32,
        block_min: f32,
        block_max: f32,
        line_index: usize,
    ) -> Result<(), TextCaretMapError> {
        let Some(start) = segment_start.take() else {
            return Ok(());
        };
        if end <= start {
            return Ok(());
        }
        let rect = LogicalRect::try_new(start, block_min, end - start, block_max - block_min)
            .map_err(|_| TextCaretMapError::InvalidGeometry)?;
        result.push(TextSelectionRect { rect, line_index });
        Ok(())
    }

    fn rect_from_box(bounds: parley::BoundingBox) -> Result<LogicalRect, TextCaretMapError> {
        let [x0, y0, x1, y1] = [bounds.x0, bounds.y0, bounds.x1, bounds.y1];
        if ![x0, y0, x1, y1].iter().all(|value| value.is_finite())
            || x1 < x0
            || y1 < y0
            || x0 < f64::from(f32::MIN)
            || x0 > f64::from(f32::MAX)
            || y0 < f64::from(f32::MIN)
            || y0 > f64::from(f32::MAX)
            || x1 - x0 > f64::from(f32::MAX)
            || y1 - y0 > f64::from(f32::MAX)
        {
            return Err(TextCaretMapError::InvalidGeometry);
        }
        LogicalRect::try_new(
            f64_to_f32(x0)?,
            f64_to_f32(y0)?,
            f64_to_f32(x1 - x0)?,
            f64_to_f32(y1 - y0)?,
        )
        .map_err(|_| TextCaretMapError::InvalidGeometry)
    }
}

impl fmt::Debug for TextCaretMap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextCaretMap")
            .field("snapshot", &self.snapshot())
            .field("has_preedit", &self.preedit_projection().is_some())
            .field("display_len", &self.display_text().len())
            .finish_non_exhaustive()
    }
}

const fn affinity_to_parley(affinity: TextAffinity) -> Affinity {
    match affinity {
        TextAffinity::Upstream => Affinity::Upstream,
        TextAffinity::Downstream => Affinity::Downstream,
    }
}

const fn affinity_from_parley(affinity: Affinity) -> TextAffinity {
    match affinity {
        Affinity::Upstream => TextAffinity::Upstream,
        Affinity::Downstream => TextAffinity::Downstream,
    }
}

fn grapheme_boundaries(source: &str) -> Arc<[usize]> {
    source
        .grapheme_indices(true)
        .map(|(offset, _)| offset)
        .chain(core::iter::once(source.len()))
        .collect::<Vec<_>>()
        .into()
}

#[allow(clippy::cast_possible_truncation)]
fn f64_to_f32(value: f64) -> Result<f32, TextCaretMapError> {
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        Err(TextCaretMapError::InvalidGeometry)
    } else {
        Ok(value as f32)
    }
}
