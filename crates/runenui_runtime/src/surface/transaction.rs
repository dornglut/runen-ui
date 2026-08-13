use crate::mounted::{
    DirtyPhases, FinalizedSemanticPublication, MountedTree, SemanticMountedCommit,
    SurfaceCapabilityPlan,
};
use crate::semantic_compositor::SemanticCandidate;

use super::{SurfaceCache, SurfacePhaseReport, SurfacePublication};

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
    use runenui_core::{StyleTokens, View, text};

    use super::super::{SurfaceBuildContext, plan_mounted_surface_cached};
    use crate::{
        LayoutConstraints,
        mounted::{DirtyPhases, MountedTree},
    };

    #[test]
    fn planning_keeps_surface_cache_and_dirty_completion_uncommitted() {
        let (mut tree, _) = MountedTree::<()>::mount(text("staged").key("root").into_element());
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        let dirty_before = tree.pending_phases();

        let planned = plan_mounted_surface_cached(&mut tree, &context, cache.as_ref())
            .unwrap_or_else(|_| unreachable!("valid staged surface plan"));
        assert!(!planned.publication().frame().is_empty());
        drop(planned);
        assert!(cache.is_none());
        assert_eq!(tree.pending_phases(), dirty_before);

        let planned = plan_mounted_surface_cached(&mut tree, &context, cache.as_ref())
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
