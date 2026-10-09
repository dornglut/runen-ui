//! Immutable secret-display coordinate projection over the retained M8 text layout.
//!
//! This value is derived from an application-owned document or an M10 transient
//! preedit. It is never a document, editor, composition or shaping authority.

use core::fmt;
use std::sync::Arc;

use runenui_core::{
    LogicalLength, LogicalPoint, LogicalRect, LogicalTransform, TextAffinity, TextDisplayPosition,
    TextDocumentSnapshot, TextPosition,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    TextCaretMap, TextCaretMapError, TextDisplaySelection, TextLayoutState, TextNavigation,
    TextNavigationMode, TextPreeditProjection, TextPreeditProjectionError, TextPreferredInline,
    TextSelectionRect,
};

/// Projection rejection contains no source, replacement or preedit payload.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextMaskedProjectionError {
    LengthOverflow,
    AllocationFailed,
    ForeignSnapshot,
    ForeignComposition,
    OutOfBounds,
    NotGraphemeBoundary,
    InvalidCoordinate,
    RetainedLayout(TextCaretMapError),
}

impl fmt::Display for TextMaskedProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LengthOverflow => "masked display length overflow",
            Self::AllocationFailed => "masked display allocation failed",
            Self::ForeignSnapshot => "masked display snapshot mismatch",
            Self::ForeignComposition => "masked display composition mismatch",
            Self::OutOfBounds => "masked display coordinate outside source",
            Self::NotGraphemeBoundary => "masked display coordinate splits a grapheme",
            Self::InvalidCoordinate => "masked display coordinate invalid",
            Self::RetainedLayout(_) => "masked display retained-layout mismatch",
        })
    }
}

impl std::error::Error for TextMaskedProjectionError {}

impl From<TextCaretMapError> for TextMaskedProjectionError {
    fn from(error: TextCaretMapError) -> Self {
        Self::RetainedLayout(error)
    }
}

#[derive(Clone)]
enum SourceSpace {
    Document {
        snapshot: TextDocumentSnapshot,
        source: Arc<str>,
    },
    Preedit(Arc<TextPreeditProjection>),
}

/// One grapheme-safe, source-coordinate ↔ masked-display mapping.
///
/// Masked glyphs, never source/preedit literals, are supplied to M8 shaping.
/// The source is retained privately only to validate revision-scoped positions.
/// Debug and errors intentionally disclose no document, preedit, or payload.
#[derive(Clone)]
pub struct TextMaskedProjection {
    source: SourceSpace,
    display: Arc<str>,
    // Sorted pairs of source byte and masked display byte grapheme boundaries.
    boundaries: Arc<[(usize, usize)]>,
}

impl TextMaskedProjection {
    /// Masks one exact application document as one bullet per extended grapheme.
    ///
    /// # Errors
    ///
    /// Rejects length/capacity overflow or failed allocations; no partial
    /// projection is returned.
    pub fn document(
        snapshot: TextDocumentSnapshot,
        source: impl Into<Arc<str>>,
    ) -> Result<Self, TextMaskedProjectionError> {
        Self::build(SourceSpace::Document {
            snapshot,
            source: source.into(),
        })
    }

    /// Masks the *entire* existing M10 preedit display, including composing text.
    /// Durable and synthetic coordinates are still distinguished by M10.
    ///
    /// # Errors
    ///
    /// Rejects length/capacity overflow or failed allocations.
    pub fn preedit(
        projection: Arc<TextPreeditProjection>,
    ) -> Result<Self, TextMaskedProjectionError> {
        Self::build(SourceSpace::Preedit(projection))
    }

    fn build(source: SourceSpace) -> Result<Self, TextMaskedProjectionError> {
        let plaintext = match &source {
            SourceSpace::Document { source, .. } => source.as_ref(),
            SourceSpace::Preedit(projection) => projection.display_text(),
        };
        let count = plaintext.graphemes(true).count();
        let capacity = count
            .checked_mul('•'.len_utf8())
            .ok_or(TextMaskedProjectionError::LengthOverflow)?;
        let boundary_capacity = count
            .checked_add(1)
            .ok_or(TextMaskedProjectionError::LengthOverflow)?;
        let mut display = String::new();
        display
            .try_reserve(capacity)
            .map_err(|_| TextMaskedProjectionError::AllocationFailed)?;
        let mut boundaries = Vec::new();
        boundaries
            .try_reserve(boundary_capacity)
            .map_err(|_| TextMaskedProjectionError::AllocationFailed)?;
        for (source_offset, _) in plaintext.grapheme_indices(true) {
            boundaries.push((source_offset, display.len()));
            display.push('•');
        }
        boundaries.push((plaintext.len(), display.len()));
        Ok(Self {
            source,
            display: Arc::from(display),
            boundaries: boundaries.into(),
        })
    }

