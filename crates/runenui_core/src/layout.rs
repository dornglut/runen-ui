//! Host-neutral authored layout intent.
//!
//! These values describe `RunenUI` semantics only. Runtime may lower them into a
//! layout algorithm, but dependency types, algorithm identities, and caches are
//! never part of this public contract.

use core::{error::Error, fmt};

use crate::{EdgeInsets, LogicalLength};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    Vertical,
    Horizontal,
}

/// Error returned when a dimensionless layout factor is not finite and non-negative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutFactorError {
    NotFinite,
    Negative,
}

impl fmt::Display for LayoutFactorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => formatter.write_str("layout factor must be finite"),
            Self::Negative => formatter.write_str("layout factor must not be negative"),
        }
    }
}

impl Error for LayoutFactorError {}

/// Finite non-negative dimensionless layout factor.
///
/// Percentages use ratio form (`0.5 == 50%`). Flex growth/shrink and grid
/// fractional tracks reuse this validated scalar without sharing semantic meaning.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct LayoutFactor(f32);

impl LayoutFactor {
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);

    /// # Errors
    ///
    /// Returns [`LayoutFactorError`] for NaN, infinity, or negative values.
    pub const fn new(value: f32) -> Result<Self, LayoutFactorError> {
        if value.is_nan() || value == f32::INFINITY || value == f32::NEG_INFINITY {
            Err(LayoutFactorError::NotFinite)
        } else if value < 0.0 {
            Err(LayoutFactorError::Negative)
        } else if value == 0.0 {
            Ok(Self::ZERO)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Preferred border-box size on one logical axis.
///
/// Padding is inside this size and margin remains outside it. `Fill` consumes
/// finite available space subject to min/max constraints; when no finite extent
/// is available it participates through intrinsic sizing rather than creating an
/// infinite logical size.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LayoutDimension {
    #[default]
    Auto,
    Length(LogicalLength),
    Percent(LayoutFactor),
    MinContent,
    MaxContent,
    Fill,
}

impl LayoutDimension {
    #[must_use]
    pub const fn length(value: LogicalLength) -> Self {
        Self::Length(value)
    }

    #[must_use]
    pub const fn percent(value: LayoutFactor) -> Self {
        Self::Percent(value)
    }
}

impl From<LogicalLength> for LayoutDimension {
    fn from(value: LogicalLength) -> Self {
        Self::Length(value)
    }
}

/// Minimum/maximum bound on one logical axis.
///
/// `Auto` means the algorithm-defined automatic minimum when used as a minimum
/// and no authored maximum when used as a maximum.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LayoutBound {
    #[default]
    Auto,
    Length(LogicalLength),
    Percent(LayoutFactor),
}

impl LayoutBound {
    #[must_use]
    pub const fn length(value: LogicalLength) -> Self {
        Self::Length(value)
    }

    #[must_use]
    pub const fn percent(value: LayoutFactor) -> Self {
        Self::Percent(value)
    }
}

impl From<LogicalLength> for LayoutBound {
    fn from(value: LogicalLength) -> Self {
        Self::Length(value)
    }
}

/// Offset from one edge of the containing block for positioned layout.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LayoutOffset {
    #[default]
    Auto,
    Length(LogicalLength),
    Percent(LayoutFactor),
}

impl LayoutOffset {
    #[must_use]
    pub const fn length(value: LogicalLength) -> Self {
        Self::Length(value)
    }

    #[must_use]
    pub const fn percent(value: LayoutFactor) -> Self {
        Self::Percent(value)
    }
}

impl From<LogicalLength> for LayoutOffset {
    fn from(value: LogicalLength) -> Self {
        Self::Length(value)
    }
}

/// Independent horizontal and vertical spacing between sibling tracks/items.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutGap {
    horizontal: LogicalLength,
    vertical: LogicalLength,
}

impl LayoutGap {
    pub const ZERO: Self = Self {
        horizontal: LogicalLength::ZERO,
        vertical: LogicalLength::ZERO,
    };

