use crate::mounted::{
    DirtyPhases, FinalizedSemanticPublication, MountedTree, SemanticMountedCommit,
    SurfaceCapabilityPlan,
};
use crate::scene::{HitTestSceneContent, PaintScene};
use crate::semantic_compositor::{SemanticCandidate, SemanticOwnerFacts, compose_semantics};
use crate::{MountedNodeId, SemanticDiagnostic};

use super::{SurfaceCache, SurfacePhaseReport, SurfacePlanningError, SurfacePublication};

/// Move-only candidate for one mounted-surface publication.
///
/// Planning may evaluate contractually read-only widget capabilities and may
/// retain a borrow-protected semantic-store plan, but it does not mutate the
/// live surface cache, mounted capability facts, mounted semantic bindings, or
/// dirty completion. Candidate-dependent admission can inspect this object
/// before [`Self::commit_store`] begins the final RunenUI-owned commit.
pub(crate) struct PlannedSurfacePublication<'a> {
    cache: SurfaceCache,
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
    report: SurfacePhaseReport,
    completed: DirtyPhases,
    capability_plan: SurfaceCapabilityPlan,
    semantic_commit: Option<SemanticMountedCommit>,
}

impl<'a> PlannedSurfacePublication<'a> {
    pub(super) const fn new(
        cache: SurfaceCache,
        report: SurfacePhaseReport,
        completed: DirtyPhases,
        capability_plan: SurfaceCapabilityPlan,
        finalized_semantics: Option<FinalizedSemanticPublication<'a>>,
    ) -> Self {
        Self {
            cache,
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

    /// Composes the renderer-independent semantic candidate and semantic-owner
    /// withdrawal diagnostics from staged publication facts while the semantic-
    /// store plan still protects exact owner/key identity. No live mounted
    /// capability or renderer output is read.
    pub(crate) fn semantic_candidate(
        &self,
        focused_owner: Option<&MountedNodeId>,
    ) -> Result<Option<(SemanticCandidate, Vec<SemanticDiagnostic>)>, SurfacePlanningError> {
        let Some(finalized) = self.finalized_semantics.as_ref() else {
            return Ok(None);
        };
        let finalized = finalized.owner_facts().collect::<Vec<_>>();
        let expected = self.cache.topology.nodes.len();
        if finalized.len() != expected || self.cache.layout.bounds.len() != expected {
            return Err(SurfacePlanningError::SemanticIntegrity);
        }
        let mut owners = Vec::with_capacity(expected);
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
            owners.push(SemanticOwnerFacts {
                id: semantic.owner,
                authored_id: topology.authored_id.clone(),
                mounted_children: topology.children.clone(),
                contribution: semantic.contribution,
                bindings: semantic.bindings,
                bounds: self.cache.layout.bounds[position],
                activation: semantic.activation,
                focusability: semantic.focusability,
            });
        }
        let root = self.cache.topology.nodes.first().map(|node| &node.id);
        Ok(Some((
            compose_semantics(&owners, root, focused_owner),
            diagnostics,
        )))
    }

    /// Begins the final commit by consuming the borrow-protected semantic-store
    /// plan. The returned value contains only infallible local commit work.
    pub(crate) fn commit_store(self) -> SurfacePublicationCommit {
        let Self {
            cache,
            report,
            completed,
            capability_plan,
            finalized_semantics,
        } = self;
        let semantic_commit = finalized_semantics.map(FinalizedSemanticPublication::commit_store);
        SurfacePublicationCommit {
            cache,
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
    ) -> (SurfacePublication, SurfacePhaseReport) {
        let Self {
            cache,
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
        (publication, report)
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{StyleEnvironment, View, text};

    use super::super::{
        SurfaceBuildContext, SurfaceInteractionProjection, plan_mounted_surface_cached,
    };
    use crate::{
        LayoutConstraints,
        mounted::{DirtyPhases, MountedTree},
    };

    #[test]
    fn planning_keeps_surface_cache_and_dirty_completion_uncommitted() {
        let (mut tree, _) = MountedTree::<()>::mount(text("staged").key("root").into_element());
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
        let mut cache = None;
        let dirty_before = tree.pending_phases();
        let interaction = SurfaceInteractionProjection::default();

        let planned =
            plan_mounted_surface_cached(&mut tree, &context, &interaction, cache.as_ref())
                .unwrap_or_else(|_| unreachable!("valid staged surface plan"));
        assert!(!planned.publication().frame().is_empty());
        drop(planned);
        assert!(cache.is_none());
        assert_eq!(tree.pending_phases(), dirty_before);

        let planned =
            plan_mounted_surface_cached(&mut tree, &context, &interaction, cache.as_ref())
                .unwrap_or_else(|_| unreachable!("valid staged surface plan"));
        let commit = planned.commit_store();
        assert!(cache.is_none());
        assert_eq!(tree.pending_phases(), dirty_before);

        let (publication, report) = commit.commit(&mut tree, &mut cache);
        assert!(!publication.frame().is_empty());
        assert!(!report.executed().is_empty());
        assert!(cache.is_some());
        assert_eq!(tree.pending_phases(), DirtyPhases::default());
    }
}
