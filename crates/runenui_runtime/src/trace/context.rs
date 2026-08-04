use runenui_core::{
    CompositionGeneration, InputDeviceId, PointerDeviceKind, PointerId, PointerPhase, SurfaceId,
    SurfaceInputContext,
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

/// Pointer/device facts shared by pointer-specific trace records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePointerContext {
    pointer_id: PointerId,
    device_id: Option<InputDeviceId>,
    device_kind: PointerDeviceKind,
    phase: Option<PointerPhase>,
}

impl TracePointerContext {
    pub(crate) const fn event(
        pointer_id: PointerId,
        device_id: Option<InputDeviceId>,
        device_kind: PointerDeviceKind,
        phase: PointerPhase,
    ) -> Self {
        Self {
            pointer_id,
            device_id,
            device_kind,
            phase: Some(phase),
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

/// Internal domain-owned payload for one normalized trace context.
#[doc(hidden)]
#[expect(
    dead_code,
    reason = "domain variants are constructed by the next producer-migration checkpoint"
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceContextData {
    Routed {
        event: TraceEventContext,
        surface: Option<TraceSurfaceContext>,
        route: Option<TraceRouteSnapshot>,
        delivery: Option<TraceDeliveryOutcome>,
    },
    Surface(TraceSurfaceContext),
    Pointer {
        event: Option<TraceEventContext>,
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        route: Option<TraceRouteSnapshot>,
        physical_path: Option<TracePointerPath>,
        target_transition: Option<TraceTargetTransition>,
        delivery: Option<TraceDeliveryOutcome>,
    },
    Focus {
        event: Option<TraceEventContext>,
        surface: Option<TraceSurfaceContext>,
        route: Option<TraceRouteSnapshot>,
        target_transition: Option<TraceTargetTransition>,
        delivery: Option<TraceDeliveryOutcome>,
    },
    Text {
        event: Option<TraceEventContext>,
        surface: Option<TraceSurfaceContext>,
        composition: Option<TraceCompositionContext>,
        text_metrics: Option<TraceTextMetrics>,
        composition_range: Option<TraceCompositionRange>,
        route: Option<TraceRouteSnapshot>,
        delivery: Option<TraceDeliveryOutcome>,
    },
    Action(TraceActionIdentity),
    Publication(TracePublicationContext),
}

/// Typed normalized context attached to one canonical trace record.
///
/// Records without normalized family context retain only a nullable pointer.
/// Enriched records own exactly one domain payload rather than twelve
/// independently optional inline fields.
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
            data: Some(Box::new(TraceContextData::Routed {
                event,
                surface: None,
                route: None,
                delivery: None,
            })),
        }
    }