    #[must_use]
    pub const fn new(horizontal: LogicalLength, vertical: LogicalLength) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    #[must_use]
    pub const fn all(value: LogicalLength) -> Self {
        Self::new(value, value)
    }

    #[must_use]
    pub const fn horizontal(self) -> LogicalLength {
        self.horizontal
    }

    #[must_use]
    pub const fn vertical(self) -> LogicalLength {
        self.vertical
    }

    #[must_use]
    pub const fn along(self, axis: Axis) -> LogicalLength {
        match axis {
            Axis::Horizontal => self.horizontal,
            Axis::Vertical => self.vertical,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

/// Alignment of individual items on an alignment axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ItemAlignment {
    #[default]
    Stretch,
    Start,
    End,
    Center,
    Baseline,
}

/// Distribution along a Flex container's main axis.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MainAxisAlignment {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Distribution of multiple lines/tracks within available container space.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ContentAlignment {
    #[default]
    Stretch,
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FlexContainerStyle {
    direction: FlexDirection,
    wrap: FlexWrap,
    justify_content: MainAxisAlignment,
    align_items: ItemAlignment,
    align_content: ContentAlignment,
}

impl FlexContainerStyle {
    #[must_use]
    pub const fn direction(self) -> FlexDirection {
        self.direction
    }

    #[must_use]
    pub const fn wrap(self) -> FlexWrap {
        self.wrap
    }

    #[must_use]
    pub const fn justify_content(self) -> MainAxisAlignment {
        self.justify_content
    }

    #[must_use]
    pub const fn align_items(self) -> ItemAlignment {
        self.align_items
    }

    #[must_use]
    pub const fn align_content(self) -> ContentAlignment {
        self.align_content
    }

    #[must_use]
    pub const fn with_direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub const fn with_wrap(mut self, wrap: FlexWrap) -> Self {
        self.wrap = wrap;
        self
    }

    #[must_use]
    pub const fn with_justify_content(mut self, alignment: MainAxisAlignment) -> Self {
        self.justify_content = alignment;
        self
    }

    #[must_use]
    pub const fn with_align_items(mut self, alignment: ItemAlignment) -> Self {
        self.align_items = alignment;
        self
    }

    #[must_use]
    pub const fn with_align_content(mut self, alignment: ContentAlignment) -> Self {
        self.align_content = alignment;
        self
    }
}

/// Initial main-axis size of a Flex item.
///
/// This is deliberately distinct from [`LayoutDimension`]: `Fill` is a node
/// sizing semantic, while `Content` is specific to Flex basis calculation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FlexBasis {
    #[default]
    Auto,
    Content,
    Length(LogicalLength),
    Percent(LayoutFactor),
}

impl FlexBasis {
    #[must_use]
    pub const fn length(value: LogicalLength) -> Self {
        Self::Length(value)
    }

    #[must_use]
    pub const fn percent(value: LayoutFactor) -> Self {
        Self::Percent(value)
    }
}

impl From<LogicalLength> for FlexBasis {
    fn from(value: LogicalLength) -> Self {
        Self::Length(value)
    }
}

/// Flex-specific participation of one node when its parent is a flex container.
///
/// Child ordering remains the runtime's authored mounted order; no independent
/// Flex ordering property is part of the M8C model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexItemStyle {
    grow: LayoutFactor,
    shrink: LayoutFactor,
    basis: FlexBasis,
    align_self: Option<ItemAlignment>,
}

impl Default for FlexItemStyle {
    fn default() -> Self {
        Self {
            grow: LayoutFactor::ZERO,
            shrink: LayoutFactor::ONE,
            basis: FlexBasis::Auto,
            align_self: None,
        }
    }
}

impl FlexItemStyle {
    #[must_use]
    pub const fn grow(self) -> LayoutFactor {
        self.grow
    }

    #[must_use]
    pub const fn shrink(self) -> LayoutFactor {
        self.shrink
    }

