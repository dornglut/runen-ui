use runenui_core::{
    InputDeviceId, PointerDeviceKind, PointerId, PointerPhase, SurfaceId, SurfaceInputContext,
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

    pub(crate) const fn stream(
        pointer_id: PointerId,
        device_id: Option<InputDeviceId>,
        device_kind: PointerDeviceKind,
    ) -> Self {
        Self {
            pointer_id,
            device_id,
            device_kind,
            phase: None,
        }
    }

    /// Returns the exact pointer-stream identity represented by these device facts.
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
    pub(crate) const fn new(previous: Option<TraceTarget>, current: Option<TraceTarget>) -> Self {
        Self { previous, current }
    }

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

/// Exact pointer-owner cleanup committed by one lifecycle boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePointerCleanup {
    pressed_owner: Option<TraceTargetTransition>,
    capture_owner: Option<TraceTargetTransition>,
    physical_path_cleared: bool,
}

impl TracePointerCleanup {
    pub(crate) const fn new(
        pressed_owner: Option<TraceTargetTransition>,
        capture_owner: Option<TraceTargetTransition>,
        physical_path_cleared: bool,
    ) -> Self {
        Self {
            pressed_owner,
            capture_owner,
            physical_path_cleared,
        }
    }

    /// Returns the exact pressed-owner transition when pressed authority was cleared.
    #[must_use]
    pub const fn pressed_owner(&self) -> Option<&TraceTargetTransition> {
        self.pressed_owner.as_ref()
    }

    /// Returns the exact capture-owner transition when capture authority was cleared.
    #[must_use]
    pub const fn capture_owner(&self) -> Option<&TraceTargetTransition> {
        self.capture_owner.as_ref()
    }

    /// Returns whether the retained physical path was cleared.
    #[must_use]
    pub const fn physical_path_cleared(&self) -> bool {
        self.physical_path_cleared
    }
}

/// Whether a required pointer notification reached callbacks or was suppressed.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceDeliveryOutcome {
    Delivered,
    Suppressed,
}

/// Semantic role of the typed pointer payload stored by one trace record.
///
/// The role makes path, transition, delivery, and cleanup meaning explicit;
/// callers do not need to infer a nullable-field combination from record kind.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TracePointerRecordRole {
    Observation,
    BoundaryPlan,
    BoundaryNotification,
    CaptureNotification,
    CaptureRequestRejection,
    Cleanup,
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
enum TracePointerRecordContext {
    Observation {
        event: TraceEventContext,
        surface: TraceSurfaceContext,
        pointer: TracePointerContext,
        current_path: TracePointerPath,
    },
    BoundaryPlan {
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        previous_path: TracePointerPath,
        transition: TraceTargetTransition,
    },
    BoundaryNotification {
        surface: TraceSurfaceContext,
        pointer: TracePointerContext,
        route: TraceRouteSnapshot,
        current_path: TracePointerPath,
        transition: TraceTargetTransition,
        delivery: TraceDeliveryOutcome,
    },
    CaptureNotification {
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        route: TraceRouteSnapshot,
        current_path: TracePointerPath,
        transition: TraceTargetTransition,
        delivery: TraceDeliveryOutcome,
    },
    CaptureRequestRejection {
        surface: Option<TraceSurfaceContext>,
        event_pointer: TracePointerContext,
        requested_pointer_id: PointerId,
        current_path: TracePointerPath,
        transition: TraceTargetTransition,
    },
    Cleanup {
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        prior_path: TracePointerPath,
        cleanup: TracePointerCleanup,
    },
}

impl TracePointerRecordContext {
    const fn role(&self) -> TracePointerRecordRole {
        match self {
            Self::Observation { .. } => TracePointerRecordRole::Observation,
            Self::BoundaryPlan { .. } => TracePointerRecordRole::BoundaryPlan,
            Self::BoundaryNotification { .. } => TracePointerRecordRole::BoundaryNotification,
            Self::CaptureNotification { .. } => TracePointerRecordRole::CaptureNotification,
            Self::CaptureRequestRejection { .. } => TracePointerRecordRole::CaptureRequestRejection,
            Self::Cleanup { .. } => TracePointerRecordRole::Cleanup,
        }
    }

