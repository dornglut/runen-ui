use crate::mounted::{
    DirtyPhases, FinalizedSemanticPublication, MountedTree, SemanticMountedCommit,
    SurfaceCapabilityPlan,
};
use crate::scene::{HitTestSceneContent, PaintScene};
use crate::semantic_compositor::{
    SemanticCandidate, SemanticCompositionDiagnostic, SemanticOwnerFacts, compose_semantics,
};
use crate::trace::StagedMotionTraceFact;
use crate::{MountedNodeId, SemanticDiagnostic};
use runenui_core::{
    LogicalSize, LogicalTransform, OverflowPolicy, OverflowStyle, TextDisplayPosition,
};
use runenui_text::{TextCaretMap, TextDisplaySelection};
use std::{collections::HashMap, sync::Arc};

use super::{
    SurfaceCache, SurfaceMotionActivity, SurfaceMotionStore, SurfacePhaseReport,
    SurfacePlanningError, SurfacePublication,
};
use crate::scene::SceneClip;

#[derive(Clone)]
pub(crate) struct DisplayedTextTarget {
    map: TextCaretMap,
    eligible_bounds: crate::LogicalRect,
    layout_to_surface: LogicalTransform,
    clips: Arc<[SceneClip]>,
}

impl DisplayedTextTarget {
    pub(crate) fn hit_position(
        &self,
        point: crate::LogicalPoint,
    ) -> Option<(TextCaretMap, TextDisplayPosition)> {
        let position = self.hit_position_in_bounds(point)?;
        Some((self.map.clone(), position))
    }

    fn hit_position_in_bounds(&self, point: crate::LogicalPoint) -> Option<TextDisplayPosition> {
        if self
            .clips
            .iter()
            .any(|clip| !clip.contains_surface_point(point))
        {
            return None;
        }
        self.map
            .hit_test(
                self.map.snapshot(),
                point,
                self.eligible_bounds,
                self.layout_to_surface,
            )
            .ok()
            .flatten()
    }

    pub(crate) fn captured_drag_position(
        &self,
        point: crate::LogicalPoint,
    ) -> Option<(TextCaretMap, TextDisplayPosition)> {
        let position = self
            .map
            .nearest_position(self.map.snapshot(), point, self.layout_to_surface)
            .ok()?;
        Some((self.map.clone(), position))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DisplayedScrollMetrics {
    pub(crate) overflow: OverflowStyle,
    pub(crate) viewport: LogicalSize,
    pub(crate) content: LogicalSize,
}

/// Staged motion products that move through the surface-publication transaction as one unit.
///
/// Motion reconciliation remains the authority for the store, activity, and canonical
/// diagnostic facts. This bundle only preserves their candidate-local transactional
/// lifetime and introduces no second retained motion state.
pub(super) struct StagedSurfaceMotion {
    store: SurfaceMotionStore,
    activity: SurfaceMotionActivity,
    trace_facts: Vec<StagedMotionTraceFact>,
}

impl StagedSurfaceMotion {
    pub(super) const fn new(
        store: SurfaceMotionStore,
        activity: SurfaceMotionActivity,
        trace_facts: Vec<StagedMotionTraceFact>,
    ) -> Self {
        Self {
            store,
            activity,
            trace_facts,
        }
    }
}

/// Move-only candidate for one mounted-surface publication.
///
/// Planning may evaluate contractually read-only widget capabilities and may
/// retain a borrow-protected semantic-store plan, but it does not mutate the
/// live surface cache, motion store, mounted capability facts, mounted semantic
/// bindings, or dirty completion. Candidate-dependent admission can inspect this
/// object before [`Self::commit_store`] begins the final RunenUI-owned commit.
pub(crate) struct PlannedSurfacePublication<'a> {
    cache: SurfaceCache,
    motion: StagedSurfaceMotion,
    report: SurfacePhaseReport,
    completed: DirtyPhases,
    capability_plan: SurfaceCapabilityPlan,
    finalized_semantics: Option<FinalizedSemanticPublication<'a>>,
}

/// Infallible remainder of an admitted mounted-surface commit.
///
/// Construction has already committed the staged semantic store. No fallible
/// work or widget callback may be inserted between construction and [`Self::commit`].
pub(crate) struct SurfacePublicationCommit {
    cache: SurfaceCache,
    motion_store: SurfaceMotionStore,
    motion_activity: SurfaceMotionActivity,
    report: SurfacePhaseReport,
    completed: DirtyPhases,
    capability_plan: SurfaceCapabilityPlan,
    semantic_commit: Option<SemanticMountedCommit>,
}

fn project_editable_semantics(
    map: &TextCaretMap,
    projected: &crate::editing::EditingSemanticProjection,
) -> Option<(Arc<str>, runenui_core::TextSelection, Arc<[usize]>)> {
    let selection = TextDisplaySelection::from_document(projected.selection);
    map.validate_position(selection.anchor()).ok()?;
    map.validate_position(selection.active()).ok()?;
    Some((
        Arc::clone(&projected.source),
        projected.selection,
        map.__runtime_legal_byte_offsets(),
    ))
}

impl<'a> PlannedSurfacePublication<'a> {
    pub(super) const fn new(
        cache: SurfaceCache,
        motion: StagedSurfaceMotion,
        report: SurfacePhaseReport,
        completed: DirtyPhases,
        capability_plan: SurfaceCapabilityPlan,
        finalized_semantics: Option<FinalizedSemanticPublication<'a>>,
    ) -> Self {
        Self {
            cache,
            motion,
            report,
            completed,
            capability_plan,
            finalized_semantics,
        }
    }