    #[must_use]
    pub const fn basis(self) -> FlexBasis {
        self.basis
    }

    #[must_use]
    pub const fn align_self(self) -> Option<ItemAlignment> {
        self.align_self
    }

    #[must_use]
    pub const fn with_grow(mut self, grow: LayoutFactor) -> Self {
        self.grow = grow;
        self
    }

    #[must_use]
    pub const fn with_shrink(mut self, shrink: LayoutFactor) -> Self {
        self.shrink = shrink;
        self
    }

    #[must_use]
    pub const fn with_basis(mut self, basis: FlexBasis) -> Self {
        self.basis = basis;
        self
    }

    #[must_use]
    pub const fn with_align_self(mut self, alignment: Option<ItemAlignment>) -> Self {
        self.align_self = alignment;
        self
    }
}

/// Legal minimum sizing function for a Grid track in the accepted M8C subset.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GridTrackMin {
    #[default]
    Auto,
    MinContent,
    MaxContent,
    Length(LogicalLength),
    Percent(LayoutFactor),
}

/// Legal maximum sizing function for a Grid track in the accepted M8C subset.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GridTrackMax {
    #[default]
    Auto,
    MinContent,
    MaxContent,
    Length(LogicalLength),
    Percent(LayoutFactor),
    Fraction(LayoutFactor),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GridTrack {
    min: GridTrackMin,
    max: GridTrackMax,
}

impl GridTrack {
    #[must_use]
    pub const fn new(min: GridTrackMin, max: GridTrackMax) -> Self {
        Self { min, max }
    }

    #[must_use]
    pub const fn auto() -> Self {
        Self::new(GridTrackMin::Auto, GridTrackMax::Auto)
    }

    #[must_use]
    pub const fn length(value: LogicalLength) -> Self {
        Self::new(GridTrackMin::Length(value), GridTrackMax::Length(value))
    }

    #[must_use]
    pub const fn fraction(value: LayoutFactor) -> Self {
        Self::new(GridTrackMin::Auto, GridTrackMax::Fraction(value))
    }

    #[must_use]
    pub const fn min(self) -> GridTrackMin {
        self.min
    }

    #[must_use]
    pub const fn max(self) -> GridTrackMax {
        self.max
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
}

/// Error returned when a one-based grid line is zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GridLineError;

impl fmt::Display for GridLineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("grid line must be one-based and non-zero")
    }
}

impl Error for GridLineError {}

/// One-based explicit grid line in the accepted positive-line subset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GridLine(u16);

impl GridLine {
    /// # Errors
    ///
    /// Returns [`GridLineError`] when `line == 0`.
    pub const fn new(line: u16) -> Result<Self, GridLineError> {
        if line == 0 {
            Err(GridLineError)
        } else {
            Ok(Self(line))
        }
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Error returned when a grid span is zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GridSpanError;

impl fmt::Display for GridSpanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("grid span must be non-zero")
    }
}

impl Error for GridSpanError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GridSpan(u16);

impl GridSpan {
    pub const ONE: Self = Self(1);

