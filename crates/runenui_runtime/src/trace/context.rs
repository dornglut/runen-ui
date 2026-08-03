use runenui_core::{
    CompositionGeneration, InputDeviceId, PointerDeviceKind, PointerId, PointerPhase, SurfaceId,
};

use crate::{ReconciliationGeneration, SurfacePhase};

use super::{TraceSurfaceSnapshotKind, TraceTarget};

/// Routed event family retained by the normalized M4 trace schema.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceEventFamily {
    SemanticCommand,
    Pointer,
    PointerBoundary,
    PointerCapture,
    Focus,
    Keyboard,
    CommittedText,
    Composition,
}

/// Common facts for one canonical routed-event transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceEventContext {
    family: TraceEventFamily,
    cancelable: bool,
}

impl TraceEventContext {
    /// Returns the normalized event family.
    #[must_use]
    pub const fn family(self) -> TraceEventFamily {
        self.family
    }

    /// Returns whether framework default behavior could be prevented.
    #[must_use]
    pub const fn is_cancelable(self) -> bool {
        self.cancelable
    }
}

/// Exact displayed-surface facts retained by one canonical trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceSurfaceContext {
    surface_id: SurfaceId,
    coordinate_revision: u64,
    hit_test_generation: u64,
    snapshot: Option<TraceSurfaceSnapshotKind>,
}

impl TraceSurfaceContext {
    /// Returns the exact logical surface identity.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }

    /// Returns the exact coordinate-space revision.
    #[must_use]
    pub const fn coordinate_revision(&self) -> u64 {
        self.coordinate_revision
    }

    /// Returns the exact displayed hit-test generation.
    #[must_use]
    pub const fn hit_test_generation(&self) -> u64 {
        self.hit_test_generation
    }

    /// Returns whether the accepted snapshot was current or retained.
    #[must_use]
    pub const fn snapshot(&self) -> Option<TraceSurfaceSnapshotKind> {
        self.snapshot
    }
}

/// Pointer/device facts shared by pointer-specific trace records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePointerContext {
    pointer_id: PointerId,
    device_id: Option<InputDeviceId>,
    device_kind: PointerDeviceKind,
    phase: Option<PointerPhase>,
}

impl TracePointerContext {
    /// Returns the exact pointer-stream identity.
    #[must_use]
    pub const fn pointer_id(&self) -> &PointerId {
        &self.pointer_id
    }

    /// Returns the optional host-neutral device identity.
    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }

    /// Returns the normalized pointer device category.
    #[must_use]
    pub const fn device_kind(&self) -> PointerDeviceKind {
        self.device_kind
    }

    /// Returns the ordinary pointer phase when this record owns one.
    #[must_use]
    pub const fn phase(&self) -> Option<PointerPhase> {
        self.phase
    }
}

/// Exact composition lifetime retained without exposing text or preedit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceCompositionContext {
    generation: CompositionGeneration,
    device_id: Option<InputDeviceId>,
}

impl TraceCompositionContext {
    /// Returns the opaque exact composition generation.
    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        &self.generation
    }

    /// Returns the optional host-neutral device identity from composition start.
    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }
}

/// Redacted size facts for committed text or composition preedit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceTextMetrics {
    bytes: usize,
    scalars: usize,
}

impl TraceTextMetrics {
    /// Returns the UTF-8 byte length without retaining text.
    #[must_use]
    pub const fn bytes(self) -> usize {
        self.bytes
    }

    /// Returns the Unicode scalar count without retaining text.
    #[must_use]
    pub const fn scalars(self) -> usize {
        self.scalars
    }
}

/// Redacted checked byte and scalar range into one composition preedit value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceCompositionRange {
    byte_start: usize,
    byte_end: usize,
    scalar_start: usize,
    scalar_end: usize,
}

impl TraceCompositionRange {
    /// Returns the inclusive UTF-8 byte start.
    #[must_use]
    pub const fn byte_start(self) -> usize {
        self.byte_start
    }

    /// Returns the exclusive UTF-8 byte end.
    #[must_use]
    pub const fn byte_end(self) -> usize {
        self.byte_end
    }

    /// Returns the inclusive Unicode scalar start.
    #[must_use]
    pub const fn scalar_start(self) -> usize {
        self.scalar_start
    }

    /// Returns the exclusive Unicode scalar end.
    #[must_use]
    pub const fn scalar_end(self) -> usize {
        self.scalar_end
    }
}

/// Queue-source classification of one accepted application action.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceActionCategory {
    DirectSubmission,
    RoutedCommand,
    ApplicationEffect,
}

/// Redacted action identity that never retains or formats the payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceActionIdentity {
    type_name: &'static str,
    category: TraceActionCategory,
}

impl TraceActionIdentity {
    /// Returns the Rust action type name without retaining a payload.
    #[must_use]
    pub const fn type_name(self) -> &'static str {
        self.type_name
    }

    /// Returns how the action entered the canonical queue.
    #[must_use]
    pub const fn category(self) -> TraceActionCategory {
        self.category
    }
}

/// Ordered exact routed path and related endpoint owned by one trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRouteSnapshot {
    targets: Vec<TraceTarget>,
    related_target: Option<TraceTarget>,
}

impl TraceRouteSnapshot {
    /// Returns the immutable root-to-target route.
    #[must_use]
    pub const fn targets(&self) -> &[TraceTarget] {
        self.targets.as_slice()
    }