    #[must_use]
    pub fn snapshot(&self) -> TextDocumentSnapshot {
        match &self.source {
            SourceSpace::Document { snapshot, .. } => *snapshot,
            SourceSpace::Preedit(projection) => projection.snapshot(),
        }
    }

    /// The sole text intended for shaping, shaped resources and paint.
    /// It contains no source or preedit character payload.
    #[must_use]
    pub fn display_text(&self) -> &str {
        &self.display
    }

    fn unmasked_source(&self) -> &str {
        match &self.source {
            SourceSpace::Document { source, .. } => source,
            SourceSpace::Preedit(projection) => projection.display_text(),
        }
    }

    /// Converts a checked M10 document/preedit position to an exact masked
    /// grapheme stop, rejecting foreign generations and interior offsets.
    ///
    /// # Errors
    ///
    /// Rejects invalid, foreign, or non-grapheme coordinates.
    pub fn display_offset_for_position(
        &self,
        position: &TextDisplayPosition,
    ) -> Result<usize, TextMaskedProjectionError> {
        if position.snapshot() != self.snapshot() {
            return Err(TextMaskedProjectionError::ForeignSnapshot);
        }
        let offset = match (&self.source, position) {
            (SourceSpace::Document { .. }, TextDisplayPosition::Document(pos)) => pos.byte_offset(),
            (SourceSpace::Document { .. }, TextDisplayPosition::Preedit(_)) => {
                return Err(TextMaskedProjectionError::ForeignComposition);
            }
            (SourceSpace::Preedit(projection), _) => projection
                .display_offset_for_position(position)
                .map_err(|_| TextMaskedProjectionError::ForeignComposition)?,
        };
        self.source_offset_to_display(offset)
    }

    fn source_offset_to_display(&self, offset: usize) -> Result<usize, TextMaskedProjectionError> {
        if offset > self.unmasked_source().len() {
            return Err(TextMaskedProjectionError::OutOfBounds);
        }
        self.boundaries
            .binary_search_by_key(&offset, |(source, _)| *source)
            .map(|index| self.boundaries[index].1)
            .map_err(|_| TextMaskedProjectionError::NotGraphemeBoundary)
    }

    /// Maps a retained masked glyph boundary back to M10's exact durable or
    /// synthetic preedit coordinate without losing selection direction/affinity.
    ///
    /// # Errors
    ///
    /// Rejects offsets that are not exact masked grapheme stops.
    pub fn position_from_display_offset(
        &self,
        offset: usize,
        affinity: TextAffinity,
    ) -> Result<TextDisplayPosition, TextMaskedProjectionError> {
        if offset > self.display.len() {
            return Err(TextMaskedProjectionError::OutOfBounds);
        }
        let index = self
            .boundaries
            .binary_search_by_key(&offset, |(_, display)| *display)
            .map_err(|_| TextMaskedProjectionError::NotGraphemeBoundary)?;
        let source_offset = self.boundaries[index].0;
        match &self.source {
            SourceSpace::Document { snapshot, source } => Ok(TextDisplayPosition::Document(
                TextPosition::new(*snapshot, source, source_offset, affinity)
                    .map_err(|_| TextMaskedProjectionError::InvalidCoordinate)?,
            )),
            SourceSpace::Preedit(projection) => projection
                .position_from_display_offset(source_offset, affinity)
                .map_err(|_| TextMaskedProjectionError::InvalidCoordinate),
        }
    }

    /// Binds this coordinate projection to the *same* retained M8
    /// measurement/paint layout, without shaping or storing another layout.
    ///
    /// # Errors
    ///
    /// Rejects missing or nonmatching retained text or unsupported caret maps.
    pub fn caret_map(
        &self,
        state: &TextLayoutState,
    ) -> Result<TextMaskedCaretMap, TextMaskedProjectionError> {
        let map = state.caret_map_for_source(self.snapshot(), &self.display)?;
        Ok(TextMaskedCaretMap {
            projection: self.clone(),
            map,
        })
    }