    /// # Errors
    ///
    /// Returns [`GridSpanError`] when `span == 0`.
    pub const fn new(span: u16) -> Result<Self, GridSpanError> {
        if span == 0 {
            Err(GridSpanError)
        } else {
            Ok(Self(span))
        }
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Placement on one grid axis. `start = None` delegates placement to auto flow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GridAxisPlacement {
    start: Option<GridLine>,
    span: GridSpan,
}

impl Default for GridAxisPlacement {
    fn default() -> Self {
        Self {
            start: None,
            span: GridSpan::ONE,
        }
    }
}

impl GridAxisPlacement {
    #[must_use]
    pub const fn new(start: Option<GridLine>, span: GridSpan) -> Self {
        Self { start, span }
    }

    #[must_use]
    pub const fn start(self) -> Option<GridLine> {
        self.start
    }

    #[must_use]
    pub const fn span(self) -> GridSpan {
        self.span
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GridItemPlacement {
    column: GridAxisPlacement,
    row: GridAxisPlacement,
}

impl GridItemPlacement {
    #[must_use]
    pub const fn new(column: GridAxisPlacement, row: GridAxisPlacement) -> Self {
        Self { column, row }
    }

    #[must_use]
    pub const fn column(self) -> GridAxisPlacement {
        self.column
    }

    #[must_use]
    pub const fn row(self) -> GridAxisPlacement {
        self.row
    }
}

/// Grid-specific participation of one node when its parent is a Grid container.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GridItemStyle {
    placement: GridItemPlacement,
    align_self: Option<ItemAlignment>,
    justify_self: Option<ItemAlignment>,
}

impl GridItemStyle {
    #[must_use]
    pub const fn placement(self) -> GridItemPlacement {
        self.placement
    }

    #[must_use]
    pub const fn align_self(self) -> Option<ItemAlignment> {
        self.align_self
    }

    #[must_use]
    pub const fn justify_self(self) -> Option<ItemAlignment> {
        self.justify_self
    }

    #[must_use]
    pub const fn with_placement(mut self, placement: GridItemPlacement) -> Self {
        self.placement = placement;
        self
    }

    #[must_use]
    pub const fn with_align_self(mut self, alignment: Option<ItemAlignment>) -> Self {
        self.align_self = alignment;
        self
    }

    #[must_use]
    pub const fn with_justify_self(mut self, alignment: Option<ItemAlignment>) -> Self {
        self.justify_self = alignment;
        self
    }
}

/// Grid container tracks, implicit placement, and two-axis alignment policy.
#[derive(Clone, Debug, PartialEq)]
pub struct GridContainerStyle {
    columns: Vec<GridTrack>,
    rows: Vec<GridTrack>,
    auto_columns: GridTrack,
    auto_rows: GridTrack,
    auto_flow: GridAutoFlow,
    align_items: ItemAlignment,
    justify_items: ItemAlignment,
    align_content: ContentAlignment,
    justify_content: ContentAlignment,
}

impl Default for GridContainerStyle {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            auto_columns: GridTrack::auto(),
            auto_rows: GridTrack::auto(),
            auto_flow: GridAutoFlow::Row,
            align_items: ItemAlignment::Stretch,
            justify_items: ItemAlignment::Stretch,
            align_content: ContentAlignment::Stretch,
            justify_content: ContentAlignment::Stretch,
        }
    }
}

impl GridContainerStyle {
    #[must_use]
    pub fn new(
        columns: impl IntoIterator<Item = GridTrack>,
        rows: impl IntoIterator<Item = GridTrack>,
    ) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            rows: rows.into_iter().collect(),
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn columns(&self) -> &[GridTrack] {
        self.columns.as_slice()
    }

    #[must_use]
    pub const fn rows(&self) -> &[GridTrack] {
        self.rows.as_slice()
    }

    #[must_use]
    pub const fn auto_columns(&self) -> GridTrack {
        self.auto_columns
    }

    #[must_use]
    pub const fn auto_rows(&self) -> GridTrack {
        self.auto_rows
    }

    #[must_use]
    pub const fn auto_flow(&self) -> GridAutoFlow {
        self.auto_flow
    }

    #[must_use]
    pub const fn align_items(&self) -> ItemAlignment {
        self.align_items
    }

    #[must_use]
    pub const fn justify_items(&self) -> ItemAlignment {
        self.justify_items
    }

    #[must_use]
    pub const fn align_content(&self) -> ContentAlignment {
        self.align_content
    }

    #[must_use]
    pub const fn justify_content(&self) -> ContentAlignment {
        self.justify_content
    }

    #[must_use]
    pub const fn with_auto_columns(mut self, track: GridTrack) -> Self {
        self.auto_columns = track;
        self
    }

    #[must_use]
    pub const fn with_auto_rows(mut self, track: GridTrack) -> Self {
        self.auto_rows = track;
        self
    }

