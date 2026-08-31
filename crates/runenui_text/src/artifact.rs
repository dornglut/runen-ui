//! Immutable renderer-neutral logical text artifacts.

use core::{fmt, ops::Range};
use std::sync::Arc;

use fontique::Blob;
use runenui_core::{LogicalSize, ResourceRef};

use super::FontSourceSnapshot;

/// One immutable paragraph layout produced from a single shaping and line-breaking result.
///
/// Measurement and paint consumers observe the same line/run/resource facts from this value;
/// no renderer scale participates in the artifact identity.
#[derive(Clone, Debug)]
pub struct TextArtifact {
    size: LogicalSize,
    source_snapshot: FontSourceSnapshot,
    lines: Arc<[TextLine]>,
}

impl TextArtifact {
    #[cfg(test)]
    pub(crate) fn new(
        size: LogicalSize,
        source_snapshot: FontSourceSnapshot,
        lines: Vec<TextLine>,
    ) -> Self {
        Self {
            size,
            source_snapshot,
            lines: lines.into(),
        }
    }

    /// Returns the exact logical paragraph size measured by the shaping result.
    #[must_use]
    pub const fn size(&self) -> LogicalSize {
        self.size
    }

    /// Returns the exact font-source identity and revision used for this artifact.
    #[must_use]
    pub const fn source_snapshot(&self) -> &FontSourceSnapshot {
        &self.source_snapshot
    }

    /// Returns the positioned lines in logical block order.
    #[must_use]
    pub fn lines(&self) -> &[TextLine] {
        &self.lines
    }
}

/// One positioned line from a [`TextArtifact`].
#[derive(Clone, Debug)]
pub struct TextLine {
    text_range: Range<usize>,
    metrics: TextLineMetrics,
    runs: Arc<[TextRun]>,
}

impl TextLine {
    #[cfg(test)]
    pub(crate) fn new(
        text_range: Range<usize>,
        metrics: TextLineMetrics,
        runs: Vec<TextRun>,
    ) -> Self {
        Self {
            text_range,
            metrics,
            runs: runs.into(),
        }
    }

    /// Returns the source UTF-8 byte range represented by this line.
    #[must_use]
    pub fn text_range(&self) -> Range<usize> {
        self.text_range.clone()
    }

    /// Returns exact logical line-box and baseline metrics.
    #[must_use]
    pub const fn metrics(&self) -> TextLineMetrics {
        self.metrics
    }

    /// Returns positioned shaped runs in visual paint order.
    #[must_use]
    pub fn runs(&self) -> &[TextRun] {
        &self.runs
    }
}

/// Exact logical metrics for one line.
///
/// Instances are produced only from finite Parley layout values. Signed coordinates are kept as
/// scalar logical offsets because alignment and line-box bounds may legitimately be negative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLineMetrics {
    line_height: f32,
    baseline: f32,
    offset: f32,
    advance: f32,
    trailing_whitespace: f32,
    inline_min: f32,
    inline_max: f32,
    block_min: f32,
    block_max: f32,
}

impl TextLineMetrics {
    #[cfg(test)]
    pub(crate) fn from_finite(values: [f32; 9]) -> Option<Self> {
        values.iter().all(|value| value.is_finite()).then(|| Self {
            line_height: values[0],
            baseline: values[1],
            offset: values[2],
            advance: values[3],
            trailing_whitespace: values[4],
            inline_min: values[5],
            inline_max: values[6],
            block_min: values[7],
            block_max: values[8],
        })
    }

    /// Returns the absolute logical line height.
    #[must_use]
    pub const fn line_height(self) -> f32 {
        self.line_height
    }

    /// Returns the block-axis baseline coordinate.
    #[must_use]
    pub const fn baseline(self) -> f32 {
        self.baseline
    }

    /// Returns the inline alignment offset.
    #[must_use]
    pub const fn offset(self) -> f32 {
        self.offset
    }

    /// Returns the full inline advance including trailing whitespace.
    #[must_use]
    pub const fn advance(self) -> f32 {
        self.advance
    }

    /// Returns the advance contributed by trailing whitespace.
    #[must_use]
    pub const fn trailing_whitespace(self) -> f32 {
        self.trailing_whitespace
    }

