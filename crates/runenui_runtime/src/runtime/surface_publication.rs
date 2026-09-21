use core::num::NonZeroUsize;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use runenui_core::{
    __runtime::RuntimeNamespace, MonotonicInstant, SurfaceId, SurfaceInputContext,
    TextDocumentSnapshot,
};
use runenui_text::{TextCaretMap, TextCaretMapError, TextLayoutError, TextSystem};

use crate::{
    LogicalPoint, LogicalRect, LogicalSize, MountedNodeId, RedrawAcknowledgeError, RedrawRequest,
    SemanticDiagnosticReport, SemanticPublication, SurfaceBuildContext, SurfacePhase,
    SurfacePhaseReport, SurfacePublication, SurfacePublicationCounter, TraceSurfaceContext,
    TraceSurfaceSnapshotKind,
    mounted::MountedTree,
    scene::{HitTestScene, PaintPublication, PaintRevision},
    semantic_publication::{
        SemanticPublicationPlan, SemanticPublicationPlanError, SemanticPublicationState,
    },
    surface::{
        DisplayedScrollMetrics, DisplayedTextTarget, MotionPlanningFailure,
        PlannedSurfacePublication, SurfaceCache, SurfaceInteractionProjection,
        SurfaceMotionActivity, SurfaceMotionStore, SurfacePlanningError, SurfacePublicationCommit,
        plan_mounted_surface_cached_with_text,
    },
    trace::StagedMotionTraceFact,
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

/// Exact non-mutating reservation for one future redraw revision.
#[derive(Clone, Copy)]
pub(in crate::runtime) struct RedrawRevisionAdmission {
    revision: u64,
}

/// One immutable publication candidate context derived at a single runtime instant.
pub(in crate::runtime) struct SurfacePublicationCandidateInputs<'a> {
    interaction: &'a SurfaceInteractionProjection,
    focused_owner: Option<&'a MountedNodeId>,
    editing: &'a HashMap<MountedNodeId, crate::editing::EditingSemanticProjection>,
    preedits: &'a HashMap<MountedNodeId, Arc<runenui_text::TextPreeditProjection>>,
    admission: SurfacePublicationAdmission,
    instant: MonotonicInstant,
}

impl<'a> SurfacePublicationCandidateInputs<'a> {
    pub(in crate::runtime) const fn new(
        interaction: &'a SurfaceInteractionProjection,
        focused_owner: Option<&'a MountedNodeId>,
        editing: &'a HashMap<MountedNodeId, crate::editing::EditingSemanticProjection>,
        preedits: &'a HashMap<MountedNodeId, Arc<runenui_text::TextPreeditProjection>>,
        admission: SurfacePublicationAdmission,
        instant: MonotonicInstant,
    ) -> Self {
        Self {
            interaction,
            focused_owner,
            editing,
            preedits,
            admission,
            instant,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::runtime) enum SurfacePublicationPlanError {
    SemanticIntegrity,
    TextLayout(TextLayoutError),
    PresentationGeometry,
    Motion(MotionPlanningFailure),
    CounterExhausted(SurfacePublicationCounter),
}

impl From<SurfacePlanningError> for SurfacePublicationPlanError {
    fn from(error: SurfacePlanningError) -> Self {
        match error {
            SurfacePlanningError::SemanticIntegrity => Self::SemanticIntegrity,
            SurfacePlanningError::TextLayout(error) => Self::TextLayout(error),
            SurfacePlanningError::PresentationGeometry => Self::PresentationGeometry,
            SurfacePlanningError::Motion(error) => Self::Motion(error),
        }
    }
}

/// Fully staged surface candidate. Holding this value mutates no live publication state.
pub(in crate::runtime) struct StagedSurfacePublication<'a> {
    planned: PlannedSurfacePublication<'a>,
    semantic_plan: SemanticPublicationPlan,
    semantic_publication: SemanticPublication,
    semantic_diagnostics: SemanticDiagnosticReport,
    hit_test_scene: HitTestScene,
    displayed_text_targets: HashMap<MountedNodeId, DisplayedTextTarget>,
    displayed_scroll_metrics: HashMap<MountedNodeId, DisplayedScrollMetrics>,
    paint_publication: PaintPublication,
    allocated_paint_revision: Option<u64>,
    hit_test_generation: u64,
    coordinate_revision: u64,
}

impl StagedSurfacePublication<'_> {
    pub(in crate::runtime) const fn motion_activity(&self) -> SurfaceMotionActivity {
        self.planned.motion_activity()
    }