    /// Returns the exact related endpoint for boundary-style delivery.
    #[must_use]
    pub const fn related_target(&self) -> Option<&TraceTarget> {
        self.related_target.as_ref()
    }
}

/// Ordered physical pointer path owned by one canonical trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePointerPath {
    targets: Vec<TraceTarget>,
}

impl TracePointerPath {
    /// Returns the immutable root-to-physical-target path.
    #[must_use]
    pub const fn targets(&self) -> &[TraceTarget] {
        self.targets.as_slice()
    }
}

/// Exact previous/current target endpoints for one interaction transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceTargetTransition {
    previous: Option<TraceTarget>,
    current: Option<TraceTarget>,
}

impl TraceTargetTransition {
    /// Returns the previous exact target endpoint.
    #[must_use]
    pub const fn previous(&self) -> Option<&TraceTarget> {
        self.previous.as_ref()
    }

    /// Returns the current exact target endpoint.
    #[must_use]
    pub const fn current(&self) -> Option<&TraceTarget> {
        self.current.as_ref()
    }
}

/// Whether a required routed notification reached callbacks or was suppressed.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceDeliveryOutcome {
    Delivered,
    Suppressed,
}

/// Renderer-facing publication identity that closes an M4 causal chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePublicationContext {
    surface: TraceSurfaceContext,
    reconciliation_generation: ReconciliationGeneration,
    node_count: usize,
    executed_phases: Vec<SurfacePhase>,
}

impl TracePublicationContext {
    /// Returns the exact displayed-surface identity.
    #[must_use]
    pub const fn surface(&self) -> &TraceSurfaceContext {
        &self.surface
    }

    /// Returns the mounted reconciliation generation consumed by publication.
    #[must_use]
    pub const fn reconciliation_generation(&self) -> ReconciliationGeneration {
        self.reconciliation_generation
    }

    /// Returns the number of published mounted nodes.
    #[must_use]
    pub const fn node_count(&self) -> usize {
        self.node_count
    }

    /// Returns the phases that actually executed for this publication.
    #[must_use]
    pub const fn executed_phases(&self) -> &[SurfacePhase] {
        self.executed_phases.as_slice()
    }
}

/// Typed normalized context attached to one canonical trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceContext {
    event: Option<TraceEventContext>,
    surface: Option<TraceSurfaceContext>,
    pointer: Option<TracePointerContext>,
    composition: Option<TraceCompositionContext>,
    text_metrics: Option<TraceTextMetrics>,
    composition_range: Option<TraceCompositionRange>,
    route: Option<TraceRouteSnapshot>,
    physical_path: Option<TracePointerPath>,
    target_transition: Option<TraceTargetTransition>,
    action: Option<TraceActionIdentity>,
    publication: Option<TracePublicationContext>,
    delivery: Option<TraceDeliveryOutcome>,
}

impl TraceContext {
    pub(crate) const fn empty() -> Self {
        Self {
            event: None,
            surface: None,
            pointer: None,
            composition: None,
            text_metrics: None,
            composition_range: None,
            route: None,
            physical_path: None,
            target_transition: None,
            action: None,
            publication: None,
            delivery: None,
        }
    }

    /// Returns normalized event family and cancelability facts.
    #[must_use]
    pub const fn event(&self) -> Option<TraceEventContext> {
        self.event
    }

    /// Returns exact displayed-surface facts.
    #[must_use]
    pub const fn surface(&self) -> Option<&TraceSurfaceContext> {
        self.surface.as_ref()
    }

    /// Returns exact pointer/device facts.
    #[must_use]
    pub const fn pointer(&self) -> Option<&TracePointerContext> {
        self.pointer.as_ref()
    }

    /// Returns exact composition lifetime facts.
    #[must_use]
    pub const fn composition(&self) -> Option<&TraceCompositionContext> {
        self.composition.as_ref()
    }

    /// Returns redacted text or preedit size facts.
    #[must_use]
    pub const fn text_metrics(&self) -> Option<TraceTextMetrics> {
        self.text_metrics
    }

    /// Returns the checked redacted composition range.
    #[must_use]
    pub const fn composition_range(&self) -> Option<TraceCompositionRange> {
        self.composition_range
    }

    /// Returns the ordered routed path and related endpoint.
    #[must_use]
    pub const fn route(&self) -> Option<&TraceRouteSnapshot> {
        self.route.as_ref()
    }

    /// Returns the ordered physical pointer path.
    #[must_use]
    pub const fn physical_path(&self) -> Option<&TracePointerPath> {
        self.physical_path.as_ref()
    }

    /// Returns exact previous/current transition endpoints.
    #[must_use]
    pub const fn target_transition(&self) -> Option<&TraceTargetTransition> {
        self.target_transition.as_ref()
    }

    /// Returns redacted application-action identity.
    #[must_use]
    pub const fn action(&self) -> Option<TraceActionIdentity> {
        self.action
    }

    /// Returns renderer-facing publication identity.
    #[must_use]
    pub const fn publication(&self) -> Option<&TracePublicationContext> {
        self.publication.as_ref()
    }

    /// Returns explicit delivery or suppression outcome.
    #[must_use]
    pub const fn delivery(&self) -> Option<TraceDeliveryOutcome> {
        self.delivery
    }
}