    /// Returns the minimum inline-axis coordinate of the line box.
    #[must_use]
    pub const fn inline_min(self) -> f32 {
        self.inline_min
    }

    /// Returns the maximum inline-axis coordinate of the line box.
    #[must_use]
    pub const fn inline_max(self) -> f32 {
        self.inline_max
    }

    /// Returns the minimum block-axis coordinate of the line box.
    #[must_use]
    pub const fn block_min(self) -> f32 {
        self.block_min
    }

    /// Returns the maximum block-axis coordinate of the line box.
    #[must_use]
    pub const fn block_max(self) -> f32 {
        self.block_max
    }
}

/// Logical inline direction selected by Unicode shaping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

impl TextDirection {
    #[cfg(test)]
    pub(crate) const fn from_rtl(rtl: bool) -> Self {
        if rtl {
            Self::RightToLeft
        } else {
            Self::LeftToRight
        }
    }

    /// Returns whether the direction is right-to-left.
    #[must_use]
    pub const fn is_rtl(self) -> bool {
        matches!(self, Self::RightToLeft)
    }
}

/// One typed cluster classification flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextClusterFlag {
    LigatureStart,
    LigatureContinuation,
    WordBoundary,
    SoftLineBreak,
    HardLineBreak,
    SpaceOrNbsp,
    Emoji,
}

impl TextClusterFlag {
    const fn mask(self) -> u8 {
        match self {
            Self::LigatureStart => 1 << 0,
            Self::LigatureContinuation => 1 << 1,
            Self::WordBoundary => 1 << 2,
            Self::SoftLineBreak => 1 << 3,
            Self::HardLineBreak => 1 << 4,
            Self::SpaceOrNbsp => 1 << 5,
            Self::Emoji => 1 << 6,
        }
    }
}

/// Compact typed Unicode/shaping classifications for one cluster.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextClusterFlags(u8);

impl TextClusterFlags {
    pub const NONE: Self = Self(0);

    /// Sets or clears one typed cluster classification.
    #[must_use]
    pub const fn with(mut self, flag: TextClusterFlag, enabled: bool) -> Self {
        let mask = flag.mask();
        if enabled {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
        self
    }

    /// Returns whether one typed classification is present.
    #[must_use]
    pub const fn contains(self, flag: TextClusterFlag) -> bool {
        self.0 & flag.mask() != 0
    }
}

/// One positioned logical shaped run within a line.
#[derive(Clone, Debug)]
pub struct TextRun {
    text_range: Range<usize>,
    origin_x: f32,
    origin_y: f32,
    advance: f32,
    direction: TextDirection,
    clusters: Arc<[TextCluster]>,
    shaped: Arc<ShapedTextResource>,
}

impl TextRun {
    #[cfg(test)]
    pub(crate) fn new(
        text_range: Range<usize>,
        origin_x: f32,
        origin_y: f32,
        advance: f32,
        direction: TextDirection,
        clusters: Vec<TextCluster>,
        shaped: Arc<ShapedTextResource>,
    ) -> Option<Self> {
        [origin_x, origin_y, advance]
            .iter()
            .all(|value| value.is_finite())
            .then(|| Self {
                text_range,
                origin_x,
                origin_y,
                advance,
                direction,
                clusters: clusters.into(),
                shaped,
            })
    }

    /// Returns the source UTF-8 byte range represented by this positioned run.
    #[must_use]
    pub fn text_range(&self) -> Range<usize> {
        self.text_range.clone()
    }

    /// Returns the logical x coordinate at which the shaped resource is placed.
    #[must_use]
    pub const fn origin_x(&self) -> f32 {
        self.origin_x
    }

    /// Returns the logical baseline y coordinate at which the shaped resource is placed.
    #[must_use]
    pub const fn origin_y(&self) -> f32 {
        self.origin_y
    }

    /// Returns the logical inline advance of this positioned run.
    #[must_use]
    pub const fn advance(&self) -> f32 {
        self.advance
    }

    /// Returns the logical inline direction selected by shaping.
    #[must_use]
    pub const fn direction(&self) -> TextDirection {
        self.direction
    }

    /// Returns whether this run has right-to-left directionality.
    #[must_use]
    pub const fn is_rtl(&self) -> bool {
        self.direction.is_rtl()
    }