    pub(in crate::runtime) fn motion_trace_facts(&self) -> Vec<StagedMotionTraceFact> {
        self.planned.motion_trace_facts()
    }

    /// Begins the irreversible local commit only after outer candidate-dependent admission.
    pub(in crate::runtime) fn commit_store(self) -> AdmittedSurfacePublicationCommit {
        let Self {
            planned,
            semantic_plan,
            semantic_publication,
            semantic_diagnostics,
            hit_test_scene,
            displayed_text_targets,
            displayed_scroll_metrics,
            paint_publication,
            allocated_paint_revision,
            hit_test_generation,
            coordinate_revision,
        } = self;
        AdmittedSurfacePublicationCommit {
            surface_commit: planned.commit_store(),
            semantic_plan,
            semantic_publication,
            semantic_diagnostics,
            hit_test_scene,
            displayed_text_targets,
            displayed_scroll_metrics,
            paint_publication,
            allocated_paint_revision,
            hit_test_generation,
            coordinate_revision,
        }
    }
}

/// Infallible runtime-level remainder after candidate-dependent admission.
pub(in crate::runtime) struct AdmittedSurfacePublicationCommit {
    surface_commit: SurfacePublicationCommit,
    semantic_plan: SemanticPublicationPlan,
    semantic_publication: SemanticPublication,
    semantic_diagnostics: SemanticDiagnosticReport,
    hit_test_scene: HitTestScene,
    displayed_text_targets: HashMap<MountedNodeId, DisplayedTextTarget>,
    displayed_scroll_metrics: HashMap<MountedNodeId, DisplayedScrollMetrics>,
    paint_publication: PaintPublication,
    allocated_paint_revision: Option<u64>,
    hit_test_generation: u64,
    coordinate_revision: u64,
}

/// Sole runtime-owned state for current surface publication, renderer revision,
/// redraw revision, live motion, and bounded displayed hit-test generations.
pub(crate) struct SurfacePublicationState {
    cache: Option<SurfaceCache>,
    motion_store: SurfaceMotionStore,
    motion_deadline: Option<MonotonicInstant>,
    current_paint: Option<PaintPublication>,
    semantic_publication: SemanticPublicationState,
    phase_report: SurfacePhaseReport,
    redraw_namespace: Arc<()>,
    redraw_revision: u64,
    redraw_acknowledged: u64,
    runtime_namespace: RuntimeNamespace,
    surface_id: SurfaceId,
    retained_snapshot_limit: NonZeroUsize,
    snapshots: VecDeque<RetainedSurfaceSnapshot>,
    retired_through_generation: Option<u64>,
    next_paint_revision: Option<u64>,
    next_hit_test_generation: Option<u64>,
    next_coordinate_revision: Option<u64>,
}

struct RetainedSurfaceSnapshot {
    scene: HitTestScene,
    text_targets: HashMap<MountedNodeId, DisplayedTextTarget>,
    scroll_metrics: HashMap<MountedNodeId, DisplayedScrollMetrics>,
}

impl RetainedSurfaceSnapshot {
    const fn input_context(&self) -> &SurfaceInputContext {
        self.scene.input_context()
    }
}