    fn to_masked_position(
        &self,
        position: &TextDisplayPosition,
    ) -> Result<TextDisplayPosition, TextMaskedProjectionError> {
        let offset = self.display_offset_for_position(position)?;
        let checked =
            TextPosition::new(self.snapshot(), &self.display, offset, position.affinity())
                .map_err(|_| TextMaskedProjectionError::InvalidCoordinate)?;
        Ok(TextDisplayPosition::Document(checked))
    }

    fn restore_source_position(
        &self,
        position: &TextDisplayPosition,
    ) -> Result<TextDisplayPosition, TextMaskedProjectionError> {
        if position.snapshot() != self.snapshot() {
            return Err(TextMaskedProjectionError::ForeignSnapshot);
        }
        let TextDisplayPosition::Document(position) = position else {
            return Err(TextMaskedProjectionError::ForeignComposition);
        };
        self.position_from_display_offset(position.byte_offset(), position.affinity())
    }

    fn to_masked_selection(
        &self,
        selection: &TextDisplaySelection,
    ) -> Result<TextDisplaySelection, TextMaskedProjectionError> {
        Ok(TextDisplaySelection::new(
            self.to_masked_position(selection.anchor())?,
            self.to_masked_position(selection.active())?,
        ))
    }

    fn restore_source_selection(
        &self,
        selection: &TextDisplaySelection,
    ) -> Result<TextDisplaySelection, TextMaskedProjectionError> {
        Ok(TextDisplaySelection::new(
            self.restore_source_position(selection.anchor())?,
            self.restore_source_position(selection.active())?,
        ))
    }
}

impl fmt::Debug for TextMaskedProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextMaskedProjection")
            .field("redacted", &true)
            .finish_non_exhaustive()
    }
}

/// Correlated geometric/text-navigation view over exactly one retained M8 map.
/// It only translates coordinates; Parley remains the only caret/layout authority.
#[derive(Clone)]
pub struct TextMaskedCaretMap {
    projection: TextMaskedProjection,
    map: TextCaretMap,
}

impl TextMaskedCaretMap {
    #[must_use]
    pub fn is_correlated_with(&self, artifact: &crate::TextArtifact) -> bool {
        self.map.is_correlated_with(artifact)
    }

    /// Returns exact retained-layout geometry for an M10 source/preedit caret.
    ///
    /// # Errors
    ///
    /// Rejects foreign/invalid coordinates or invalid retained layout geometry.
    pub fn caret_rect(
        &self,
        position: &TextDisplayPosition,
        width: LogicalLength,
    ) -> Result<LogicalRect, TextMaskedProjectionError> {
        let masked = self.projection.to_masked_position(position)?;
        self.map.caret_rect(&masked, width).map_err(Into::into)
    }

    /// Returns correlated selection geometry without exposing selected source.
    ///
    /// # Errors
    ///
    /// Rejects invalid source/preedit selection endpoints.
    pub fn selection_rects(
        &self,
        selection: &TextDisplaySelection,
    ) -> Result<Vec<TextSelectionRect>, TextMaskedProjectionError> {
        let masked = self.projection.to_masked_selection(selection)?;
        self.map.selection_rects(&masked).map_err(Into::into)
    }

    /// Maps an initial displayed pointer through the existing retained hit path.
    ///
    /// # Errors
    ///
    /// Rejects a stale surface snapshot or invalid presentation transform.
    pub fn hit_test(
        &self,
        snapshot: TextDocumentSnapshot,
        point: LogicalPoint,
        eligible: LogicalRect,
        transform: LogicalTransform,
    ) -> Result<Option<TextDisplayPosition>, TextMaskedProjectionError> {
        self.map
            .hit_test(snapshot, point, eligible, transform)?
            .map(|position| self.projection.restore_source_position(&position))
            .transpose()
    }