    /// Returns the run clusters in logical source order.
    #[must_use]
    pub fn clusters(&self) -> &[TextCluster] {
        &self.clusters
    }

    /// Returns the opaque scale-independent resource identity for this shaped run.
    #[must_use]
    pub fn resource_ref(&self) -> &ResourceRef {
        self.shaped.resource_ref()
    }

    /// Returns the immutable logical shaped-resource payload used by paint realization.
    #[must_use]
    pub fn shaped_resource(&self) -> &ShapedTextResource {
        &self.shaped
    }
}

/// One Unicode shaping cluster in logical source order.
#[derive(Clone, Debug, PartialEq)]
pub struct TextCluster {
    text_range: Range<usize>,
    advance: f32,
    direction: TextDirection,
    flags: TextClusterFlags,
}

impl TextCluster {
    #[cfg(test)]
    pub(crate) fn new(
        text_range: Range<usize>,
        advance: f32,
        direction: TextDirection,
        flags: TextClusterFlags,
    ) -> Option<Self> {
        advance.is_finite().then_some(Self {
            text_range,
            advance,
            direction,
            flags,
        })
    }

    /// Returns the source UTF-8 byte range represented by this cluster.
    #[must_use]
    pub fn text_range(&self) -> Range<usize> {
        self.text_range.clone()
    }

    /// Returns the logical inline advance of this cluster.
    #[must_use]
    pub const fn advance(&self) -> f32 {
        self.advance
    }

    /// Returns the logical inline direction selected by shaping.
    #[must_use]
    pub const fn direction(&self) -> TextDirection {
        self.direction
    }

    /// Returns all typed Unicode/shaping classifications for this cluster.
    #[must_use]
    pub const fn flags(&self) -> TextClusterFlags {
        self.flags
    }

    /// Returns whether this cluster has right-to-left directionality.
    #[must_use]
    pub const fn is_rtl(&self) -> bool {
        self.direction.is_rtl()
    }

    /// Returns whether this cluster begins a ligature.
    #[must_use]
    pub const fn is_ligature_start(&self) -> bool {
        self.flags.contains(TextClusterFlag::LigatureStart)
    }

    /// Returns whether this cluster continues a ligature.
    #[must_use]
    pub const fn is_ligature_continuation(&self) -> bool {
        self.flags.contains(TextClusterFlag::LigatureContinuation)
    }

    /// Returns whether this cluster begins a word boundary.
    #[must_use]
    pub const fn is_word_boundary(&self) -> bool {
        self.flags.contains(TextClusterFlag::WordBoundary)
    }

    /// Returns whether this cluster is a soft line-break opportunity.
    #[must_use]
    pub const fn is_soft_line_break(&self) -> bool {
        self.flags.contains(TextClusterFlag::SoftLineBreak)
    }

    /// Returns whether this cluster carries a hard line break.
    #[must_use]
    pub const fn is_hard_line_break(&self) -> bool {
        self.flags.contains(TextClusterFlag::HardLineBreak)
    }

    /// Returns whether this cluster represents space or non-breaking space.
    #[must_use]
    pub const fn is_space_or_nbsp(&self) -> bool {
        self.flags.contains(TextClusterFlag::SpaceOrNbsp)
    }

    /// Returns whether this cluster is an emoji sequence.
    #[must_use]
    pub const fn is_emoji(&self) -> bool {
        self.flags.contains(TextClusterFlag::Emoji)
    }
}

/// Immutable exact font binding used by one logical shaped resource.
///
/// The byte slice and face index identify the exact font face. Normalized variation coordinates
/// and synthesis facts preserve the exact outline realization inputs selected during shaping.
#[derive(Clone)]
pub struct TextFontBinding {
    data: Blob<u8>,
    face_index: u32,
    normalized_coords: Arc<[i16]>,
    faux_bold: bool,
    faux_skew: Option<f32>,
}

impl TextFontBinding {
    #[cfg(test)]
    pub(crate) fn new(
        data: Blob<u8>,
        face_index: u32,
        normalized_coords: Vec<i16>,
        faux_bold: bool,
        faux_skew: Option<f32>,
    ) -> Option<Self> {
        faux_skew.is_none_or(f32::is_finite).then(|| Self {
            data,
            face_index,
            normalized_coords: normalized_coords.into(),
            faux_bold,
            faux_skew,
        })
    }