    #[must_use]
    pub const fn with_auto_flow(mut self, flow: GridAutoFlow) -> Self {
        self.auto_flow = flow;
        self
    }

    #[must_use]
    pub const fn with_align_items(mut self, alignment: ItemAlignment) -> Self {
        self.align_items = alignment;
        self
    }

    #[must_use]
    pub const fn with_justify_items(mut self, alignment: ItemAlignment) -> Self {
        self.justify_items = alignment;
        self
    }

    #[must_use]
    pub const fn with_align_content(mut self, alignment: ContentAlignment) -> Self {
        self.align_content = alignment;
        self
    }

    #[must_use]
    pub const fn with_justify_content(mut self, alignment: ContentAlignment) -> Self {
        self.justify_content = alignment;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OverlayAlignment {
    #[default]
    Stretch,
    Start,
    End,
    Center,
}

/// Overlay/stack container policy. Children share the same container space and
/// are aligned independently from paint ordering.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OverlayContainerStyle {
    horizontal: OverlayAlignment,
    vertical: OverlayAlignment,
}

impl OverlayContainerStyle {
    #[must_use]
    pub const fn new(horizontal: OverlayAlignment, vertical: OverlayAlignment) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    #[must_use]
    pub const fn horizontal(self) -> OverlayAlignment {
        self.horizontal
    }

    #[must_use]
    pub const fn vertical(self) -> OverlayAlignment {
        self.vertical
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LayoutContainer {
    #[default]
    Block,
    Flex(FlexContainerStyle),
    Grid(GridContainerStyle),
    Overlay(OverlayContainerStyle),
}

/// Logical insets for an absolutely positioned node. `Auto` leaves an edge
/// unconstrained on that axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutInsets {
    top: LayoutOffset,
    right: LayoutOffset,
    bottom: LayoutOffset,
    left: LayoutOffset,
}

impl LayoutInsets {
    #[must_use]
    pub const fn new(
        top: LayoutOffset,
        right: LayoutOffset,
        bottom: LayoutOffset,
        left: LayoutOffset,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    #[must_use]
    pub const fn top(self) -> LayoutOffset {
        self.top
    }

    #[must_use]
    pub const fn right(self) -> LayoutOffset {
        self.right
    }

    #[must_use]
    pub const fn bottom(self) -> LayoutOffset {
        self.bottom
    }

    #[must_use]
    pub const fn left(self) -> LayoutOffset {
        self.left
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LayoutPosition {
    #[default]
    Flow,
    Absolute(LayoutInsets),
}

/// Logical overflow behavior independent of platform scrolling mechanics.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OverflowPolicy {
    #[default]
    Visible,
    Clip,
    Scroll,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OverflowStyle {
    horizontal: OverflowPolicy,
    vertical: OverflowPolicy,
}

impl OverflowStyle {
    #[must_use]
    pub const fn new(horizontal: OverflowPolicy, vertical: OverflowPolicy) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    #[must_use]
    pub const fn all(policy: OverflowPolicy) -> Self {
        Self::new(policy, policy)
    }

    #[must_use]
    pub const fn horizontal(self) -> OverflowPolicy {
        self.horizontal
    }

    #[must_use]
    pub const fn vertical(self) -> OverflowPolicy {
        self.vertical
    }
}

/// Authored structural layout intent for one element.
///
/// Theme/cascade-resolved values such as padding and typography remain in the
/// accepted M8A style authority. This value owns structural layout intent only:
/// sizing, container algorithm, sibling gaps, parent-item participation,
/// placement, margin, and logical overflow behavior. Width/height/min/max
/// describe the border box; M8C has no independent authored border-width model.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutStyle {
    container: LayoutContainer,
    width: LayoutDimension,
    height: LayoutDimension,
    min_width: LayoutBound,
    min_height: LayoutBound,
    max_width: LayoutBound,
    max_height: LayoutBound,
    margin: EdgeInsets,
    gap: LayoutGap,
    flex_item: FlexItemStyle,
    grid_item: GridItemStyle,
    position: LayoutPosition,
    overflow: OverflowStyle,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            container: LayoutContainer::Block,
            width: LayoutDimension::Auto,
            height: LayoutDimension::Auto,
            min_width: LayoutBound::Auto,
            min_height: LayoutBound::Auto,
            max_width: LayoutBound::Auto,
            max_height: LayoutBound::Auto,
            margin: EdgeInsets::ZERO,
            gap: LayoutGap::ZERO,
            flex_item: FlexItemStyle::default(),
            grid_item: GridItemStyle::default(),
            position: LayoutPosition::Flow,
            overflow: OverflowStyle::default(),
        }
    }
}

impl LayoutStyle {
    #[must_use]
    pub const fn container(&self) -> &LayoutContainer {
        &self.container
    }