    pub(crate) const fn publication(&self) -> &SurfacePublication {
        &self.cache.publication
    }

    pub(crate) const fn paint_scene(&self) -> &PaintScene {
        &self.cache.paint
    }

    pub(crate) const fn hit_test_content(&self) -> &HitTestSceneContent {
        &self.cache.hit_test
    }

    pub(crate) const fn motion_activity(&self) -> SurfaceMotionActivity {
        self.motion.activity
    }

    /// Returns the bounded candidate-local motion facts projected by the motion
    /// reconciliation authority. The transaction transports them only; it does
    /// not reinterpret style or motion state and retains no second diagnostic ledger.
    pub(crate) fn motion_trace_facts(&self) -> Vec<StagedMotionTraceFact> {
        self.motion.trace_facts.clone()
    }

    pub(crate) fn displayed_text_targets(
        &self,
        editing: &HashMap<MountedNodeId, crate::editing::EditingSemanticProjection>,
    ) -> HashMap<MountedNodeId, DisplayedTextTarget> {
        // A surface can be republished for hover, scrolling, or coordinate changes
        // without staging semantic reconciliation. In that case the active editing
        // projection remains the committed runtime authority for its owner; requiring
        // `finalized_semantics` here would silently drop all text hit targets from the
        // newly retained surface snapshot.
        let finalized_editables = self.finalized_semantics.as_ref().map(|finalized| {
            finalized
                .owner_facts()
                .filter_map(|owner| {
                    owner.editable.map(|editable| {
                        (
                            owner.owner,
                            (editable.snapshot, editable.text, editable.sensitivity),
                        )
                    })
                })
                .collect::<HashMap<_, _>>()
        });
        let mut targets = HashMap::new();
        for (position, topology) in self.cache.topology.nodes.iter().enumerate() {
            let Some(projected) = editing.get(&topology.id) else {
                continue;
            };
            if finalized_editables.as_ref().is_some_and(|owners| {
                owners
                    .get(&topology.id)
                    .is_none_or(|(snapshot, text, sensitivity)| {
                        (*snapshot, text.as_ref(), *sensitivity)
                            != (
                                projected.snapshot,
                                projected.source.as_ref(),
                                projected.sensitivity,
                            )
                    })
            }) {
                continue;
            }
            let Some(layout) = self.cache.layout.text_layouts.get(position) else {
                continue;
            };
            let Ok(map) = layout.caret_map_for_source(projected.snapshot, &projected.source) else {
                continue;
            };
            let presentation = self.cache.presentation.node(position);
            let padding = self
                .cache
                .effective
                .node(position)
                .computed_style()
                .padding()
                .unwrap_or_default();
            let Ok(text_origin) =
                LogicalTransform::translation(padding.left().get(), padding.top().get())
            else {
                continue;
            };
            let Ok(layout_to_surface) = text_origin.then(presentation.content_to_surface()) else {
                continue;
            };
            targets.insert(
                topology.id.clone(),
                DisplayedTextTarget {
                    map,
                    eligible_bounds: presentation.visible_bounds(),
                    layout_to_surface,
                    clips: Arc::from(presentation.content_clips().to_vec()),
                },
            );
        }
        targets
    }