    /// Returns an exact masked-layout nearest source/preedit caret.
    ///
    /// # Errors
    ///
    /// Rejects a stale snapshot or invalid transform.
    pub fn nearest_position(
        &self,
        snapshot: TextDocumentSnapshot,
        point: LogicalPoint,
        transform: LogicalTransform,
    ) -> Result<TextDisplayPosition, TextMaskedProjectionError> {
        let position = self.map.nearest_position(snapshot, point, transform)?;
        self.projection.restore_source_position(&position)
    }

    /// Navigates against the retained shaped **mask**, returning source/preedit
    /// coordinates and the unchanged M10 preferred-inline navigation hint.
    /// Word navigation here follows the mask's word boundaries, **not** the
    /// original secret's linguistic words; callers must define their desired
    /// password word-navigation policy at the M10 ingress.
    ///
    /// # Errors
    ///
    /// Rejects invalid/stale selection, or unsupported caret navigation.
    pub fn navigate(
        &self,
        selection: &TextDisplaySelection,
        operation: TextNavigation,
        mode: TextNavigationMode,
        preferred_inline: Option<TextPreferredInline>,
    ) -> Result<(TextDisplaySelection, Option<TextPreferredInline>), TextMaskedProjectionError>
    {
        let masked = self.projection.to_masked_selection(selection)?;
        let result = self
            .map
            .navigate(&masked, operation, mode, preferred_inline)?;
        Ok((
            self.projection
                .restore_source_selection(result.selection())?,
            result.preferred_inline(),
        ))
    }

    /// Returns legal **durable document** byte offsets for the retained mask.
    /// A preedit projection has synthetic offsets and must not expose them as
    /// untyped durable document bytes. Use checked `TextDisplayPosition` mapping
    /// for synthetic composition positions instead.
    ///
    /// # Errors
    ///
    /// Rejects preedit coordinate space and inconsistent retained grapheme mapping.
    pub fn legal_source_offsets(&self) -> Result<Vec<usize>, TextMaskedProjectionError> {
        if !matches!(self.projection.source, SourceSpace::Document { .. }) {
            return Err(TextMaskedProjectionError::ForeignComposition);
        }
        self.map
            .legal_byte_offsets()
            .into_iter()
            .map(|offset| {
                self.projection
                    .boundaries
                    .binary_search_by_key(&offset, |(_, displayed)| *displayed)
                    .map(|index| self.projection.boundaries[index].0)
                    .map_err(|_| TextMaskedProjectionError::NotGraphemeBoundary)
            })
            .collect()
    }
}

impl fmt::Debug for TextMaskedCaretMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextMaskedCaretMap")
            .field("redacted", &true)
            .finish_non_exhaustive()
    }
}

impl From<TextPreeditProjectionError> for TextMaskedProjectionError {
    fn from(_: TextPreeditProjectionError) -> Self {
        Self::InvalidCoordinate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FontSourcePolicy, TextConstraints, TextRequest, TextSystem};
    use runenui_core::{
        __runtime::RuntimeNamespace, CompositionRange, TextDocumentId, TextDocumentRevision,
        TextPreeditPosition, TextRange, Typography,
    };

    fn snapshot() -> TextDocumentSnapshot {
        TextDocumentSnapshot::new(TextDocumentId::new(199), TextDocumentRevision::new(5))
    }

    #[test]
    fn unicode_graphemes_map_exact_source_boundaries_without_plaintext_debug() {
        let source = "s\u{0065}\u{0301}🔐👩‍👩‍👧‍👧秘密";
        let projection = TextMaskedProjection::document(snapshot(), source)
            .unwrap_or_else(|_| unreachable!("fixture mask is valid"));
        assert_eq!(projection.display_text(), "••••••");
        assert!(!projection.display_text().contains("秘密"));
        assert!(!format!("{projection:?}").contains("秘密"));
        let boundary = source
            .find('🔐')
            .unwrap_or_else(|| unreachable!("fixture contains emoji"));
        let p = TextDisplayPosition::Document(
            TextPosition::new(snapshot(), source, boundary, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("valid grapheme boundary")),
        );
        let masked = projection
            .display_offset_for_position(&p)
            .unwrap_or_else(|_| unreachable!("mapped boundary"));
        assert_eq!(masked, 6);
        assert_eq!(
            projection
                .position_from_display_offset(masked, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("inverse mapped boundary")),
            p
        );
        assert_eq!(
            projection.source_offset_to_display(2),
            Err(TextMaskedProjectionError::NotGraphemeBoundary)
        );
        assert_eq!(
            projection.position_from_display_offset(1, TextAffinity::Downstream),
            Err(TextMaskedProjectionError::NotGraphemeBoundary)
        );
    }

