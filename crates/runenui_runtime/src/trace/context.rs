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
    pub(crate) const fn new(family: TraceEventFamily, cancelable: bool) -> Self {
        Self { family, cancelable }
    }

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
    pub(crate) const fn new(
        surface_id: SurfaceId,
        coordinate_revision: u64,
        hit_test_generation: u64,
        snapshot: Option<TraceSurfaceSnapshotKind>,
    ) -> Self {
        Self {
            surface_id,
            coordinate_revision,
            hit_test_generation,
            snapshot,
        }
    }

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
    pub(crate) const fn new(
        pointer_id: PointerId,
        device_id: Option<InputDeviceId>,
        device_kind: PointerDeviceKind,
        phase: Option<PointerPhase>,
    ) -> Self {
        Self {
            pointer_id,
            device_id,
            device_kind,
            phase,
        }
    }

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
    pub(crate) const fn new(
        generation: CompositionGeneration,
        device_id: Option<InputDeviceId>,
    ) -> Self {
        Self {
            generation,
            device_id,
        }
    }

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

/// Redacted checked byte range into one composition preedit value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceCompositionRange {
    start: usize,
    end: usize,
}

impl TraceCompositionRange {
    pub(crate) const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the inclusive UTF-8 byte start.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Returns the exclusive UTF-8 byte end.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }
}

/// Origin category of one accepted application action.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceActionOrigin {
    DirectSubmission,
    RoutedCommand,
    ApplicationEffect,
}

/// Redacted action identity that never retains or formats the payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceActionIdentity {
    type_name: &'static str,
    origin: TraceActionOrigin,
}

impl TraceActionIdentity {
    pub(crate) const fn new(type_name: &'static str, origin: TraceActionOrigin) -> Self {
        Self { type_name, origin }
    }

    /// Returns the Rust action type name without retaining a payload.
    #[must_use]
    pub const fn type_name(self) -> &'static str {
        self.type_name
    }

    /// Returns how the action entered the canonical queue.
    #[must_use]
    pub const fn origin(self) -> TraceActionOrigin {
        self.origin
    }
}

/// Ordered exact route snapshot owned by one canonical trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRouteSnapshot {
    event: TraceEventContext,
    targets: Vec<TraceTarget>,
}

impl TraceRouteSnapshot {
    pub(crate) const fn new(event: TraceEventContext, targets: Vec<TraceTarget>) -> Self {
        Self { event, targets }
    }

    /// Returns the event family and cancelability facts.
    #[must_use]
    pub const fn event(&self) -> TraceEventContext {
        self.event
    }

    /// Returns the immutable root-to-target route.
    #[must_use]
    pub const fn targets(&self) -> &[TraceTarget] {
        self.targets.as_slice()
    }
}

/// Ordered physical pointer path owned by one canonical trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePointerPath {
    targets: Vec<TraceTarget>,
}

impl TracePointerPath {
    pub(crate) const fn new(targets: Vec<TraceTarget>) -> Self {
        Self { targets }
    }

    /// Returns the immutable root-to-physical-target path.
    #[must_use]
    pub const fn targets(&self) -> &[TraceTarget] {
        self.targets.as_slice()
    }
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
    pub(crate) const fn new(
        surface: TraceSurfaceContext,
        reconciliation_generation: ReconciliationGeneration,
        node_count: usize,
        executed_phases: Vec<SurfacePhase>,
    ) -> Self {
        Self {
            surface,
            reconciliation_generation,
            node_count,
            executed_phases,
        }
    }

    /// Returns the exact displayed-surface identity.
    #[must_use]
    pub const fn surface(&self) -> &TraceSurfaceContext {
        &self.surface
    }

    /// Returns the mounted reconciliation generation consumed by publication.
    #[must_use]
    pub const fn reconciliation_generation(self) -> ReconciliationGeneration {
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
