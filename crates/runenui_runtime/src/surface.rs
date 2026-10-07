//! Surface layout/debug products and canonical scene planning.
#![allow(clippy::redundant_pub_crate)]
//!
//! `SurfaceFrame` remains an aligned layout/debug snapshot. Canonical renderer
//! paint and pointer-hit authority live in `PaintScene`/`PaintPublication` and
//! `HitTestScene`, not in this debug product.

mod cache;
mod context;
mod interaction;
mod motion;
mod planning;
#[cfg(feature = "internal-test-seams")]
pub(crate) mod profile;
mod resolve;
mod taffy_layout;
#[cfg(test)]
mod tests;
mod transaction;

use std::sync::Arc;

pub(crate) use cache::SurfaceCache;
pub(crate) use cache::TextEditingPaintInputs;
pub use cache::{SurfacePhase, SurfacePhaseReport};
pub use context::{RasterScale, RasterScaleError, SurfaceBuildContext};
pub(crate) use interaction::{SurfaceInteractionProjection, SurfaceScrollProjection};
pub(crate) use motion::MotionPlanningFailure;
#[cfg(test)]
use planning::plan_mounted_surface_cached;
#[cfg(test)]
use planning::publish_mounted_surface_cached;
pub(crate) use planning::{SurfacePlanningError, plan_mounted_surface_cached_with_text};
#[cfg(feature = "internal-test-seams")]
#[doc(hidden)]
pub use profile::SurfacePublicationTestProfile;
pub(crate) use transaction::{
    DisplayedScrollMetrics, DisplayedTextTarget, PlannedSurfacePublication,
    SurfacePublicationCommit,
};

use runenui_core::{
    ComputedStyle, ElementId, LogicalRect, LogicalSize, ResourceRef, ScrollControlBinding,
    ScrollControlSnapshot, SurfacePresentationPlacement, WidgetDiagnostic, WidgetMeasureInput,
    WidgetTypeId,
};
use runenui_text::{TextConstraints, TextLayoutDecision};

use crate::style_debug::SurfaceStyleReport;
use crate::{LayoutConstraints, MountedNodeId};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ScrollControlProjectionLookup {
    Unavailable,
    Unbound,
    Bound {
        owner: MountedNodeId,
        binding: ScrollControlBinding,
        snapshot: ScrollControlSnapshot,
    },
}

/// Surface-owned live motion state. Lifecycle details stay private to surface planning.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SurfaceMotionStore(motion::MotionStore);

impl SurfaceMotionStore {
    pub(crate) fn retire_owner(&mut self, owner: &MountedNodeId) {
        self.0.retire_owner(owner);
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}

/// Scheduling facts produced by one accepted motion candidate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SurfaceMotionActivity(motion::MotionActivity);

impl SurfaceMotionActivity {
    pub(crate) const fn continuous_redraw(self) -> bool {
        self.0.continuous_redraw()
    }

    pub(crate) const fn followup_publication(self) -> bool {
        self.0.followup_publication()
    }

    pub(crate) const fn next_deadline(self) -> Option<runenui_core::MonotonicInstant> {
        self.0.next_deadline()
    }
}

/// Runtime-resolved same-surface placement for one presentation root.
///
/// This is immutable publication evidence, not application state. The mounted
/// root remains the lifetime identity and ordinary logical ancestry is unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePresentationSnapshot {
    candidate_index: usize,
    placement: SurfacePresentationPlacement,
    anchor_bounds: LogicalRect,
    placed_bounds: LogicalRect,
    visible_bounds: LogicalRect,
}

impl SurfacePresentationSnapshot {
    pub(crate) const fn new(
        candidate_index: usize,
        placement: SurfacePresentationPlacement,
        anchor_bounds: LogicalRect,
        placed_bounds: LogicalRect,
        visible_bounds: LogicalRect,
    ) -> Self {
        Self {
            candidate_index,
            placement,
            anchor_bounds,
            placed_bounds,
            visible_bounds,
        }
    }

    /// Returns the authored candidate index selected by deterministic fallback.
    #[must_use]
    pub const fn candidate_index(self) -> usize {
        self.candidate_index
    }

    /// Returns the exact authored candidate that resolved this placement.
    #[must_use]
    pub const fn placement(self) -> SurfacePresentationPlacement {
        self.placement
    }