    #[test]
    fn empty_document_and_foreign_snapshot_fail_closed() {
        let projection = TextMaskedProjection::document(snapshot(), "")
            .unwrap_or_else(|_| unreachable!("empty mask is valid"));
        assert_eq!(projection.display_text(), "");
        assert_eq!(
            projection
                .position_from_display_offset(0, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("empty boundary is valid"))
                .snapshot(),
            snapshot()
        );
        let foreign =
            TextDocumentSnapshot::new(TextDocumentId::new(199), TextDocumentRevision::new(6));
        let position = TextDisplayPosition::Document(
            TextPosition::new(foreign, "", 0, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("foreign coordinate is individually valid")),
        );
        assert_eq!(
            projection.display_offset_for_position(&position),
            Err(TextMaskedProjectionError::ForeignSnapshot)
        );
    }

    #[test]
    fn controlled_font_mask_geometry_and_navigation_reuse_one_retained_layout() {
        const FONT: &[u8] = include_bytes!("../tests/fixtures/Cantarell-Regular.ttf");
        let source = "a\u{0301}🧑‍🔬z";
        let projection = TextMaskedProjection::document(snapshot(), source)
            .unwrap_or_else(|_| unreachable!("valid source"));
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        assert!(
            system
                .register_font_bytes(FONT.to_vec())
                .unwrap_or_else(|_| unreachable!("fixture font"))
                > 0
        );
        let mut state = TextLayoutState::new();
        let request = TextRequest::new(
            projection.display_text(),
            Typography::default(),
            TextConstraints::unbounded(),
        );
        let artifact = system
            .layout_text(&mut state, &request)
            .unwrap_or_else(|_| unreachable!("masked layout succeeds"))
            .artifact()
            .clone();
        let map = projection
            .caret_map(&state)
            .unwrap_or_else(|_| unreachable!("mask corresponds to retained layout"));
        assert!(map.is_correlated_with(&artifact));
        assert_eq!(
            map.legal_source_offsets()
                .unwrap_or_else(|_| unreachable!("masked grapheme stops")),
            vec![0, 3, 14, 15]
        );
        let begin = TextDisplayPosition::Document(
            TextPosition::new(snapshot(), source, 0, TextAffinity::Downstream)
                .unwrap_or_else(|_| unreachable!("source start")),
        );
        let end = TextDisplayPosition::Document(
            TextPosition::new(snapshot(), source, source.len(), TextAffinity::Upstream)
                .unwrap_or_else(|_| unreachable!("source end")),
        );
        assert!(map.caret_rect(&begin, LogicalLength::from(1_u8)).is_ok());
        assert!(
            !map.selection_rects(&TextDisplaySelection::new(begin.clone(), end))
                .unwrap_or_else(|_| unreachable!("grapheme-aligned selection"))
                .is_empty()
        );
        let expected_mask = state
            .caret_map_for_source(snapshot(), projection.display_text())
            .unwrap_or_else(|_| unreachable!("retained mask is exact"));
        assert!(projection.caret_map(&TextLayoutState::new()).is_err());
        assert!(map.is_correlated_with(expected_mask.artifact()));
        let (moved, _) = map
            .navigate(
                &TextDisplaySelection::new(begin.clone(), begin.clone()),
                TextNavigation::NextLogical,
                TextNavigationMode::Move,
                None,
            )
            .unwrap_or_else(|_| unreachable!("masked logical navigation succeeds"));
        assert_eq!(
            moved.active(),
            &TextDisplayPosition::Document(
                TextPosition::new(snapshot(), source, 3, TextAffinity::Downstream)
                    .unwrap_or_else(|_| unreachable!("first grapheme boundary"))
            )
        );
    }