    /// Returns the immutable exact font-file bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.data.as_ref()
    }

    /// Returns the face index inside the immutable font data.
    #[must_use]
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }

    /// Returns normalized variation coordinates selected by shaping.
    #[must_use]
    pub fn normalized_coords(&self) -> &[i16] {
        &self.normalized_coords
    }

    /// Returns whether faux emboldening is required for exact outline realization.
    #[must_use]
    pub const fn faux_bold(&self) -> bool {
        self.faux_bold
    }

    /// Returns the faux skew angle required for exact outline realization, when any.
    #[must_use]
    pub const fn faux_skew(&self) -> Option<f32> {
        self.faux_skew
    }
}

impl fmt::Debug for TextFontBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextFontBinding")
            .field("byte_len", &self.data.as_ref().len())
            .field("face_index", &self.face_index)
            .field("normalized_coords", &self.normalized_coords)
            .field("faux_bold", &self.faux_bold)
            .field("faux_skew", &self.faux_skew)
            .finish()
    }
}

/// Immutable logical glyph geometry bound to one exact font face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextGlyph {
    id: u32,
    x: f32,
    y: f32,
    advance: f32,
}

impl TextGlyph {
    #[cfg(test)]
    pub(crate) fn new(id: u32, x: f32, y: f32, advance: f32) -> Option<Self> {
        [x, y, advance]
            .iter()
            .all(|value| value.is_finite())
            .then_some(Self { id, x, y, advance })
    }

    /// Returns the font-local glyph identifier.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.id
    }

    /// Returns the logical x offset relative to the positioned run origin.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    /// Returns the logical y offset relative to the positioned run baseline.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }

    /// Returns this glyph's logical inline advance.
    #[must_use]
    pub const fn advance(self) -> f32 {
        self.advance
    }
}

/// Immutable scale-independent shaped resource retained while artifacts or leases are live.
#[derive(Debug)]
pub struct ShapedTextResource {
    resource_ref: ResourceRef,
    font: TextFontBinding,
    font_size: f32,
    glyphs: Arc<[TextGlyph]>,
}

impl ShapedTextResource {
    #[cfg(test)]
    pub(crate) fn new(
        resource_ref: ResourceRef,
        font: TextFontBinding,
        font_size: f32,
        glyphs: Vec<TextGlyph>,
    ) -> Option<Self> {
        font_size.is_finite().then(|| Self {
            resource_ref,
            font,
            font_size,
            glyphs: glyphs.into(),
        })
    }

    /// Returns the opaque scale-independent shaped-resource identity.
    #[must_use]
    pub const fn resource_ref(&self) -> &ResourceRef {
        &self.resource_ref
    }

    /// Returns the exact immutable font binding used by shaping.
    #[must_use]
    pub const fn font(&self) -> &TextFontBinding {
        &self.font
    }

    /// Returns the logical font size used by shaping.
    #[must_use]
    pub const fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Returns local logical glyph geometry in paint order.
    #[must_use]
    pub fn glyphs(&self) -> &[TextGlyph] {
        &self.glyphs
    }
}

/// Strong lifetime token for one immutable shaped resource.
///
/// Runtime caches or retained publications hold this token for exactly as long as the associated
/// [`ResourceRef`] must remain resolvable. Dropping the last artifact/lease permits reclamation;
/// the text system keeps only weak lookup entries.
#[derive(Clone, Debug)]
pub struct ShapedTextLease {
    shaped: Arc<ShapedTextResource>,
}

impl ShapedTextLease {
    pub(crate) const fn new(shaped: Arc<ShapedTextResource>) -> Self {
        Self { shaped }
    }

    /// Returns the sole opaque identity of the leased logical shaped resource.
    #[must_use]
    pub fn resource_ref(&self) -> &ResourceRef {
        self.shaped.resource_ref()
    }

    /// Returns the immutable logical shaped-resource payload.
    #[must_use]
    pub fn shaped_resource(&self) -> &ShapedTextResource {
        &self.shaped
    }
}
