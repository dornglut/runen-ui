use core::num::NonZeroUsize;
use std::{collections::VecDeque, sync::Arc};

use runenui_core::{__runtime::RuntimeNamespace, SurfaceId, SurfaceInputContext};

use crate::{
    LogicalPoint, LogicalRect, MountedNodeId, RedrawAcknowledgeError, RedrawRequest,
    SurfaceBuildContext, SurfacePhase, SurfacePhaseReport, SurfacePublication,
    SurfacePublicationCounter, TraceSurfaceContext, TraceSurfaceSnapshotKind,
    mounted::MountedTree,
    scene::{HitTestScene, PaintPublication, PaintRevision},
    semantic_publication::{
        SemanticPublicationPlan, SemanticPublicationPlanError, SemanticPublicationState,
    },
    surface::{
        SurfaceCache, SurfaceInteractionProjection, SurfacePlanningError,
        plan_mounted_surface_cached,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::runtime) enum SurfaceSnapshotError {
    ForeignSurfaceContext,
    ForeignSurface,
    RetiredSurfaceContext,
    MissingSurfaceGeneration,
    CoordinateRevisionMismatch,
    NoTarget,
    TargetNotInSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::runtime) enum SurfaceIdentityError {
    Foreign,
    Wrong,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::runtime) enum SurfaceSnapshotKind {
    Current,
    Retained,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::runtime) struct SurfaceSnapshotSelection {
    snapshot_kind: SurfaceSnapshotKind,
    hit_test_generation: u64,
    coordinate_revision: u64,
}

impl SurfaceSnapshotSelection {
    pub(in crate::runtime) const fn snapshot_kind(self) -> SurfaceSnapshotKind {
        self.snapshot_kind
    }

    pub(in crate::runtime) const fn hit_test_generation(self) -> u64 {
        self.hit_test_generation
    }

    pub(in crate::runtime) const fn coordinate_revision(self) -> u64 {
        self.coordinate_revision
    }
}

pub(in crate::runtime) struct SurfaceTargetResolution {
    target: MountedNodeId,
    selection: SurfaceSnapshotSelection,
}

impl SurfaceTargetResolution {
    pub(in crate::runtime) const fn snapshot_kind(&self) -> SurfaceSnapshotKind {
        self.selection.snapshot_kind()
    }

    pub(in crate::runtime) const fn hit_test_generation(&self) -> u64 {
        self.selection.hit_test_generation()
    }

    pub(in crate::runtime) const fn coordinate_revision(&self) -> u64 {
        self.selection.coordinate_revision()
    }

    pub(in crate::runtime) fn into_target(self) -> MountedNodeId {
        self.target
    }
}

/// Valid retained geometry and its optional physical hit target for pointer input.
pub(in crate::runtime) struct SurfacePointResolution {
    target: Option<MountedNodeId>,
    selection: SurfaceSnapshotSelection,
}

impl SurfacePointResolution {
    pub(in crate::runtime) const fn snapshot_kind(&self) -> SurfaceSnapshotKind {
        self.selection.snapshot_kind()
    }

    pub(in crate::runtime) fn into_target(self) -> Option<MountedNodeId> {
        self.target
    }
}

/// Exact non-mutating reservation for one displayed surface publication attempt.
///
/// Construction succeeds only while the inherited displayed-input counters can
/// issue their next values. Paint revision admission is intentionally later: it
/// depends on the staged renderer candidate and occurs before `commit_store`.
pub(in crate::runtime) struct SurfacePublicationAdmission {
    hit_test_generation: u64,
    coordinate_revision: u64,
}

impl SurfacePublicationAdmission {
    const fn into_parts(self) -> (u64, u64) {
        (self.hit_test_generation, self.coordinate_revision)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::runtime) enum SurfacePublicationPlanError {
    SemanticIntegrity,
    CounterExhausted(SurfacePublicationCounter),
}

impl From<SurfacePlanningError> for SurfacePublicationPlanError {
    fn from(_: SurfacePlanningError) -> Self {
        Self::SemanticIntegrity
    }
}

/// Sole runtime-owned state for current surface publication, renderer revision,
/// redraw revision, and bounded displayed hit-test generations.
pub(crate) struct SurfacePublicationState {
    cache: Option<SurfaceCache>,
    current_paint: Option<PaintPublication>,
    semantic_publication: SemanticPublicationState,
    phase_report: SurfacePhaseReport,
    redraw_namespace: Arc<()>,
    redraw_revision: u64,
    redraw_acknowledged: u64,
    runtime_namespace: RuntimeNamespace,
    surface_id: SurfaceId,
    retained_snapshot_limit: NonZeroUsize,
    snapshots: VecDeque<HitTestScene>,
    retired_through_generation: Option<u64>,
    next_paint_revision: Option<u64>,
    next_hit_test_generation: Option<u64>,
    next_coordinate_revision: Option<u64>,
}

impl SurfacePublicationState {
    pub(crate) fn new(
        runtime_namespace: RuntimeNamespace,
        retained_snapshot_limit: NonZeroUsize,
    ) -> Self {
        let surface_id = runtime_namespace.__runtime_surface_id(0, 1);
        Self {
            cache: None,
            current_paint: None,
            semantic_publication: SemanticPublicationState::default(),
            phase_report: SurfacePhaseReport::default(),
            redraw_namespace: Arc::new(()),
            redraw_revision: 1,
            redraw_acknowledged: 0,
            runtime_namespace,
            surface_id,
            retained_snapshot_limit,
            snapshots: VecDeque::new(),
            retired_through_generation: None,
            next_paint_revision: Some(1),
            next_hit_test_generation: Some(1),
            next_coordinate_revision: Some(1),
        }
    }

    pub(in crate::runtime) fn admit_publication(
        &self,
    ) -> Result<SurfacePublicationAdmission, SurfacePublicationCounter> {
        let hit_test_generation = self
            .next_hit_test_generation
            .ok_or(SurfacePublicationCounter::HitTestGeneration)?;
        let coordinate_revision = self
            .next_coordinate_revision
            .ok_or(SurfacePublicationCounter::CoordinateRevision)?;
        Ok(SurfacePublicationAdmission {
            hit_test_generation,
            coordinate_revision,
        })
    }

    pub(crate) fn publish<Action>(
        &mut self,
        tree: &mut MountedTree<Action>,
        context: &SurfaceBuildContext<'_>,
        interaction: &SurfaceInteractionProjection,
        focused_owner: Option<&MountedNodeId>,
        admission: SurfacePublicationAdmission,
    ) -> Result<SurfacePublication, SurfacePublicationPlanError> {
        let (hit_test_generation, coordinate_revision) = admission.into_parts();
        let planned =
            plan_mounted_surface_cached(tree, context, interaction, self.cache.as_ref())?;
        let semantic_candidate = planned.semantic_candidate(focused_owner)?;
        let semantic_plan: SemanticPublicationPlan = self
            .semantic_publication
            .plan(&self.surface_id, semantic_candidate)
            .map_err(|error| match error {
                SemanticPublicationPlanError::RevisionExhausted => {
                    SurfacePublicationPlanError::CounterExhausted(
                        SurfacePublicationCounter::SemanticRevision,
                    )
                }
            })?;
        let semantic_publication = semantic_plan
            .publication()
            .cloned()
            .ok_or(SurfacePublicationPlanError::SemanticIntegrity)?;
        let semantic_diagnostics = semantic_plan
            .diagnostics()
            .cloned()
            .ok_or(SurfacePublicationPlanError::SemanticIntegrity)?;

        let input_context = self
            .runtime_namespace
            .__runtime_surface_context(
                self.surface_id.clone(),
                coordinate_revision,
                hit_test_generation,
            )
            .unwrap_or_else(|| unreachable!("surface identity shares the runtime namespace"));
        let hit_test_scene = HitTestScene::new(input_context, planned.hit_test_content().clone());

        let paint_size = planned.publication().frame().size();
        let raster_scale = context.raster_scale();
        let paint_changed = self.current_paint.as_ref().is_none_or(|current| {
            current.scene() != planned.paint_scene()
                || current.logical_size() != paint_size
                || current.raster_scale() != raster_scale
        });
        let (paint_publication, allocated_paint_revision) = if paint_changed {
            let value =
                self.next_paint_revision
                    .ok_or(SurfacePublicationPlanError::CounterExhausted(
                        SurfacePublicationCounter::PaintRevision,
                    ))?;
            let revision = PaintRevision::new(value)
                .unwrap_or_else(|| unreachable!("paint revision starts at one and never wraps"));
            let base_revision = self.current_paint.as_ref().map(PaintPublication::revision);
            (
                PaintPublication::new(
                    self.surface_id.clone(),
                    revision,
                    base_revision,
                    paint_size,
                    raster_scale,
                    planned.paint_scene().clone(),
                ),
                Some(value),
            )
        } else {
            (
                self.current_paint
                    .as_ref()
                    .unwrap_or_else(|| unreachable!("unchanged paint has an accepted predecessor"))
                    .clone(),
                None,
            )
        };

        let commit = planned.commit_store();
        let (products, report) = commit.commit(tree, &mut self.cache);
        self.semantic_publication.commit(semantic_plan);
        if let Some(revision) = allocated_paint_revision {
            self.next_paint_revision = revision.checked_add(1);
            self.current_paint = Some(paint_publication.clone());
        }
        self.phase_report = report;
        self.retain_new_snapshot(
            hit_test_scene.clone(),
            hit_test_generation,
            coordinate_revision,
        );
        Ok(SurfacePublication::new(
            paint_publication,
            hit_test_scene,
            products,
            semantic_publication,
            semantic_diagnostics,
        ))
    }

    fn retain_new_snapshot(
        &mut self,
        scene: HitTestScene,
        hit_test_generation: u64,
        coordinate_revision: u64,
    ) {
        debug_assert_eq!(
            self.next_hit_test_generation,
            Some(hit_test_generation),
            "surface publication admission names the current hit-test generation"
        );
        debug_assert_eq!(
            self.next_coordinate_revision,
            Some(coordinate_revision),
            "surface publication admission names the current coordinate revision"
        );
        debug_assert_eq!(
            scene.input_context().hit_test_generation(),
            hit_test_generation,
            "retained hit scene owns the admitted hit-test generation"
        );
        debug_assert_eq!(
            scene.input_context().coordinate_revision(),
            coordinate_revision,
            "retained hit scene owns the admitted coordinate revision"
        );
        self.next_hit_test_generation = hit_test_generation.checked_add(1);
        self.next_coordinate_revision = coordinate_revision.checked_add(1);
        if self.snapshots.len() == self.retained_snapshot_limit.get()
            && let Some(retired) = self.snapshots.pop_front()
        {
            self.retired_through_generation = Some(retired.input_context().hit_test_generation());
        }
        self.snapshots.push_back(scene);
    }

    pub(in crate::runtime) fn current_trace_surface_context(&self) -> Option<TraceSurfaceContext> {
        self.snapshots.back().map(|snapshot| {
            TraceSurfaceContext::accepted(
                snapshot.input_context(),
                TraceSurfaceSnapshotKind::Current,
            )
        })
    }

    pub(in crate::runtime) fn validate_surface_id(
        &self,
        surface: &SurfaceId,
    ) -> Result<(), SurfaceIdentityError> {
        if self
            .runtime_namespace
            .__runtime_surface_parts(surface)
            .is_none()
        {
            return Err(SurfaceIdentityError::Foreign);
        }
        if surface != &self.surface_id {
            return Err(SurfaceIdentityError::Wrong);
        }
        Ok(())
    }

    pub(in crate::runtime) const fn current_semantic_publication(
        &self,
    ) -> Option<&crate::SemanticPublication> {
        self.semantic_publication.current_publication()
    }

    /// Validates only runtime namespace and logical-surface identity.
    pub(in crate::runtime) fn validate_surface_identity(
        &self,
        context: &SurfaceInputContext,
    ) -> Result<SurfaceId, SurfaceSnapshotError> {
        let Some((surface_id, _, _)) = self
            .runtime_namespace
            .__runtime_surface_context_parts(context)
        else {
            return Err(SurfaceSnapshotError::ForeignSurfaceContext);
        };
        if surface_id != self.surface_id {
            return Err(SurfaceSnapshotError::ForeignSurface);
        }
        Ok(surface_id)
    }

    pub(in crate::runtime) fn resolve_point(
        &self,
        context: &SurfaceInputContext,
        point: LogicalPoint,
    ) -> Result<SurfaceTargetResolution, SurfaceSnapshotError> {
        let resolution = self.resolve_pointer_point(context, point)?;
        let target = resolution.target.ok_or(SurfaceSnapshotError::NoTarget)?;
        Ok(SurfaceTargetResolution {
            target,
            selection: resolution.selection,
        })
    }

    /// Validates retained geometry and returns an optional physical hit target.
    pub(in crate::runtime) fn resolve_pointer_point(
        &self,
        context: &SurfaceInputContext,
        point: LogicalPoint,
    ) -> Result<SurfacePointResolution, SurfaceSnapshotError> {
        let (snapshot, snapshot_kind) = self.validate_context(context)?;
        Ok(SurfacePointResolution {
            target: snapshot.target_at(point).cloned(),
            selection: Self::selection(snapshot, snapshot_kind),
        })
    }

    pub(in crate::runtime) fn validate_resolved_target(
        &self,
        context: &SurfaceInputContext,
        target: &MountedNodeId,
    ) -> Result<SurfaceSnapshotSelection, SurfaceSnapshotError> {
        let (snapshot, snapshot_kind) = self.validate_context(context)?;
        snapshot
            .contains_mounted_target(target)
            .then(|| Self::selection(snapshot, snapshot_kind))
            .ok_or(SurfaceSnapshotError::TargetNotInSnapshot)
    }

    const fn selection(
        snapshot: &HitTestScene,
        snapshot_kind: SurfaceSnapshotKind,
    ) -> SurfaceSnapshotSelection {
        SurfaceSnapshotSelection {
            snapshot_kind,
            hit_test_generation: snapshot.input_context().hit_test_generation(),
            coordinate_revision: snapshot.input_context().coordinate_revision(),
        }
    }

    fn validate_context(
        &self,
        context: &SurfaceInputContext,
    ) -> Result<(&HitTestScene, SurfaceSnapshotKind), SurfaceSnapshotError> {
        let Some((surface_id, coordinate_revision, hit_test_generation)) = self
            .runtime_namespace
            .__runtime_surface_context_parts(context)
        else {
            return Err(SurfaceSnapshotError::ForeignSurfaceContext);
        };
        if surface_id != self.surface_id {
            return Err(SurfaceSnapshotError::ForeignSurface);
        }
        let Some(snapshot) = self
            .snapshots
            .iter()
            .find(|snapshot| snapshot.input_context().hit_test_generation() == hit_test_generation)
        else {
            return Err(
                if self
                    .retired_through_generation
                    .is_some_and(|retired| hit_test_generation <= retired)
                {
                    SurfaceSnapshotError::RetiredSurfaceContext
                } else {
                    SurfaceSnapshotError::MissingSurfaceGeneration
                },
            );
        };
        if snapshot.input_context().coordinate_revision() != coordinate_revision {
            return Err(SurfaceSnapshotError::CoordinateRevisionMismatch);
        }
        let snapshot_kind = if self.snapshots.back().is_some_and(|current| {
            current.input_context().hit_test_generation() == hit_test_generation
        }) {
            SurfaceSnapshotKind::Current
        } else {
            SurfaceSnapshotKind::Retained
        };
        Ok((snapshot, snapshot_kind))
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn context_for_test(
        &self,
        surface_slot: u32,
        surface_generation: u64,
        coordinate_revision: u64,
        hit_test_generation: u64,
    ) -> SurfaceInputContext {
        let surface = self
            .runtime_namespace
            .__runtime_surface_id(surface_slot, surface_generation);
        self.runtime_namespace
            .__runtime_surface_context(surface, coordinate_revision, hit_test_generation)
            .unwrap_or_else(|| unreachable!("test surface shares the runtime namespace"))
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn replace_snapshot_target_for_test(
        &mut self,
        context: &SurfaceInputContext,
        original: &MountedNodeId,
        replacement: MountedNodeId,
    ) {
        let generation = context.hit_test_generation();
        let snapshot = self
            .snapshots
            .iter_mut()
            .find(|snapshot| snapshot.input_context().hit_test_generation() == generation)
            .unwrap_or_else(|| unreachable!("test context names one retained snapshot"));
        snapshot.replace_target_for_test(original, replacement);
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn replace_current_focus_geometry_for_test(
        &mut self,
        geometry: &[(MountedNodeId, [f32; 4])],
    ) {
        let projected = geometry
            .iter()
            .map(|(id, [x, y, width, height])| {
                (
                    id.clone(),
                    LogicalRect::new(
                        LogicalPoint::new(*x, *y)
                            .unwrap_or_else(|_| unreachable!("test focus origin is finite")),
                        crate::LogicalSize::try_new(*width, *height).unwrap_or_else(|_| {
                            unreachable!("test focus size is finite and non-negative")
                        }),
                    ),
                )
            })
            .collect::<Vec<_>>();
        self.cache
            .as_mut()
            .unwrap_or_else(|| unreachable!("test publishes before replacing focus geometry"))
            .replace_focus_geometry_for_test(&projected);
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_publication_counters_for_test(
        &mut self,
        hit_test_generation: Option<u64>,
        coordinate_revision: Option<u64>,
    ) {
        self.next_hit_test_generation = hit_test_generation;
        self.next_coordinate_revision = coordinate_revision;
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_paint_revision_for_test(&mut self, revision: Option<u64>) {
        self.next_paint_revision = revision;
    }

    pub(crate) fn note_focus_validation(&mut self) {
        self.phase_report = SurfacePhaseReport::one(SurfacePhase::FocusValidation);
    }

    pub(crate) const fn phase_report(&self) -> &SurfacePhaseReport {
        &self.phase_report
    }

    /// Projects current focus-selection geometry from the retained layout phase.
    pub(crate) fn current_focus_geometry(&self) -> Vec<(MountedNodeId, LogicalRect)> {
        self.cache
            .as_ref()
            .map(SurfaceCache::current_focus_geometry)
            .unwrap_or_default()
    }

    pub(crate) fn clear_cache(&mut self) {
        self.cache = None;
        if let Some(latest) = self.snapshots.back() {
            self.retired_through_generation = Some(latest.input_context().hit_test_generation());
        }
        self.snapshots.clear();
    }

    pub(crate) fn request_redraw(&mut self) -> Option<u64> {
        let next = self.redraw_revision.checked_add(1)?;
        self.redraw_revision = next;
        Some(next)
    }

    pub(crate) fn take_redraw_request(&self) -> Option<RedrawRequest> {
        (self.redraw_revision > self.redraw_acknowledged)
            .then(|| RedrawRequest::new(Arc::clone(&self.redraw_namespace), self.redraw_revision))
    }

    pub(crate) fn acknowledge_redraw(
        &mut self,
        request: &RedrawRequest,
    ) -> Result<(), RedrawAcknowledgeError> {
        if !Arc::ptr_eq(&self.redraw_namespace, &request.namespace) {
            return Err(RedrawAcknowledgeError::ForeignRuntime);
        }
        if request.revision > self.redraw_revision {
            return Err(RedrawAcknowledgeError::FutureRevision);
        }
        self.redraw_acknowledged = self.redraw_acknowledged.max(request.revision);
        Ok(())
    }

    pub(crate) const fn is_dirty(&self) -> bool {
        self.redraw_revision > self.redraw_acknowledged
    }
}
