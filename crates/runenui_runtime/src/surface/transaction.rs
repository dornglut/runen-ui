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
use runenui_core::TextDisplayPosition;
use runenui_text::TextDisplaySelection;
use std::{collections::HashMap, sync::Arc};

use super::{
    SurfaceCache, SurfaceMotionActivity, SurfaceMotionStore, SurfacePhaseReport,
    SurfacePlanningError, SurfacePublication,
};

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
                    let selection = TextDisplaySelection::from_document(projected.selection);
                    map.validate_position(selection.anchor()).ok()?;
                    map.validate_position(selection.active()).ok()?;
                    let mut offsets = Vec::new();
                    for position in map.legal_positions() {
                        if let TextDisplayPosition::Document(position) = position
                            && offsets.last() != Some(&position.byte_offset())
                        {
                            offsets.push(position.byte_offset());
                        }
                    }
                    Some((
                        Arc::clone(&projected.source),
                        projected.selection,
                        Arc::<[usize]>::from(offsets),
                    ))
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
                bounds: presentation.owner_bounds(),
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
    use runenui_core::{StyleEnvironment, View, text};

    use super::super::{
        SurfaceBuildContext, SurfaceInteractionProjection, SurfaceMotionStore,
        plan_mounted_surface_cached,
    };
    use crate::{
        LayoutConstraints,
        mounted::{DirtyPhases, MountedTree},
    };

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
