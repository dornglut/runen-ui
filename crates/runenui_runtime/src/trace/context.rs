use runenui_core::{
    CompositionGeneration, InputDeviceId, InputModality, PointerDeviceKind, PointerId,
    PointerPhase, SurfaceId, SurfaceInputContext,
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

/// Exact previous/current input modality endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceModalityTransition {
    previous: Option<InputModality>,
    current: InputModality,
}

impl TraceModalityTransition {
    pub(crate) const fn new(previous: Option<InputModality>, current: InputModality) -> Self {
        Self { previous, current }
    }

    /// Returns the previous accepted modality, if any.
    #[must_use]
    pub const fn previous(self) -> Option<InputModality> {
        self.previous
    }

    /// Returns the newly accepted modality.
    #[must_use]
    pub const fn current(self) -> InputModality {
        self.current
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

/// Internal domain-owned pointer payload for one normalized trace context.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePointerRecordContext {
    event: Option<TraceEventContext>,
    surface: Option<TraceSurfaceContext>,
    pointer: TracePointerContext,
    route: Option<TraceRouteSnapshot>,
    physical_path: TracePointerPath,
    target_transition: Option<TraceTargetTransition>,
    cleanup: Option<TracePointerCleanup>,
    delivery: Option<TraceDeliveryOutcome>,
}

/// Internal domain-owned focus/modality payload for one normalized trace context.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceFocusRecordContext {
    event: Option<TraceEventContext>,
    surface: Option<TraceSurfaceContext>,
    route: Option<TraceRouteSnapshot>,
    target_transition: Option<TraceTargetTransition>,
    modality_transition: Option<TraceModalityTransition>,
    delivery: Option<TraceDeliveryOutcome>,
}