    #[must_use]
    pub const fn width(&self) -> LayoutDimension {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> LayoutDimension {
        self.height
    }

    #[must_use]
    pub const fn min_width(&self) -> LayoutBound {
        self.min_width
    }

    #[must_use]
    pub const fn min_height(&self) -> LayoutBound {
        self.min_height
    }

    #[must_use]
    pub const fn max_width(&self) -> LayoutBound {
        self.max_width
    }

    #[must_use]
    pub const fn max_height(&self) -> LayoutBound {
        self.max_height
    }

    #[must_use]
    pub const fn margin(&self) -> EdgeInsets {
        self.margin
    }

    #[must_use]
    pub const fn gap(&self) -> LayoutGap {
        self.gap
    }

    #[must_use]
    pub const fn flex_item(&self) -> FlexItemStyle {
        self.flex_item
    }

    #[must_use]
    pub const fn grid_item(&self) -> GridItemStyle {
        self.grid_item
    }

    #[must_use]
    pub const fn position(&self) -> LayoutPosition {
        self.position
    }

    #[must_use]
    pub const fn overflow(&self) -> OverflowStyle {
        self.overflow
    }

    #[must_use]
    pub fn with_container(mut self, container: LayoutContainer) -> Self {
        self.container = container;
        self
    }

    #[must_use]
    pub const fn with_width(mut self, width: LayoutDimension) -> Self {
        self.width = width;
        self
    }

    #[must_use]
    pub const fn with_height(mut self, height: LayoutDimension) -> Self {
        self.height = height;
        self
    }

    #[must_use]
    pub const fn with_min_width(mut self, width: LayoutBound) -> Self {
        self.min_width = width;
        self
    }

    #[must_use]
    pub const fn with_min_height(mut self, height: LayoutBound) -> Self {
        self.min_height = height;
        self
    }

    #[must_use]
    pub const fn with_max_width(mut self, width: LayoutBound) -> Self {
        self.max_width = width;
        self
    }

    #[must_use]
    pub const fn with_max_height(mut self, height: LayoutBound) -> Self {
        self.max_height = height;
        self
    }

    #[must_use]
    pub const fn with_margin(mut self, margin: EdgeInsets) -> Self {
        self.margin = margin;
        self
    }

    #[must_use]
    pub fn with_gap(mut self, gap: impl Into<LogicalLength>) -> Self {
        self.gap = LayoutGap::all(gap.into());
        self
    }

    #[must_use]
    pub const fn with_gaps(mut self, gap: LayoutGap) -> Self {
        self.gap = gap;
        self
    }

    #[must_use]
    pub const fn with_flex_item(mut self, flex_item: FlexItemStyle) -> Self {
        self.flex_item = flex_item;
        self
    }

    #[must_use]
    pub const fn with_grid_item(mut self, grid_item: GridItemStyle) -> Self {
        self.grid_item = grid_item;
        self
    }

    #[must_use]
    pub const fn with_position(mut self, position: LayoutPosition) -> Self {
        self.position = position;
        self
    }

    #[must_use]
    pub const fn with_overflow(mut self, overflow: OverflowStyle) -> Self {
        self.overflow = overflow;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FlexBasis, FlexContainerStyle, FlexDirection, FlexItemStyle, GridAxisPlacement,
        GridContainerStyle, GridLine, GridLineError, GridSpan, GridSpanError, GridTrack,
        GridTrackMax, GridTrackMin, ItemAlignment, LayoutContainer, LayoutDimension, LayoutFactor,
        LayoutFactorError, LayoutGap, LayoutStyle, OverflowPolicy, OverflowStyle,
    };
    use crate::LogicalLength;

    #[test]
    fn layout_factor_validation_is_explicit() {
        assert_eq!(
            LayoutFactor::new(f32::NAN),
            Err(LayoutFactorError::NotFinite)
        );
        assert_eq!(
            LayoutFactor::new(f32::INFINITY),
            Err(LayoutFactorError::NotFinite)
        );
        assert_eq!(LayoutFactor::new(-1.0), Err(LayoutFactorError::Negative));
        assert_eq!(LayoutFactor::new(-0.0), Ok(LayoutFactor::ZERO));
        assert_eq!(LayoutFactor::new(1.5).map(LayoutFactor::get), Ok(1.5));
    }

    #[test]
    fn grid_line_and_span_are_one_based_non_zero_values() {
        assert_eq!(GridLine::new(0), Err(GridLineError));
        assert_eq!(GridSpan::new(0), Err(GridSpanError));
        assert_eq!(GridLine::new(2).map(GridLine::get), Ok(2));
        assert_eq!(GridSpan::new(3).map(GridSpan::get), Ok(3));
    }

    #[test]
    fn layout_style_keeps_structural_facts_without_dependency_types() {
        let half = LayoutFactor::new(0.5).unwrap_or_default();
        let two = LayoutFactor::new(2.0).unwrap_or_default();
        let column = FlexContainerStyle::default()
            .with_direction(FlexDirection::Column)
            .with_align_items(ItemAlignment::Baseline);
        let gap = LayoutGap::new(LogicalLength::from(4_u16), LogicalLength::from(8_u16));
        let layout = LayoutStyle::default()
            .with_container(LayoutContainer::Flex(column))
            .with_width(LayoutDimension::percent(half))
            .with_gaps(gap)
            .with_flex_item(
                FlexItemStyle::default()
                    .with_grow(two)
                    .with_basis(FlexBasis::Content),
            )
            .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll));