    pub(crate) fn displayed_scroll_metrics(
        &self,
    ) -> HashMap<MountedNodeId, DisplayedScrollMetrics> {
        let mut metrics = HashMap::new();
        for (position, topology) in self.cache.topology.nodes.iter().enumerate() {
            if topology.overflow.horizontal() != OverflowPolicy::Scroll
                && topology.overflow.vertical() != OverflowPolicy::Scroll
            {
                continue;
            }
            let Some(layout) = self.cache.layout.bounds.get(position) else {
                continue;
            };
            let Some(layout_node) = self.cache.layout.report.node(&topology.id) else {
                continue;
            };
            metrics.insert(
                topology.id.clone(),
                DisplayedScrollMetrics {
                    overflow: topology.overflow,
                    viewport: layout.size(),
                    content: layout_node.scrollable_extent(),
                },
            );
        }
        metrics
    }

    /// Composes the renderer-independent semantic candidate and semantic-owner
    /// withdrawal diagnostics from staged publication facts while the semantic-
    /// store plan still protects exact owner/key identity. No live mounted
    /// capability or renderer output is read.
    pub(crate) fn semantic_candidate(
        &self,
        focused_owner: Option<&MountedNodeId>,
        editing: &HashMap<MountedNodeId, crate::editing::EditingSemanticProjection>,
    ) -> Result<Option<(SemanticCandidate, Vec<SemanticDiagnostic>)>, SurfacePlanningError> {
        let Some(finalized) = self.finalized_semantics.as_ref() else {
            return Ok(None);
        };
        let finalized = finalized.owner_facts().collect::<Vec<_>>();
        let expected = self.cache.topology.nodes.len();
        if finalized.len() != expected
            || self.cache.layout.bounds.len() != expected
            || self.cache.presentation.nodes.len() != expected
        {
            return Err(SurfacePlanningError::SemanticIntegrity);
        }
        let mut owners = Vec::with_capacity(expected);
        let mut owner_transforms = Vec::with_capacity(expected);
        let mut diagnostics = Vec::new();
        for (position, (topology, semantic)) in
            self.cache.topology.nodes.iter().zip(finalized).enumerate()
        {
            if topology.id != semantic.owner {
                return Err(SurfacePlanningError::SemanticIntegrity);
            }
            if let Some(reason) = semantic.withdrawal_reason {
                diagnostics.push(SemanticDiagnostic::OwnerWithdrawn {
                    authored_id: topology.authored_id.clone(),
                    reason,
                });
            }
            let presentation = self.cache.presentation.node(position);
            let (editable_source, editable_selection, editable_caret_offsets) = semantic
                .editable
                .as_ref()
                .zip(editing.get(&semantic.owner))
                .and_then(|(authored, projected)| {
                    if (
                        authored.snapshot,
                        authored.text.as_ref(),
                        authored.sensitivity,
                    ) != (
                        projected.snapshot,
                        projected.source.as_ref(),
                        projected.sensitivity,
                    ) {
                        return None;
                    }
                    let map = self
                        .cache
                        .layout
                        .text_layouts
                        .get(position)?
                        .caret_map_for_source(projected.snapshot, &projected.source)
                        .ok()?;
                    project_editable_semantics(&map, projected)
                })
                .map_or((None, None, None), |(source, selection, offsets)| {
                    (Some(source), Some(selection), Some(offsets))
                });
            owners.push(SemanticOwnerFacts {
                id: semantic.owner,
                authored_id: topology.authored_id.clone(),
                mounted_children: topology.children.clone(),
                contribution: semantic.contribution,
                bindings: semantic.bindings,
                bounds: presentation.visible_bounds(),
                activation: semantic.activation,
                focusability: semantic.focusability,
                editable_source,
                editable_selection,
                editable_caret_offsets,
            });
            owner_transforms.push(presentation.owner_to_surface());
        }
        let root = self.cache.topology.nodes.first().map(|node| &node.id);
        let candidate = compose_semantics(&owners, &owner_transforms, root, focused_owner);
        if candidate.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic,
                SemanticCompositionDiagnostic::UnrepresentableBounds { .. }
            )
        }) {
            return Err(SurfacePlanningError::PresentationGeometry);
        }
        Ok(Some((candidate, diagnostics)))
    }

    /// Begins the final commit by consuming the borrow-protected semantic-store
    /// plan. The returned value contains only infallible local commit work.
    pub(crate) fn commit_store(self) -> SurfacePublicationCommit {
        let Self {
            cache,
            motion,
            report,
            completed,
            capability_plan,
            finalized_semantics,
        } = self;
        let StagedSurfaceMotion {
            store: motion_store,
            activity: motion_activity,
            trace_facts: _,
        } = motion;
        let semantic_commit = finalized_semantics.map(FinalizedSemanticPublication::commit_store);
        SurfacePublicationCommit {
            cache,
            motion_store,
            motion_activity,
            report,
            completed,
            capability_plan,
            semantic_commit,
        }
    }
}