    /// Returns the exact surface-logical anchor bounds used for this publication.
    #[must_use]
    pub const fn anchor_bounds(self) -> LogicalRect {
        self.anchor_bounds
    }

    /// Returns the untransformed measured presentation rectangle after fallback/clamp.
    #[must_use]
    pub const fn placed_bounds(self) -> LogicalRect {
        self.placed_bounds
    }

    /// Returns the final surface-clipped projected root bounds.
    #[must_use]
    pub const fn visible_bounds(self) -> LogicalRect {
        self.visible_bounds
    }
}

/// One ordered node in the non-renderer layout/debug surface frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceNode {
    id: MountedNodeId,
    parent: Option<MountedNodeId>,
    authored_id: Option<ElementId>,
    bounds: LogicalRect,
    surface_presentation: Option<SurfacePresentationSnapshot>,
    widget_debug: SurfaceWidgetDebug,
    computed_style: ComputedStyle,
}

#[derive(Clone, Debug, PartialEq)]
struct SurfaceWidgetDebug {
    widget_type_id: WidgetTypeId,
    diagnostics: Vec<WidgetDiagnostic>,
}

impl SurfaceNode {
    #[must_use]
    fn new(
        id: MountedNodeId,
        parent: Option<MountedNodeId>,
        authored_id: Option<ElementId>,
        bounds: LogicalRect,
        surface_presentation: Option<SurfacePresentationSnapshot>,
        widget_debug: SurfaceWidgetDebug,
        computed_style: &ComputedStyle,
    ) -> Self {
        Self {
            id,
            parent,
            authored_id,
            bounds,
            surface_presentation,
            widget_debug,
            computed_style: computed_style.clone(),
        }
    }

    #[must_use]
    pub const fn id(&self) -> &MountedNodeId {
        &self.id
    }

    #[must_use]
    pub const fn parent(&self) -> Option<&MountedNodeId> {
        self.parent.as_ref()
    }

    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }

    #[must_use]
    pub const fn bounds(&self) -> LogicalRect {
        self.bounds
    }

    /// Returns runtime-resolved same-surface presentation placement when this
    /// exact node is a published presentation root.
    #[must_use]
    pub const fn surface_presentation(&self) -> Option<SurfacePresentationSnapshot> {
        self.surface_presentation
    }

    /// Returns process-local widget identity for diagnostics only.
    #[must_use]
    pub const fn widget_type_id(&self) -> WidgetTypeId {
        self.widget_debug.widget_type_id
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &[WidgetDiagnostic] {
        self.widget_debug.diagnostics.as_slice()
    }

    #[must_use]
    pub const fn computed_style(&self) -> &ComputedStyle {
        &self.computed_style
    }
}

/// Non-renderer layout/debug snapshot for one UI tree.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFrame {
    size: LogicalSize,
    nodes: Vec<SurfaceNode>,
}

impl SurfaceFrame {
    #[must_use]
    pub(crate) const fn new(size: LogicalSize, nodes: Vec<SurfaceNode>) -> Self {
        Self { size, nodes }
    }

    #[must_use]
    pub const fn size(&self) -> LogicalSize {
        self.size
    }