        assert_eq!(layout.width(), LayoutDimension::percent(half));
        assert_eq!(layout.gap(), gap);
        assert_eq!(layout.flex_item().grow(), two);
        assert_eq!(layout.flex_item().basis(), FlexBasis::Content);
        assert_eq!(layout.overflow().vertical(), OverflowPolicy::Scroll);
        assert!(matches!(layout.container(), LayoutContainer::Flex(_)));
    }

    #[test]
    fn grid_track_minimum_cannot_encode_fractional_breadth() {
        let two = LayoutFactor::new(2.0).unwrap_or_default();
        let fractional = GridTrack::fraction(two);
        assert_eq!(fractional.min(), GridTrackMin::Auto);
        assert_eq!(fractional.max(), GridTrackMax::Fraction(two));
    }

    #[test]
    fn grid_tracks_and_placement_are_explicit() {
        let two = LayoutFactor::new(2.0).unwrap_or_default();
        let columns = [
            GridTrack::fraction(LayoutFactor::ONE),
            GridTrack::fraction(two),
        ];
        let grid = GridContainerStyle::new(columns, [GridTrack::auto()]);
        let placement = GridAxisPlacement::new(
            GridLine::new(2).ok(),
            GridSpan::new(2).unwrap_or(GridSpan::ONE),
        );

        assert_eq!(grid.columns().len(), 2);
        assert_eq!(grid.rows(), [GridTrack::auto()]);
        assert_eq!(placement.start().map(GridLine::get), Some(2));
        assert_eq!(placement.span().get(), 2);
    }
}