    const fn event(&self) -> Option<TraceEventContext> {
        match self {
            Self::Observation { event, .. } => Some(*event),
            Self::BoundaryPlan { .. } | Self::BoundaryNotification { .. } => Some(
                TraceEventContext::new(TraceEventFamily::PointerBoundary, false),
            ),
            Self::CaptureNotification { .. } | Self::CaptureRequestRejection { .. } => Some(
                TraceEventContext::new(TraceEventFamily::PointerCapture, false),
            ),
            Self::Cleanup { .. } => None,
        }
    }

    const fn surface(&self) -> Option<&TraceSurfaceContext> {
        match self {
            Self::Observation { surface, .. } | Self::BoundaryNotification { surface, .. } => {
                Some(surface)
            }
            Self::BoundaryPlan { surface, .. }
            | Self::CaptureNotification { surface, .. }
            | Self::CaptureRequestRejection { surface, .. }
            | Self::Cleanup { surface, .. } => surface.as_ref(),
        }
    }

    const fn pointer(&self) -> &TracePointerContext {
        match self {
            Self::Observation { pointer, .. }
            | Self::BoundaryPlan { pointer, .. }
            | Self::BoundaryNotification { pointer, .. }
            | Self::CaptureNotification { pointer, .. }
            | Self::Cleanup { pointer, .. } => pointer,
            Self::CaptureRequestRejection { event_pointer, .. } => event_pointer,
        }
    }

    const fn requested_pointer_id(&self) -> Option<&PointerId> {
        match self {
            Self::CaptureRequestRejection {
                requested_pointer_id,
                ..
            } => Some(requested_pointer_id),
            _ => None,
        }
    }

    const fn route(&self) -> Option<&TraceRouteSnapshot> {
        match self {
            Self::BoundaryNotification { route, .. } | Self::CaptureNotification { route, .. } => {
                Some(route)
            }
            _ => None,
        }
    }

    const fn physical_path(&self) -> &TracePointerPath {
        match self {
            Self::Observation { current_path, .. }
            | Self::BoundaryNotification { current_path, .. }
            | Self::CaptureNotification { current_path, .. }
            | Self::CaptureRequestRejection { current_path, .. } => current_path,
            Self::BoundaryPlan { previous_path, .. } => previous_path,
            Self::Cleanup { prior_path, .. } => prior_path,
        }
    }

    const fn target_transition(&self) -> Option<&TraceTargetTransition> {
        match self {
            Self::BoundaryPlan { transition, .. }
            | Self::BoundaryNotification { transition, .. }
            | Self::CaptureNotification { transition, .. }
            | Self::CaptureRequestRejection { transition, .. } => Some(transition),
            Self::Observation { .. } | Self::Cleanup { .. } => None,
        }
    }

    const fn cleanup(&self) -> Option<&TracePointerCleanup> {
        match self {
            Self::Cleanup { cleanup, .. } => Some(cleanup),
            _ => None,
        }
    }