    #[must_use]
    pub const fn nodes(&self) -> &[SurfaceNode] {
        self.nodes.as_slice()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn node(&self, id: &MountedNodeId) -> Option<&SurfaceNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    #[must_use]
    pub fn root(&self) -> Option<&SurfaceNode> {
        self.nodes.first()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutOverflow {
    width: bool,
    height: bool,
}

impl LayoutOverflow {
    const fn new(width: bool, height: bool) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(&self) -> bool {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> bool {
        self.height
    }

    #[must_use]
    pub const fn any(&self) -> bool {
        self.width || self.height
    }
}

/// One successful production text-measurement call that contributed to a retained layout product.
///
/// This is a read-only diagnostic correlation record. It does not own text state,
/// layout authority, or shaped-resource lifetime. `retained_resource_refs` is
/// populated only for the final callback whose immutable text state was retained
/// for paint publication.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceTextMeasurementRecord {
    input: WidgetMeasureInput,
    text_constraints: TextConstraints,
    decision: TextLayoutDecision,
    measured_size: LogicalSize,
    retained_for_paint: bool,
    retained_resource_refs: Vec<ResourceRef>,
}

impl SurfaceTextMeasurementRecord {
    pub(crate) const fn new(
        input: WidgetMeasureInput,
        text_constraints: TextConstraints,
        decision: TextLayoutDecision,
        measured_size: LogicalSize,
        retained_for_paint: bool,
        retained_resource_refs: Vec<ResourceRef>,
    ) -> Self {
        Self {
            input,
            text_constraints,
            decision,
            measured_size,
            retained_for_paint,
            retained_resource_refs,
        }
    }

    /// Returns the exact renderer-neutral known/available-space facts passed to the text leaf.
    #[must_use]
    pub const fn input(&self) -> WidgetMeasureInput {
        self.input
    }

    /// Returns the exact text-specific constraint projection used by this measurement.
    #[must_use]
    pub const fn text_constraints(&self) -> TextConstraints {
        self.text_constraints
    }

    /// Returns whether private text work was reused, re-line-broken, or reshaped.
    #[must_use]
    pub const fn decision(&self) -> TextLayoutDecision {
        self.decision
    }

    /// Returns the exact logical text size returned to the layout algorithm.
    #[must_use]
    pub const fn measured_size(&self) -> LogicalSize {
        self.measured_size
    }

    /// Returns whether this call supplied the immutable text state retained for paint.
    #[must_use]
    pub const fn retained_for_paint(&self) -> bool {
        self.retained_for_paint
    }

    /// Returns the ordered shaped-resource identities from the retained artifact.
    ///
    /// Non-retained measurement calls return an empty slice. These identities are
    /// diagnostic copies only; resource lifetime remains owned by the retained text
    /// state and paint publication.
    #[must_use]
    pub const fn retained_resource_refs(&self) -> &[ResourceRef] {
        self.retained_resource_refs.as_slice()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceLayoutNode {
    id: MountedNodeId,
    parent: Option<MountedNodeId>,
    authored_id: Option<ElementId>,
    outer_constraints: LayoutConstraints,
    content_constraints: LayoutConstraints,
    desired_content_size: LogicalSize,
    desired_outer_size: LogicalSize,
    constrained_outer_size: LogicalSize,
    layout_extent: LogicalSize,
    content_extent: LogicalSize,
    scrollable_extent: LogicalSize,
    scroll_viewport_extent: LogicalSize,
    overflow: LayoutOverflow,
    text_measurements: Vec<SurfaceTextMeasurementRecord>,
    diagnostics: Vec<WidgetDiagnostic>,
}

impl SurfaceLayoutNode {
    const fn new(
        id: MountedNodeId,
        parent: Option<MountedNodeId>,
        authored_id: Option<ElementId>,
        constraints: [LayoutConstraints; 2],
        sizes: [LogicalSize; 3],
        overflow: LayoutOverflow,
    ) -> Self {
        Self {
            id,
            parent,
            authored_id,
            outer_constraints: constraints[0],
            content_constraints: constraints[1],
            desired_content_size: sizes[0],
            desired_outer_size: sizes[1],
            constrained_outer_size: sizes[2],
            layout_extent: sizes[2],
            content_extent: sizes[0],
            scrollable_extent: sizes[0],
            scroll_viewport_extent: sizes[2],
            overflow,
            text_measurements: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn with_text_measurements(
        mut self,
        text_measurements: Vec<SurfaceTextMeasurementRecord>,
    ) -> Self {
        self.text_measurements = text_measurements;
        self
    }

    fn with_diagnostics(mut self, diagnostics: Vec<WidgetDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    const fn with_extents(
        mut self,
        layout_extent: LogicalSize,
        content_extent: LogicalSize,
        scrollable_extent: LogicalSize,
        scroll_viewport_extent: LogicalSize,
    ) -> Self {
        self.layout_extent = layout_extent;
        self.content_extent = content_extent;
        self.scrollable_extent = scrollable_extent;
        self.scroll_viewport_extent = scroll_viewport_extent;
        self
    }

    fn replace_derived_chrome_extent(&mut self, extent: LogicalSize) {
        self.desired_content_size = extent;
        self.desired_outer_size = extent;
        self.constrained_outer_size = extent;
        self.layout_extent = extent;
        self.content_extent = extent;
        self.scrollable_extent = extent;
        self.scroll_viewport_extent = extent;
        self.overflow = LayoutOverflow::default();
        self.text_measurements.clear();
    }

    #[must_use]
    pub const fn id(&self) -> &MountedNodeId {
        &self.id
    }

    #[must_use]
    pub const fn parent(&self) -> Option<&MountedNodeId> {
        self.parent.as_ref()
    }

    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }

    #[must_use]
    pub const fn outer_constraints(&self) -> LayoutConstraints {
        self.outer_constraints
    }

    #[must_use]
    pub const fn content_constraints(&self) -> LayoutConstraints {
        self.content_constraints
    }

    #[must_use]
    pub const fn desired_content_size(&self) -> LogicalSize {
        self.desired_content_size
    }

    #[must_use]
    pub const fn desired_outer_size(&self) -> LogicalSize {
        self.desired_outer_size
    }

    #[must_use]
    pub const fn constrained_outer_size(&self) -> LogicalSize {
        self.constrained_outer_size
    }

    /// Returns the final border-box extent used by paint and hit testing.
    #[must_use]
    pub const fn layout_extent(&self) -> LogicalSize {
        self.layout_extent
    }

    /// Returns the runtime-computed content extent before clipping.
    #[must_use]
    pub const fn content_extent(&self) -> LogicalSize {
        self.content_extent
    }

    /// Returns the scrollable content extent after layout.
    #[must_use]
    pub const fn scrollable_extent(&self) -> LogicalSize {
        self.scrollable_extent
    }

    /// Returns the exact scroll viewport extent used by scrolling, clipping and
    /// bound scroll-control metrics. This may be smaller than the border-box
    /// layout extent when reserved viewport chrome is present.
    #[must_use]
    pub const fn scroll_viewport_extent(&self) -> LogicalSize {
        self.scroll_viewport_extent
    }

    #[must_use]
    pub const fn overflow(&self) -> LayoutOverflow {
        self.overflow
    }

    /// Returns the ordered successful text-measurement calls that produced this retained layout.
    #[must_use]
    pub const fn text_measurements(&self) -> &[SurfaceTextMeasurementRecord] {
        self.text_measurements.as_slice()
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &[WidgetDiagnostic] {
        self.diagnostics.as_slice()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceLayoutReport {
    nodes: Vec<SurfaceLayoutNode>,
}

impl SurfaceLayoutReport {
    const fn new(nodes: Vec<SurfaceLayoutNode>) -> Self {
        Self { nodes }
    }

    #[must_use]
    pub const fn nodes(&self) -> &[SurfaceLayoutNode] {
        self.nodes.as_slice()
    }

    const fn nodes_mut(&mut self) -> &mut [SurfaceLayoutNode] {
        self.nodes.as_mut_slice()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn node(&self, id: &MountedNodeId) -> Option<&SurfaceLayoutNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    #[must_use]
    pub fn root(&self) -> Option<&SurfaceLayoutNode> {
        self.nodes.first()
    }
}

/// Aligned layout/debug products from one surface preparation.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePublication {
    products: Arc<SurfacePublicationProducts>,
}

#[derive(Clone, Debug, PartialEq)]
struct SurfacePublicationProducts {
    frame: SurfaceFrame,
    style_report: SurfaceStyleReport,
    layout_report: SurfaceLayoutReport,
}

impl SurfacePublication {
    fn new(
        frame: SurfaceFrame,
        style_report: SurfaceStyleReport,
        layout_report: SurfaceLayoutReport,
    ) -> Self {
        Self {
            products: Arc::new(SurfacePublicationProducts {
                frame,
                style_report,
                layout_report,
            }),
        }
    }

    #[must_use]
    pub fn frame(&self) -> &SurfaceFrame {
        &self.products.frame
    }

    #[must_use]
    pub fn style_report(&self) -> &SurfaceStyleReport {
        &self.products.style_report
    }

    #[must_use]
    pub fn layout_report(&self) -> &SurfaceLayoutReport {
        &self.products.layout_report
    }

    #[must_use]
    pub fn into_parts(self) -> (SurfaceFrame, SurfaceStyleReport, SurfaceLayoutReport) {
        let products = match Arc::try_unwrap(self.products) {
            Ok(products) => products,
            Err(shared) => (*shared).clone(),
        };
        (
            products.frame,
            products.style_report,
            products.layout_report,
        )
    }

    #[cfg(test)]
    fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.products, &other.products)
    }
}