impl SurfacePublicationCommit {
    pub(crate) fn commit<Action>(
        self,
        tree: &mut MountedTree<Action>,
        live_cache: &mut Option<SurfaceCache>,
        live_motion_store: &mut SurfaceMotionStore,
    ) -> (
        SurfacePublication,
        SurfacePhaseReport,
        SurfaceMotionActivity,
    ) {
        let Self {
            cache,
            motion_store,
            motion_activity,
            report,
            completed,
            capability_plan,
            semantic_commit,
        } = self;
        for (owner, offset) in cache.scroll.offsets() {
            let _ = tree.commit_scroll_offset(owner, *offset);
        }
        if let Some(semantic_commit) = semantic_commit {
            tree.commit_semantic_publication(semantic_commit);
        }
        tree.commit_surface_publication_capabilities(capability_plan);
        tree.finish_publication(completed);
        let publication = cache.publication.clone();
        *live_cache = Some(cache);
        *live_motion_store = motion_store;
        (publication, report, motion_activity)
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        StyleEnvironment, TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot,
        TextPosition, TextSelection, TextSensitivity, View, text,
    };

    use super::{project_editable_semantics, super::{
        SurfaceBuildContext, SurfaceInteractionProjection, SurfaceMotionStore,
        plan_mounted_surface_cached,
    }};
    use crate::{
        LayoutConstraints,
        editing::EditingSemanticProjection,
        mounted::{DirtyPhases, MountedTree},
    };

    #[test]
    fn semantic_editable_projection_reuses_retained_legal_offset_allocation() {
        let (mut tree, _) = MountedTree::<()>::mount(text("abc").key("root").into_element());
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
        let interaction = SurfaceInteractionProjection::default();
        let planned = plan_mounted_surface_cached(&mut tree, &context, &interaction, None)
            .unwrap_or_else(|_| unreachable!("controlled text surface plan"));

        let snapshot = TextDocumentSnapshot::new(
            TextDocumentId::new(266),
            TextDocumentRevision::new(1),
        );
        let source = Arc::<str>::from("abc");
        let position = TextPosition::new(snapshot, &source, 1, TextAffinity::Downstream)
            .unwrap_or_else(|_| unreachable!("ASCII fixture position is valid"));
        let projected = EditingSemanticProjection {
            snapshot,
            source,
            selection: TextSelection::collapsed(position),
            sensitivity: TextSensitivity::Public,
        };
        let map = planned.cache.layout.text_layouts[0]
            .caret_map_for_source(snapshot, &projected.source)
            .unwrap_or_else(|_| unreachable!("planned text layout matches editable source"));
        let retained = map.__runtime_legal_byte_offsets();
        let (_, selection, published) = project_editable_semantics(&map, &projected)
            .unwrap_or_else(|| unreachable!("controlled editable projection is valid"));

        assert_eq!(selection, projected.selection);
        assert!(Arc::ptr_eq(&retained, &published));
    }

    #[test]
    fn planning_keeps_surface_cache_motion_and_dirty_completion_uncommitted() {
        let (mut tree, _) = MountedTree::<()>::mount(text("staged").key("root").into_element());
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
        let mut cache = None;
        let mut motion_store = SurfaceMotionStore::default();
        let dirty_before = tree.pending_phases();
        let interaction = SurfaceInteractionProjection::default();

        let planned =
            plan_mounted_surface_cached(&mut tree, &context, &interaction, cache.as_ref())
                .unwrap_or_else(|_| unreachable!("valid staged surface plan"));
        assert!(!planned.publication().frame().is_empty());
        drop(planned);
        assert!(cache.is_none());
        assert_eq!(motion_store, SurfaceMotionStore::default());
        assert_eq!(tree.pending_phases(), dirty_before);

        let planned =
            plan_mounted_surface_cached(&mut tree, &context, &interaction, cache.as_ref())
                .unwrap_or_else(|_| unreachable!("valid staged surface plan"));
        let commit = planned.commit_store();
        assert!(cache.is_none());
        assert_eq!(motion_store, SurfaceMotionStore::default());
        assert_eq!(tree.pending_phases(), dirty_before);

        let (publication, report, _activity) =
            commit.commit(&mut tree, &mut cache, &mut motion_store);
        assert!(!publication.frame().is_empty());
        assert!(!report.executed().is_empty());
        assert!(cache.is_some());
        assert_eq!(tree.pending_phases(), DirtyPhases::default());
    }
}
