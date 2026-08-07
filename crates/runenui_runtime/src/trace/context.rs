use runenui_core::{SurfaceId, SurfaceInputContext};

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
    pub(crate) fn requested(context: &SurfaceInputContext) -> Self {
        Self {
            surface_id: context.surface_id().clone(),
            coordinate_revision: context.coordinate_revision(),
            hit_test_generation: context.hit_test_generation(),
            snapshot: None,
        }
    }

    pub(crate) fn accepted(
        context: &SurfaceInputContext,
        snapshot: TraceSurfaceSnapshotKind,
    ) -> Self {
        Self {
            surface_id: context.surface_id().clone(),
            coordinate_revision: context.coordinate_revision(),
            hit_test_generation: context.hit_test_generation(),
            snapshot: Some(snapshot),
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

/// Ordered exact routed path and related endpoint owned by one trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRouteSnapshot {
    targets: Vec<TraceTarget>,
    related_target: Option<TraceTarget>,
}

impl TraceRouteSnapshot {
    pub(crate) const fn new(
        targets: Vec<TraceTarget>,
        related_target: Option<TraceTarget>,
    ) -> Self {
        Self {
            targets,
            related_target,
        }
    }

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

#[derive(Clone, Debug, Eq, PartialEq)]
enum TraceContextData {
    Routed {
        event: TraceEventContext,
        route: Option<TraceRouteSnapshot>,
    },
    Surface(TraceSurfaceContext),
    Publication(TracePublicationContext),
}

/// Typed normalized context attached to one canonical trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceContext {
    data: Option<Box<TraceContextData>>,
}

impl TraceContext {
    pub(in crate::trace) const fn empty() -> Self {
        Self { data: None }
    }

    pub(crate) fn routed_event(event: TraceEventContext) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Routed { event, route: None })),
        }
    }

    pub(crate) fn routed_snapshot(event: TraceEventContext, route: TraceRouteSnapshot) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Routed {
                event,
                route: Some(route),
            })),
        }
    }

    pub(crate) fn surface_record(surface: TraceSurfaceContext) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Surface(surface))),
        }
    }

    pub(crate) fn publication_record(publication: TracePublicationContext) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Publication(publication))),
        }
    }

    /// Returns normalized event family and cancelability facts.
    #[must_use]
    pub fn event(&self) -> Option<TraceEventContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Routed { event, .. }) => Some(*event),
            Some(TraceContextData::Surface(_) | TraceContextData::Publication(_)) | None => None,
        }
    }

    /// Returns exact displayed-surface facts.
    #[must_use]
    pub fn surface(&self) -> Option<&TraceSurfaceContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Surface(surface)) => Some(surface),
            Some(TraceContextData::Publication(publication)) => Some(publication.surface()),
            Some(TraceContextData::Routed { .. }) | None => None,
        }
    }

    /// Returns the ordered routed path and related endpoint.
    #[must_use]
    pub fn route(&self) -> Option<&TraceRouteSnapshot> {
        match self.data.as_deref() {
            Some(TraceContextData::Routed { route, .. }) => route.as_ref(),
            Some(TraceContextData::Surface(_) | TraceContextData::Publication(_)) | None => None,
        }
    }

    /// Returns renderer-facing publication identity.
    #[must_use]
    pub fn publication(&self) -> Option<&TracePublicationContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Publication(publication)) => Some(publication),
            Some(TraceContextData::Routed { .. } | TraceContextData::Surface(_)) | None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TraceContext;

    #[test]
    fn empty_context_exposes_no_normalized_facts() {
        let context = TraceContext::empty();

        assert_eq!(context.event(), None);
        assert_eq!(context.surface(), None);
        assert_eq!(context.route(), None);
        assert_eq!(context.publication(), None);
    }
}