    #[test]
    fn composed_secret_preedit_masks_synthetic_positions_and_preserves_caret_geometry() {
        const FONT: &[u8] = include_bytes!("../tests/fixtures/Cantarell-Regular.ttf");
        let source = "abXYZcd";
        let composing = "かな";
        let namespace = RuntimeNamespace::__runtime_new();
        let generation = namespace.__runtime_composition_generation(91);
        let replacement = TextRange::new(snapshot(), source, 2, 5)
            .unwrap_or_else(|_| unreachable!("replacement is checked"));
        let projection = Arc::new(
            TextPreeditProjection::new(
                snapshot(),
                source,
                replacement,
                generation.clone(),
                composing,
                Some(
                    CompositionRange::new(composing, 0, "か".len())
                        .unwrap_or_else(|_| unreachable!("composition range is checked")),
                ),
            )
            .unwrap_or_else(|_| unreachable!("transient preedit is checked")),
        );
        let mask = TextMaskedProjection::preedit(Arc::clone(&projection))
            .unwrap_or_else(|_| unreachable!("complete preedit display can be masked"));
        assert_eq!(mask.display_text(), "••••••");
        assert!(!mask.display_text().contains("かな"));
        assert!(!format!("{mask:?}").contains("かな"));
        assert!(!format!("{mask:?}").contains("XYZ"));

        let synthetic = TextDisplayPosition::Preedit(
            TextPreeditPosition::new(
                snapshot(),
                generation,
                composing,
                "か".len(),
                TextAffinity::Downstream,
            )
            .unwrap_or_else(|_| unreachable!("synthetic coordinate is valid")),
        );
        assert_eq!(
            mask.display_offset_for_position(&synthetic),
            Ok(3 * '•'.len_utf8())
        );
        assert_eq!(
            mask.position_from_display_offset(3 * '•'.len_utf8(), TextAffinity::Downstream),
            Ok(synthetic.clone())
        );
        let foreign = TextDisplayPosition::Preedit(
            TextPreeditPosition::new(
                snapshot(),
                namespace.__runtime_composition_generation(92),
                composing,
                "か".len(),
                TextAffinity::Downstream,
            )
            .unwrap_or_else(|_| unreachable!("foreign generation is valid")),
        );
        assert_eq!(
            mask.display_offset_for_position(&foreign),
            Err(TextMaskedProjectionError::ForeignComposition)
        );

        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        assert!(
            system
                .register_font_bytes(FONT.to_vec())
                .unwrap_or_else(|_| unreachable!("font registers"))
                > 0
        );
        let mut state = TextLayoutState::new();
        system
            .layout_text(
                &mut state,
                &TextRequest::new(
                    mask.display_text(),
                    Typography::default(),
                    TextConstraints::unbounded(),
                ),
            )
            .unwrap_or_else(|_| unreachable!("mask shapes"));
        let map = mask
            .caret_map(&state)
            .unwrap_or_else(|_| unreachable!("one retained layout is correlated"));
        assert_eq!(
            map.legal_source_offsets(),
            Err(TextMaskedProjectionError::ForeignComposition)
        );
        assert!(
            map.caret_rect(&synthetic, LogicalLength::from(1_u8))
                .is_ok()
        );
    }

    #[test]
    fn preedit_that_joins_prior_grapheme_rejects_hidden_interior_caret() {
        let source = "a";
        let composing = "\u{0301}";
        let namespace = RuntimeNamespace::__runtime_new();
        let generation = namespace.__runtime_composition_generation(93);
        let replacement = TextRange::new(snapshot(), source, source.len(), source.len())
            .unwrap_or_else(|_| unreachable!("empty replacement range is valid"));
        let preedit = Arc::new(
            TextPreeditProjection::new(
                snapshot(),
                source,
                replacement,
                generation.clone(),
                composing,
                None,
            )
            .unwrap_or_else(|_| unreachable!("joined grapheme preedit is valid")),
        );
        let mask = TextMaskedProjection::preedit(preedit)
            .unwrap_or_else(|_| unreachable!("grapheme mask is valid"));
        assert_eq!(mask.display_text(), "•");
        let at_preedit_start = TextDisplayPosition::Preedit(
            TextPreeditPosition::new(
                snapshot(),
                generation,
                composing,
                0,
                TextAffinity::Downstream,
            )
            .unwrap_or_else(|_| unreachable!("synthetic position is scalar aligned")),
        );
        assert_eq!(
            mask.display_offset_for_position(&at_preedit_start),
            Err(TextMaskedProjectionError::NotGraphemeBoundary)
        );
    }
}