    pub(crate) fn routed_snapshot(event: TraceEventContext, route: TraceRouteSnapshot) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Routed {
                event,
                surface: None,
                route: Some(route),
                delivery: None,
            })),
        }
    }

    pub(crate) fn surface_record(surface: TraceSurfaceContext) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Surface(surface))),
        }
    }

    pub(crate) fn pointer_observation(
        event: TraceEventContext,
        surface: TraceSurfaceContext,
        pointer: TracePointerContext,
        physical_path: TracePointerPath,
    ) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Pointer {
                event: Some(event),
                surface: Some(surface),
                pointer,
                route: None,
                physical_path: Some(physical_path),
                target_transition: None,
                delivery: None,
            })),
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
            Some(
                TraceContextData::Pointer { event, .. }
                | TraceContextData::Focus { event, .. }
                | TraceContextData::Text { event, .. },
            ) => *event,
            Some(
                TraceContextData::Surface(_)
                | TraceContextData::Action(_)
                | TraceContextData::Publication(_),
            )
            | None => None,
        }
    }

    /// Returns exact displayed-surface facts.
    #[must_use]
    pub fn surface(&self) -> Option<&TraceSurfaceContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Surface(surface)) => Some(surface),
            Some(
                TraceContextData::Routed { surface, .. }
                | TraceContextData::Pointer { surface, .. }
                | TraceContextData::Focus { surface, .. }
                | TraceContextData::Text { surface, .. },
            ) => surface.as_ref(),
            Some(TraceContextData::Publication(publication)) => Some(publication.surface()),
            Some(TraceContextData::Action(_)) | None => None,
        }
    }

    /// Returns exact pointer/device facts.
    #[must_use]
    pub fn pointer(&self) -> Option<&TracePointerContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer { pointer, .. }) => Some(pointer),
            _ => None,
        }
    }

    /// Returns exact composition lifetime facts.
    #[must_use]
    pub fn composition(&self) -> Option<&TraceCompositionContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Text { composition, .. }) => composition.as_ref(),
            _ => None,
        }
    }

    /// Returns redacted text or preedit size facts.
    #[must_use]
    pub fn text_metrics(&self) -> Option<TraceTextMetrics> {
        match self.data.as_deref() {
            Some(TraceContextData::Text { text_metrics, .. }) => *text_metrics,
            _ => None,
        }
    }

    /// Returns the checked redacted composition range.
    #[must_use]
    pub fn composition_range(&self) -> Option<TraceCompositionRange> {
        match self.data.as_deref() {
            Some(TraceContextData::Text {
                composition_range, ..
            }) => *composition_range,
            _ => None,
        }
    }

    /// Returns the ordered routed path and related endpoint.
    #[must_use]
    pub fn route(&self) -> Option<&TraceRouteSnapshot> {
        match self.data.as_deref() {
            Some(
                TraceContextData::Routed { route, .. }
                | TraceContextData::Pointer { route, .. }
                | TraceContextData::Focus { route, .. }
                | TraceContextData::Text { route, .. },
            ) => route.as_ref(),
            _ => None,
        }
    }

    /// Returns the ordered physical pointer path.
    #[must_use]
    pub fn physical_path(&self) -> Option<&TracePointerPath> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer { physical_path, .. }) => physical_path.as_ref(),
            _ => None,
        }
    }

    /// Returns exact previous/current transition endpoints.
    #[must_use]
    pub fn target_transition(&self) -> Option<&TraceTargetTransition> {
        match self.data.as_deref() {
            Some(
                TraceContextData::Pointer {
                    target_transition, ..
                }
                | TraceContextData::Focus {
                    target_transition, ..
                },
            ) => target_transition.as_ref(),
            _ => None,
        }
    }

    /// Returns redacted application-action identity.
    #[must_use]
    pub fn action(&self) -> Option<TraceActionIdentity> {
        match self.data.as_deref() {
            Some(TraceContextData::Action(action)) => Some(*action),
            _ => None,
        }
    }

    /// Returns renderer-facing publication identity.
    #[must_use]
    pub fn publication(&self) -> Option<&TracePublicationContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Publication(publication)) => Some(publication),
            _ => None,
        }
    }

    /// Returns explicit delivery or suppression outcome.
    #[must_use]
    pub fn delivery(&self) -> Option<TraceDeliveryOutcome> {
        match self.data.as_deref() {
            Some(
                TraceContextData::Routed { delivery, .. }
                | TraceContextData::Pointer { delivery, .. }
                | TraceContextData::Focus { delivery, .. }
                | TraceContextData::Text { delivery, .. },
            ) => *delivery,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        __runtime::RuntimeNamespace, PointerDeviceKind, PointerId, PointerPhase,
    };

    use super::{
        TraceContext, TraceEventContext, TraceEventFamily, TracePointerContext, TracePointerPath,
        TraceRouteSnapshot, TraceSurfaceContext,
    };
    use crate::TraceSurfaceSnapshotKind;

    #[test]
    fn empty_context_exposes_no_normalized_family_facts() {
        let context = TraceContext::empty();

        assert_eq!(context.event(), None);
        assert_eq!(context.surface(), None);
        assert_eq!(context.pointer(), None);
        assert_eq!(context.composition(), None);
        assert_eq!(context.text_metrics(), None);
        assert_eq!(context.composition_range(), None);
        assert_eq!(context.route(), None);
        assert_eq!(context.physical_path(), None);
        assert_eq!(context.target_transition(), None);
        assert_eq!(context.action(), None);
        assert_eq!(context.publication(), None);
        assert_eq!(context.delivery(), None);
    }

    #[test]
    fn routed_context_separates_event_and_route_ownership() {
        let event = TraceEventContext::new(TraceEventFamily::SemanticCommand, true);
        let started = TraceContext::routed_event(event);
        let snapshot =
            TraceContext::routed_snapshot(event, TraceRouteSnapshot::new(Vec::new(), None));

        assert_eq!(started.event(), Some(event));
        assert_eq!(started.route(), None);
        assert_eq!(snapshot.event(), Some(event));
        assert_eq!(
            snapshot.route().map(TraceRouteSnapshot::targets),
            Some([].as_slice())
        );
    }

    #[test]
    fn pointer_observation_owns_event_surface_device_and_physical_path() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let surface_context = namespace
            .__runtime_surface_context(surface, 7, 9)
            .unwrap_or_else(|| unreachable!("surface context belongs to the test namespace"));
        let pointer_id = PointerId::new(5)
            .unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
        let event = TraceEventContext::new(TraceEventFamily::Pointer, false);
        let pointer = TracePointerContext::event(
            pointer_id,
            None,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
        );
        let context = TraceContext::pointer_observation(
            event,
            TraceSurfaceContext::accepted(
                &surface_context,
                TraceSurfaceSnapshotKind::Current,
            ),
            pointer,
            TracePointerPath::new(Vec::new()),
        );

        assert_eq!(context.event(), Some(event));
        assert_eq!(
            context.pointer().map(TracePointerContext::pointer_id),
            Some(&pointer_id)
        );
        assert_eq!(
            context.surface().map(TraceSurfaceContext::snapshot),
            Some(Some(TraceSurfaceSnapshotKind::Current))
        );
        assert_eq!(
            context.physical_path().map(TracePointerPath::targets),
            Some([].as_slice())
        );
    }
}