/// Internal domain-owned payload for one normalized trace context.
#[doc(hidden)]
#[expect(
    dead_code,
    reason = "text and action variants are constructed by later producer-migration checkpoints"
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
    Pointer(Box<TracePointerRecordContext>),
    Focus(Box<TraceFocusRecordContext>),
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

    fn pointer_record(context: TracePointerRecordContext) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Pointer(Box::new(context)))),
        }
    }

    fn focus_record(context: TraceFocusRecordContext) -> Self {
        Self {
            data: Some(Box::new(TraceContextData::Focus(Box::new(context)))),
        }
    }

    pub(crate) fn pointer_observation(
        event: TraceEventContext,
        surface: TraceSurfaceContext,
        pointer: TracePointerContext,
        physical_path: TracePointerPath,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext {
            event: Some(event),
            surface: Some(surface),
            pointer,
            route: None,
            physical_path,
            target_transition: None,
            cleanup: None,
            delivery: None,
        })
    }

    pub(crate) fn pointer_boundary_plan(
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        physical_path: TracePointerPath,
        target_transition: TraceTargetTransition,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext {
            event: Some(TraceEventContext::new(
                TraceEventFamily::PointerBoundary,
                false,
            )),
            surface,
            pointer,
            route: None,
            physical_path,
            target_transition: Some(target_transition),
            cleanup: None,
            delivery: None,
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
        Self::pointer_record(TracePointerRecordContext {
            event: Some(TraceEventContext::new(
                TraceEventFamily::PointerBoundary,
                false,
            )),
            surface: Some(surface),
            pointer,
            route: Some(route),
            physical_path,
            target_transition: Some(target_transition),
            cleanup: None,
            delivery: Some(delivery),
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
        Self::pointer_record(TracePointerRecordContext {
            event: Some(TraceEventContext::new(
                TraceEventFamily::PointerCapture,
                false,
            )),
            surface,
            pointer,
            route: Some(route),
            physical_path,
            target_transition: Some(target_transition),
            cleanup: None,
            delivery: Some(delivery),
        })
    }

    pub(crate) fn pointer_capture_request_rejection(
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        physical_path: TracePointerPath,
        target_transition: TraceTargetTransition,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext {
            event: Some(TraceEventContext::new(
                TraceEventFamily::PointerCapture,
                false,
            )),
            surface,
            pointer,
            route: None,
            physical_path,
            target_transition: Some(target_transition),
            cleanup: None,
            delivery: None,
        })
    }

    pub(crate) fn pointer_integrity_cleanup(
        surface: Option<TraceSurfaceContext>,
        pointer: TracePointerContext,
        physical_path: TracePointerPath,
        cleanup: TracePointerCleanup,
    ) -> Self {
        Self::pointer_record(TracePointerRecordContext {
            event: None,
            surface,
            pointer,
            route: None,
            physical_path,
            target_transition: None,
            cleanup: Some(cleanup),
            delivery: None,
        })
    }

    pub(crate) fn focus_transition(
        surface: Option<TraceSurfaceContext>,
        target_transition: TraceTargetTransition,
    ) -> Self {
        Self::focus_record(TraceFocusRecordContext {
            event: None,
            surface,
            route: None,
            target_transition: Some(target_transition),
            modality_transition: None,
            delivery: None,
        })
    }

    pub(crate) fn focus_notification(
        surface: Option<TraceSurfaceContext>,
        route: TraceRouteSnapshot,
        target_transition: TraceTargetTransition,
        delivery: TraceDeliveryOutcome,
    ) -> Self {
        Self::focus_record(TraceFocusRecordContext {
            event: Some(TraceEventContext::new(TraceEventFamily::Focus, false)),
            surface,
            route: Some(route),
            target_transition: Some(target_transition),
            modality_transition: None,
            delivery: Some(delivery),
        })
    }

    pub(crate) fn modality_transition(transition: TraceModalityTransition) -> Self {
        Self::focus_record(TraceFocusRecordContext {
            event: None,
            surface: None,
            route: None,
            target_transition: None,
            modality_transition: Some(transition),
            delivery: None,
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
            Some(TraceContextData::Pointer(context)) => context.event,
            Some(TraceContextData::Focus(context)) => context.event,
            Some(TraceContextData::Text { event, .. }) => *event,
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
            Some(TraceContextData::Pointer(context)) => context.surface.as_ref(),
            Some(TraceContextData::Focus(context)) => context.surface.as_ref(),
            Some(
                TraceContextData::Routed { surface, .. } | TraceContextData::Text { surface, .. },
            ) => surface.as_ref(),
            Some(TraceContextData::Publication(publication)) => Some(publication.surface()),
            Some(TraceContextData::Action(_)) | None => None,
        }
    }

    /// Returns exact pointer/device facts.
    #[must_use]
    pub fn pointer(&self) -> Option<&TracePointerContext> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => Some(&context.pointer),
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
            Some(TraceContextData::Pointer(context)) => context.route.as_ref(),
            Some(TraceContextData::Focus(context)) => context.route.as_ref(),
            Some(TraceContextData::Routed { route, .. } | TraceContextData::Text { route, .. }) => {
                route.as_ref()
            }
            _ => None,
        }
    }

    /// Returns the ordered physical pointer path.
    #[must_use]
    pub fn physical_path(&self) -> Option<&TracePointerPath> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => Some(&context.physical_path),
            _ => None,
        }
    }

    /// Returns exact previous/current transition endpoints.
    #[must_use]
    pub fn target_transition(&self) -> Option<&TraceTargetTransition> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => context.target_transition.as_ref(),
            Some(TraceContextData::Focus(context)) => context.target_transition.as_ref(),
            _ => None,
        }
    }

    /// Returns exact pointer-owner cleanup facts.
    #[must_use]
    pub fn pointer_cleanup(&self) -> Option<&TracePointerCleanup> {
        match self.data.as_deref() {
            Some(TraceContextData::Pointer(context)) => context.cleanup.as_ref(),
            _ => None,
        }
    }

    /// Returns exact previous/current modality endpoints.
    #[must_use]
    pub fn modality_transition(&self) -> Option<TraceModalityTransition> {
        match self.data.as_deref() {
            Some(TraceContextData::Focus(context)) => context.modality_transition,
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
            Some(TraceContextData::Pointer(context)) => context.delivery,
            Some(TraceContextData::Focus(context)) => context.delivery,
            Some(
                TraceContextData::Routed { delivery, .. } | TraceContextData::Text { delivery, .. },
            ) => *delivery,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        __runtime::RuntimeNamespace, InputModality, PointerDeviceKind, PointerId, PointerPhase,
    };

    use super::{
        TraceContext, TraceDeliveryOutcome, TraceEventContext, TraceEventFamily,
        TraceModalityTransition, TracePointerCleanup, TracePointerContext, TracePointerPath,
        TraceRouteSnapshot, TraceSurfaceContext, TraceTargetTransition,
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
        assert_eq!(context.pointer_cleanup(), None);
        assert_eq!(context.modality_transition(), None);
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
        let pointer_id =
            PointerId::new(5).unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
        let event = TraceEventContext::new(TraceEventFamily::Pointer, false);
        let pointer = TracePointerContext::event(
            pointer_id,
            None,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
        );
        let context = TraceContext::pointer_observation(
            event,
            TraceSurfaceContext::accepted(&surface_context, TraceSurfaceSnapshotKind::Current),
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

    #[test]
    fn boundary_notification_owns_transition_route_related_endpoint_and_outcome() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let surface_context = namespace
            .__runtime_surface_context(surface, 3, 4)
            .unwrap_or_else(|| unreachable!("surface context belongs to the test namespace"));
        let pointer_id =
            PointerId::new(8).unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
        let pointer = TracePointerContext::event(
            pointer_id,
            None,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
        );
        let context = TraceContext::pointer_boundary_notification(
            TraceSurfaceContext::accepted(&surface_context, TraceSurfaceSnapshotKind::Current),
            pointer,
            TraceRouteSnapshot::new(Vec::new(), None),
            TracePointerPath::new(Vec::new()),
            TraceTargetTransition::new(None, None),
            TraceDeliveryOutcome::Suppressed,
        );

        assert_eq!(
            context.event().map(TraceEventContext::family),
            Some(TraceEventFamily::PointerBoundary)
        );
        assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Suppressed));
        assert_eq!(
            context.route().map(TraceRouteSnapshot::targets),
            Some([].as_slice())
        );
        assert!(context.target_transition().is_some());
    }

    #[test]
    fn capture_notification_owns_transition_route_related_endpoint_and_outcome() {
        let pointer_id =
            PointerId::new(9).unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
        let pointer = TracePointerContext::event(
            pointer_id,
            None,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
        );
        let context = TraceContext::pointer_capture_notification(
            None,
            pointer,
            TraceRouteSnapshot::new(Vec::new(), None),
            TracePointerPath::new(Vec::new()),
            TraceTargetTransition::new(None, None),
            TraceDeliveryOutcome::Delivered,
        );

        assert_eq!(
            context.event().map(TraceEventContext::family),
            Some(TraceEventFamily::PointerCapture)
        );
        assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Delivered));
        assert_eq!(
            context.pointer().and_then(TracePointerContext::phase),
            Some(PointerPhase::Move)
        );
        assert!(context.target_transition().is_some());
    }

    #[test]
    fn pointer_cleanup_owns_stream_identity_transitions_and_path_outcome() {
        let pointer_id =
            PointerId::new(10).unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
        let context = TraceContext::pointer_integrity_cleanup(
            None,
            TracePointerContext::stream(pointer_id, None, PointerDeviceKind::Mouse),
            TracePointerPath::new(Vec::new()),
            TracePointerCleanup::new(
                Some(TraceTargetTransition::new(None, None)),
                Some(TraceTargetTransition::new(None, None)),
                true,
            ),
        );

        assert_eq!(context.event(), None);
        assert_eq!(
            context.pointer().map(TracePointerContext::pointer_id),
            Some(&pointer_id)
        );
        assert_eq!(context.pointer().and_then(TracePointerContext::phase), None);
        let cleanup = context
            .pointer_cleanup()
            .unwrap_or_else(|| unreachable!("cleanup context owns cleanup facts"));
        assert!(cleanup.pressed_owner().is_some());
        assert!(cleanup.capture_owner().is_some());
        assert!(cleanup.physical_path_cleared());
    }

    #[test]
    fn focus_notification_owns_route_transition_and_outcome() {
        let context = TraceContext::focus_notification(
            None,
            TraceRouteSnapshot::new(Vec::new(), None),
            TraceTargetTransition::new(None, None),
            TraceDeliveryOutcome::Delivered,
        );

        assert_eq!(
            context.event().map(TraceEventContext::family),
            Some(TraceEventFamily::Focus)
        );
        assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Delivered));
        assert!(context.route().is_some());
        assert!(context.target_transition().is_some());
    }

    #[test]
    fn modality_context_owns_exact_endpoints() {
        let transition = TraceModalityTransition::new(None, InputModality::Keyboard);
        let context = TraceContext::modality_transition(transition);

        assert_eq!(context.modality_transition(), Some(transition));
        assert_eq!(transition.previous(), None);
        assert_eq!(transition.current(), InputModality::Keyboard);
    }
}