    const fn delivery(&self) -> Option<TraceDeliveryOutcome> {
        match self {
            Self::BoundaryNotification { delivery, .. }
            | Self::CaptureNotification { delivery, .. } => Some(*delivery),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TraceContextData {
    Routed {
        event: TraceEventContext,
        route: Option<TraceRouteSnapshot>,
    },
    Surface(TraceSurfaceContext),
    Pointer(Box<TracePointerRecordContext>),
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

    fn pointer_record(context: TracePointerRecordContext) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Pointer(Box::new(context)))),
        }
    }

    pub(crate) fn pointer_observation(
        event: TraceEventContext,
        surface: TraceSurfaceContext,
        pointer: TracePointerContext,
        physical_path: TracePointerPath,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext::Observation {
            event,
            surface,
            pointer,
            current_path: physical_path,
        })
    }

    pub(crate) fn pointer_boundary_plan(
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        physical_path: TracePointerPath,
        target_transition: TraceTargetTransition,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext::BoundaryPlan {
            surface,
            pointer,
            previous_path: physical_path,
            transition: target_transition,
        })
    }

    pub(crate) fn pointer_boundary_notification(
        surface: TraceSurfaceContext,
        pointer: TracePointerContext,
        route: TraceRouteSnapshot,
        physical_path: TracePointerPath,
        target_transition: TraceTargetTransition,
        delivery: TraceDeliveryOutcome,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext::BoundaryNotification {
            surface,
            pointer,
            route,
            current_path: physical_path,
            transition: target_transition,
            delivery,
        })
    }

    pub(crate) fn pointer_capture_notification(
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        route: TraceRouteSnapshot,
        physical_path: TracePointerPath,
        target_transition: TraceTargetTransition,
        delivery: TraceDeliveryOutcome,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext::CaptureNotification {
            surface,
            pointer,
            route,
            current_path: physical_path,
            transition: target_transition,
            delivery,
        })
    }

    pub(crate) fn pointer_capture_request_rejection(
        surface: Option<TraceSurfaceContext>,
        event_pointer: TracePointerContext,
        requested_pointer_id: PointerId,
        physical_path: TracePointerPath,
        target_transition: TraceTargetTransition,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext::CaptureRequestRejection {
            surface,
            event_pointer,
            requested_pointer_id,
            current_path: physical_path,
            transition: target_transition,
        })
    }

    pub(crate) fn pointer_integrity_cleanup(
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        physical_path: TracePointerPath,
        cleanup: TracePointerCleanup,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext::Cleanup {
            surface,
            pointer,
            prior_path: physical_path,
            cleanup,
        })
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
            Some(TraceContextData::Pointer(context)) => context.event(),
            Some(TraceContextData::Surface(_) | TraceContextData::Publication(_)) | None => None,
        }
    }

    /// Returns exact displayed-surface facts.
    #[must_use]
    pub fn surface(&self) -> Option<&TraceSurfaceContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Surface(surface)) => Some(surface),
            Some(TraceContextData::Pointer(context)) => context.surface(),
            Some(TraceContextData::Publication(publication)) => Some(publication.surface()),
            Some(TraceContextData::Routed { .. }) | None => None,
        }
    }

    /// Returns exact active pointer/device facts for pointer-domain records.
    ///
    /// For capture-request rejection this is the active event pointer. The
    /// separately requested identity is available through [`Self::requested_pointer_id`].
    #[must_use]
    pub fn pointer(&self) -> Option<&TracePointerContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => Some(context.pointer()),
            _ => None,
        }
    }

    /// Returns the semantic role of the typed pointer payload.
    #[must_use]
    pub fn pointer_record_role(&self) -> Option<TracePointerRecordRole> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => Some(context.role()),
            _ => None,
        }
    }

    /// Returns the exact pointer identity named by a rejected capture request.
    ///
    /// This differs from [`Self::pointer`] only for the mismatch rejection case.
    #[must_use]
    pub fn requested_pointer_id(&self) -> Option<&PointerId> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => context.requested_pointer_id(),
            _ => None,
        }
    }

    /// Returns the ordered routed path and related endpoint.
    #[must_use]
    pub fn route(&self) -> Option<&TraceRouteSnapshot> {
        match self.data.as_deref() {
            Some(TraceContextData::Routed { route, .. }) => route.as_ref(),
            Some(TraceContextData::Pointer(context)) => context.route(),
            Some(TraceContextData::Surface(_) | TraceContextData::Publication(_)) | None => None,
        }
    }

    /// Returns the role-owned physical path.
    ///
    /// Observation/notification/rejection roles retain the current observed
    /// path, `BoundaryPlan` retains the previous path used for the diff, and
    /// `Cleanup` retains the pre-cleanup path. Use [`Self::pointer_record_role`]
    /// to interpret the path without relying on record-kind convention.
    #[must_use]
    pub fn physical_path(&self) -> Option<&TracePointerPath> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => Some(context.physical_path()),
            _ => None,
        }
    }

    /// Returns exact previous/current transition endpoints when the role owns them.
    #[must_use]
    pub fn target_transition(&self) -> Option<&TraceTargetTransition> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => context.target_transition(),
            _ => None,
        }
    }

    /// Returns exact pointer-owner cleanup facts.
    #[must_use]
    pub fn pointer_cleanup(&self) -> Option<&TracePointerCleanup> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => context.cleanup(),
            _ => None,
        }
    }

    /// Returns renderer-facing publication identity.
    #[must_use]
    pub fn publication(&self) -> Option<&TracePublicationContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Publication(publication)) => Some(publication),
            Some(
                TraceContextData::Routed { .. }
                | TraceContextData::Surface(_)
                | TraceContextData::Pointer(_),
            )
            | None => None,
        }
    }

    /// Returns explicit pointer-notification delivery or suppression outcome.
    #[must_use]
    pub fn delivery(&self) -> Option<TraceDeliveryOutcome> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => context.delivery(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{__runtime::RuntimeNamespace, PointerDeviceKind, PointerId, PointerPhase};

    use super::{
        TraceContext, TraceDeliveryOutcome, TracePointerContext, TracePointerPath,
        TracePointerRecordRole, TraceRouteSnapshot, TraceSurfaceContext, TraceSurfaceSnapshotKind,
        TraceTargetTransition,
    };

    fn pointer_id(value: u64) -> PointerId {
        PointerId::new(value).unwrap_or_else(|| unreachable!("test pointer identity is non-zero"))
    }

    #[test]
    fn empty_context_exposes_no_normalized_facts() {
        let context = TraceContext::empty();

        assert_eq!(context.event(), None);
        assert_eq!(context.surface(), None);
        assert_eq!(context.pointer(), None);
        assert_eq!(context.pointer_record_role(), None);
        assert_eq!(context.requested_pointer_id(), None);
        assert_eq!(context.route(), None);
        assert_eq!(context.physical_path(), None);
        assert_eq!(context.target_transition(), None);
        assert_eq!(context.pointer_cleanup(), None);
        assert_eq!(context.publication(), None);
        assert_eq!(context.delivery(), None);
    }

    #[test]
    fn capture_rejection_keeps_active_and_requested_pointer_identities_distinct() {
        let active = pointer_id(11);
        let requested = pointer_id(12);
        let context = TraceContext::pointer_capture_request_rejection(
            None,
            TracePointerContext::event(active, None, PointerDeviceKind::Mouse, PointerPhase::Move),
            requested,
            TracePointerPath::new(Vec::new()),
            TraceTargetTransition::new(None, None),
        );

        assert_eq!(
            context.pointer_record_role(),
            Some(TracePointerRecordRole::CaptureRequestRejection)
        );
        let pointer = context
            .pointer()
            .unwrap_or_else(|| unreachable!("rejection retains active event pointer facts"));
        assert_eq!(pointer.pointer_id().get(), 11);
        assert_eq!(pointer.device_kind(), PointerDeviceKind::Mouse);
        assert_eq!(pointer.phase(), Some(PointerPhase::Move));
        assert_eq!(
            context.requested_pointer_id().map(|pointer| pointer.get()),
            Some(12)
        );
        assert_eq!(context.delivery(), None);
        assert_eq!(context.route(), None);
    }

    #[test]
    fn suppressed_boundary_resolution_has_an_explicit_typed_role() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace
            .__runtime_surface_context(namespace.__runtime_surface_id(0, 1), 1, 1)
            .unwrap_or_else(|| unreachable!("test surface belongs to the namespace"));
        let context = TraceContext::pointer_boundary_notification(
            TraceSurfaceContext::accepted(&surface, TraceSurfaceSnapshotKind::Current),
            TracePointerContext::event(
                pointer_id(13),
                None,
                PointerDeviceKind::Mouse,
                PointerPhase::Move,
            ),
            TraceRouteSnapshot::new(Vec::new(), None),
            TracePointerPath::new(Vec::new()),
            TraceTargetTransition::new(None, None),
            TraceDeliveryOutcome::Suppressed,
        );

        assert_eq!(
            context.pointer_record_role(),
            Some(TracePointerRecordRole::BoundaryNotification)
        );
        assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Suppressed));
        assert!(context.route().is_some());
        assert_eq!(context.requested_pointer_id(), None);
    }
}