impl SurfacePublicationState {
    pub(in crate::runtime) const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }

    pub(in crate::runtime) fn current_surface_input_context(&self) -> Option<SurfaceInputContext> {
        self.snapshots
            .back()
            .map(|snapshot| snapshot.input_context().clone())
    }

    pub(in crate::runtime) fn text_caret_map(
        &self,
        owner: &MountedNodeId,
        snapshot: TextDocumentSnapshot,
        source: &str,
    ) -> Result<TextCaretMap, TextCaretMapError> {
        self.cache
            .as_ref()
            .ok_or(TextCaretMapError::MissingLayout)?
            .text_caret_map(owner, snapshot, source)
    }

    pub(in crate::runtime) fn text_candidate_area(
        &self,
        owner: &MountedNodeId,
        snapshot: TextDocumentSnapshot,
        source: &str,
        selection: runenui_core::TextSelection,
        preedit: Option<std::sync::Arc<runenui_text::TextPreeditProjection>>,
    ) -> Result<LogicalRect, TextCaretMapError> {
        self.cache
            .as_ref()
            .ok_or(TextCaretMapError::MissingLayout)?
            .text_candidate_area(owner, snapshot, source, selection, preedit)
    }

    pub(crate) fn new(
        runtime_namespace: RuntimeNamespace,
        retained_snapshot_limit: NonZeroUsize,
    ) -> Self {
        let surface_id = runtime_namespace.__runtime_surface_id(0, 1);
        Self {
            cache: None,
            motion_store: SurfaceMotionStore::default(),
            motion_deadline: None,
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

    pub(crate) fn plan_publication<'tree, Action>(
        &self,
        tree: &'tree mut MountedTree<Action>,
        text_system: &mut TextSystem,
        context: &SurfaceBuildContext<'_>,
        candidate: SurfacePublicationCandidateInputs<'_>,
    ) -> Result<StagedSurfacePublication<'tree>, SurfacePublicationPlanError> {
        let SurfacePublicationCandidateInputs {
            interaction,
            focused_owner,
            editing,
            preedits,
            admission,
            instant,
        } = candidate;
        let (hit_test_generation, coordinate_revision) = admission.into_parts();
        let text_editing =
            crate::surface::TextEditingPaintInputs::new(focused_owner, editing, preedits);
        let planned = plan_mounted_surface_cached_with_text(
            tree,
            context,
            interaction,
            text_system,
            text_editing,
            self.cache.as_ref(),
            &self.motion_store,
            instant,
        )?;
        let displayed_text_targets = planned.displayed_text_targets(editing);
        let displayed_scroll_metrics = planned.displayed_scroll_metrics();
        let semantic_candidate = planned.semantic_candidate(focused_owner, editing)?;
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

        Ok(StagedSurfacePublication {
            planned,
            semantic_plan,
            semantic_publication,
            semantic_diagnostics,
            hit_test_scene,
            displayed_text_targets,
            displayed_scroll_metrics,
            paint_publication,
            allocated_paint_revision,
            hit_test_generation,
            coordinate_revision,
        })
    }

    pub(crate) fn commit_publication<Action>(
        &mut self,
        tree: &mut MountedTree<Action>,
        commit: AdmittedSurfacePublicationCommit,
    ) -> SurfacePublication {
        let AdmittedSurfacePublicationCommit {
            surface_commit,
            semantic_plan,
            semantic_publication,
            semantic_diagnostics,
            hit_test_scene,
            displayed_text_targets,
            displayed_scroll_metrics,
            paint_publication,
            allocated_paint_revision,
            hit_test_generation,
            coordinate_revision,
        } = commit;
        let (products, report, motion_activity) =
            surface_commit.commit(tree, &mut self.cache, &mut self.motion_store);
        self.semantic_publication.commit(semantic_plan);
        if let Some(revision) = allocated_paint_revision {
            self.next_paint_revision = revision.checked_add(1);
            self.current_paint = Some(paint_publication.clone());
        }
        self.phase_report = report;
        self.motion_deadline = motion_activity.next_deadline();
        self.retain_new_snapshot(
            hit_test_scene.clone(),
            displayed_text_targets,
            displayed_scroll_metrics,
            hit_test_generation,
            coordinate_revision,
        );
        SurfacePublication::new(
            paint_publication,
            hit_test_scene,
            products,
            semantic_publication,
            semantic_diagnostics,
        )
    }

    fn retain_new_snapshot(
        &mut self,
        scene: HitTestScene,
        text_targets: HashMap<MountedNodeId, DisplayedTextTarget>,
        scroll_metrics: HashMap<MountedNodeId, DisplayedScrollMetrics>,
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
        self.snapshots.push_back(RetainedSurfaceSnapshot {
            scene,
            text_targets,
            scroll_metrics,
        });
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
            target: snapshot.scene.target_at(point).cloned(),
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
            .scene
            .contains_mounted_target(target)
            .then(|| Self::selection(snapshot, snapshot_kind))
            .ok_or(SurfaceSnapshotError::TargetNotInSnapshot)
    }

    const fn selection(
        snapshot: &RetainedSurfaceSnapshot,
        snapshot_kind: SurfaceSnapshotKind,
    ) -> SurfaceSnapshotSelection {
        SurfaceSnapshotSelection {
            snapshot_kind,
            hit_test_generation: snapshot.scene.input_context().hit_test_generation(),
            coordinate_revision: snapshot.scene.input_context().coordinate_revision(),
        }
    }

    fn validate_context(
        &self,
        context: &SurfaceInputContext,
    ) -> Result<(&RetainedSurfaceSnapshot, SurfaceSnapshotKind), SurfaceSnapshotError> {
        let Some((surface_id, coordinate_revision, hit_test_generation)) = self
            .runtime_namespace
            .__runtime_surface_context_parts(context)
        else {
            return Err(SurfaceSnapshotError::ForeignSurfaceContext);
        };
        if surface_id != self.surface_id {
            return Err(SurfaceSnapshotError::ForeignSurface);
        }
        let Some(snapshot) = self.snapshots.iter().find(|snapshot| {
            snapshot.scene.input_context().hit_test_generation() == hit_test_generation
        }) else {
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

    pub(in crate::runtime) fn text_map_position_at(
        &self,
        context: &SurfaceInputContext,
        owner: &MountedNodeId,
        point: LogicalPoint,
    ) -> Option<(
        runenui_text::TextCaretMap,
        runenui_core::TextDisplayPosition,
    )> {
        let (snapshot, _) = self.validate_context(context).ok()?;
        snapshot.text_targets.get(owner)?.map_and_hit_test(point)
    }

    pub(crate) fn displayed_scroll_metrics(
        &self,
        hit_test_generation: u64,
        coordinate_revision: u64,
        owner: &MountedNodeId,
    ) -> Option<DisplayedScrollMetrics> {
        let snapshot = self.snapshots.iter().find(|snapshot| {
            snapshot.scene.input_context().hit_test_generation() == hit_test_generation
                && snapshot.scene.input_context().coordinate_revision() == coordinate_revision
        })?;
        snapshot.scroll_metrics.get(owner).copied()
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
            .find(|snapshot| snapshot.scene.input_context().hit_test_generation() == generation)
            .unwrap_or_else(|| unreachable!("test context names one retained snapshot"));
        snapshot
            .scene
            .replace_target_for_test(original, replacement);
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

    #[cfg(test)]
    pub(in crate::runtime) const fn seed_redraw_revision_for_test(&mut self, revision: u64) {
        self.redraw_revision = revision;
        self.redraw_acknowledged = revision;
    }

    pub(crate) fn note_focus_validation(&mut self) {
        self.phase_report = SurfacePhaseReport::one(SurfacePhase::FocusValidation);
    }

    pub(crate) const fn phase_report(&self) -> &SurfacePhaseReport {
        &self.phase_report
    }

    /// Projects current focus-selection geometry from the retained presentation facts.
    pub(crate) fn current_focus_geometry(&self) -> Vec<(MountedNodeId, LogicalRect)> {
        self.cache
            .as_ref()
            .map(SurfaceCache::current_focus_geometry)
            .unwrap_or_default()
    }

    pub(crate) fn current_scroll_metrics(
        &self,
        owner: &MountedNodeId,
    ) -> Option<DisplayedScrollMetrics> {
        self.cache.as_ref()?.current_scroll_metrics(owner)
    }

    pub(crate) fn scroll_target_geometry(
        &self,
        target: &MountedNodeId,
        owner: &MountedNodeId,
    ) -> Option<(LogicalRect, LogicalSize)> {
        self.cache.as_ref()?.scroll_target_geometry(target, owner)
    }

    pub(crate) fn retire_motion_owner(&mut self, owner: &MountedNodeId) {
        self.motion_store.retire_owner(owner);
        self.motion_deadline = None;
    }

    pub(crate) fn clear_motion_for_shutdown(&mut self) {
        self.motion_store.clear();
        self.motion_deadline = None;
    }

    pub(crate) const fn motion_deadline(&self) -> Option<MonotonicInstant> {
        self.motion_deadline
    }

    pub(crate) fn motion_deadline_is_due(&self, now: MonotonicInstant) -> bool {
        self.motion_deadline.is_some_and(|deadline| now >= deadline)
    }

    pub(crate) fn clear_cache(&mut self) {
        self.cache = None;
        if let Some(latest) = self.snapshots.back() {
            self.retired_through_generation = Some(latest.input_context().hit_test_generation());
        }
        self.snapshots.clear();
    }

    pub(in crate::runtime) const fn admit_redraw_request(
        &self,
    ) -> Result<RedrawRevisionAdmission, SurfacePublicationCounter> {
        match self.redraw_revision.checked_add(1) {
            Some(revision) => Ok(RedrawRevisionAdmission { revision }),
            None => Err(SurfacePublicationCounter::RedrawRevision),
        }
    }

    pub(in crate::runtime) fn commit_redraw_request(
        &mut self,
        admission: RedrawRevisionAdmission,
    ) -> u64 {
        debug_assert!(
            self.redraw_revision.checked_add(1) == Some(admission.revision),
            "redraw admission names the exact next revision"
        );
        self.redraw_revision = admission.revision;
        admission.revision
    }

    pub(crate) fn request_redraw(&mut self) -> Option<u64> {
        let admission = self.admit_redraw_request().ok()?;
        Some(self.commit_redraw_request(admission))
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
