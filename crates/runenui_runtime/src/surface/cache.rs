use std::sync::Arc;

use runenui_core::{StyleTokens, WidgetDiagnostic, WidgetPaintProof};

use crate::{AxisConstraints, AxisLimit, LogicalRect, LogicalSize, MountedNodeId};

use super::{
    SurfaceBuildContext, SurfaceLayoutReport, SurfacePublication,
    resolve::{CachedStyleFacts, SurfaceTopologySnapshot},
};

#[cfg(test)]
std::thread_local! {
    static PHASE_FUNCTION_COUNTS: std::cell::Cell<[usize; 7]> = const {
        std::cell::Cell::new([0; 7])
    };
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfacePhase {
    Tree,
    Style,
    Layout,
    HitTesting,
    Paint,
    Semantics,
    Diagnostics,
    FocusValidation,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SurfacePhaseReport {
    executed: Vec<SurfacePhase>,
}

impl SurfacePhaseReport {
    #[must_use]
    pub fn executed(&self) -> &[SurfacePhase] {
        &self.executed
    }
    #[must_use]
    pub fn contains(&self, phase: SurfacePhase) -> bool {
        self.executed.contains(&phase)
    }

    pub(crate) fn one(phase: SurfacePhase) -> Self {
        Self {
            executed: vec![phase],
        }
    }

    pub(super) fn record(&mut self, phase: SurfacePhase) {
        if !self.executed.contains(&phase) {
            self.executed.push(phase);
        }
    }
}

#[cfg(test)]
fn note_phase_function_execution(index: usize) {
    PHASE_FUNCTION_COUNTS.with(|counts| {
        let mut next = counts.get();
        next[index] += 1;
        counts.set(next);
    });
}

#[cfg(test)]
pub(super) fn note_tree_phase_execution() {
    note_phase_function_execution(0);
}

#[cfg(test)]
pub(super) fn note_style_phase_execution() {
    note_phase_function_execution(1);
}

#[cfg(test)]
pub(super) fn note_layout_phase_execution() {
    note_phase_function_execution(2);
}

#[cfg(test)]
pub(super) fn note_hit_test_phase_execution() {
    note_phase_function_execution(3);
}

#[cfg(test)]
pub(super) fn note_paint_phase_execution() {
    note_phase_function_execution(4);
}

#[cfg(test)]
pub(super) fn note_semantics_phase_execution() {
    note_phase_function_execution(5);
}

#[cfg(test)]
pub(super) fn note_diagnostics_phase_execution() {
    note_phase_function_execution(6);
}

#[cfg(test)]
pub(super) fn reset_phase_function_counts() {
    PHASE_FUNCTION_COUNTS.with(|counts| counts.set([0; 7]));
}

#[cfg(test)]
pub(super) fn phase_function_counts() -> [usize; 7] {
    PHASE_FUNCTION_COUNTS.with(std::cell::Cell::get)
}

#[derive(Clone, Debug, Eq, PartialEq)]
// Publication context key: normalized root constraints.
pub(super) struct RootConstraintKey([u32; 4]);

#[derive(Clone, Debug, PartialEq)]
// Publication context key: exact token content plus a diagnostic revision hint.
pub(super) struct StyleTokensCacheKey {
    pub(super) snapshot: StyleTokens,
    pub(super) revision: u64,
}

impl StyleTokensCacheKey {
    pub(super) fn content_differs(&self, other: &Self) -> bool {
        if self.revision == other.revision && self.snapshot == other.snapshot {
            return false;
        }
        self.snapshot != other.snapshot
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SurfaceContextKey {
    // Every field is a context key, not a mounted or phase-owned authored fact.
    pub(super) constraints: RootConstraintKey,
    pub(super) style_tokens: StyleTokensCacheKey,
    pub(super) measurement_identity: u64,
    pub(super) measurement_revision: u64,
}

#[derive(Clone, Debug)]
pub(super) struct CachedLayoutFacts {
    // Layout-phase facts: invalid whenever layout executes.
    pub(super) size: LogicalSize,
    pub(super) bounds: Vec<LogicalRect>,
    pub(super) report: SurfaceLayoutReport,
}

#[derive(Clone, Debug)]
pub(super) struct CachedHitTestFacts {
    // Layout-phase fact projected by the explicit hit-test phase.
    pub(super) bounds: Vec<LogicalRect>,
}

pub(super) fn build_hit_test_facts(layout: &CachedLayoutFacts) -> CachedHitTestFacts {
    #[cfg(test)]
    note_hit_test_phase_execution();
    CachedHitTestFacts {
        bounds: layout.bounds.clone(),
    }
}

/// Sole retained renderer-side publication substrate.
///
/// Every phase product is immutable once retained. Non-structural planning
/// stages by cloning these handles and replaces only the products owned by
/// phases that actually execute. This keeps rollback behavior structural rather
/// than relying on a deep clone of the complete surface state.
pub(crate) struct SurfaceCache {
    // Context key.
    pub(super) context_key: Arc<SurfaceContextKey>,
    // Topology facts.
    pub(super) topology: Arc<SurfaceTopologySnapshot>,
    // Style-phase facts.
    pub(super) styles: Arc<CachedStyleFacts>,
    // Layout-phase facts. This is the single retained geometry storage owner
    // used by layout publication and current directional-focus projection.
    pub(super) layout: Arc<CachedLayoutFacts>,
    // Layout-phase hit-test projection.
    pub(super) hit_test: Arc<CachedHitTestFacts>,
    // Paint-phase facts.
    pub(super) paint: Arc<Vec<WidgetPaintProof>>,
    // Diagnostic-phase facts.
    pub(super) diagnostics: Arc<Vec<Vec<WidgetDiagnostic>>>,
    // Derived materialization of the aligned phase facts above, never separate
    // authority. Its clone is cheap immutable snapshot sharing.
    // No authored StyleIntent or LayoutStyle is retained here.
    pub(super) publication: SurfacePublication,
}

impl SurfaceCache {
    /// Creates a staged non-structural candidate by sharing every retained
    /// product. Dirty phase execution must replace the corresponding handle
    /// explicitly before this candidate can commit.
    pub(super) fn staged(&self) -> Self {
        Self {
            context_key: Arc::clone(&self.context_key),
            topology: Arc::clone(&self.topology),
            styles: Arc::clone(&self.styles),
            layout: Arc::clone(&self.layout),
            hit_test: Arc::clone(&self.hit_test),
            paint: Arc::clone(&self.paint),
            diagnostics: Arc::clone(&self.diagnostics),
            publication: self.publication.clone(),
        }
    }

    /// Projects current directional-focus geometry from the retained layout
    /// phase. The displayed-input snapshot remains separate until M6B because
    /// it still owns historical input-generation membership and point routing.
    pub(crate) fn current_focus_geometry(&self) -> Vec<(MountedNodeId, LogicalRect)> {
        self.topology
            .nodes
            .iter()
            .zip(&self.layout.bounds)
            .map(|(node, bounds)| (node.id.clone(), *bounds))
            .collect()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn replace_focus_geometry_for_test(
        &mut self,
        geometry: &[(MountedNodeId, LogicalRect)],
    ) {
        let layout = Arc::make_mut(&mut self.layout);
        for (id, bounds) in geometry {
            let position = self
                .topology
                .nodes
                .iter()
                .position(|node| &node.id == id)
                .unwrap_or_else(|| unreachable!("test geometry names a published node"));
            layout.bounds[position] = *bounds;
        }
    }

    #[cfg(test)]
    pub(super) fn retained_product_reuse(&self, other: &Self) -> [bool; 7] {
        [
            Arc::ptr_eq(&self.topology, &other.topology),
            Arc::ptr_eq(&self.styles, &other.styles),
            Arc::ptr_eq(&self.layout, &other.layout),
            Arc::ptr_eq(&self.hit_test, &other.hit_test),
            Arc::ptr_eq(&self.paint, &other.paint),
            Arc::ptr_eq(&self.diagnostics, &other.diagnostics),
            self.publication.shares_storage_with(&other.publication),
        ]
    }
}

pub(super) fn context_key(context: &SurfaceBuildContext<'_>) -> SurfaceContextKey {
    const fn axis(axis: AxisConstraints) -> [u32; 2] {
        [
            axis.min().get().to_bits(),
            match axis.max() {
                AxisLimit::Finite(value) => value.get().to_bits(),
                AxisLimit::Unbounded => f32::INFINITY.to_bits(),
            },
        ]
    }
    let horizontal = axis(context.root_constraints().horizontal());
    let vertical = axis(context.root_constraints().vertical());
    SurfaceContextKey {
        constraints: RootConstraintKey([horizontal[0], horizontal[1], vertical[0], vertical[1]]),
        style_tokens: StyleTokensCacheKey {
            snapshot: context.style_tokens().clone(),
            revision: context.style_tokens().revision(),
        },
        measurement_identity: context.measurement_provider().cache_identity(),
        measurement_revision: context.measurement_provider().cache_revision(),
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{StyleTokens, View, WidgetInvalidation, text};

    use super::{SurfaceCache, SurfacePhase};
    use crate::{
        LayoutConstraints,
        mounted::{DirtyPhases, MountedTree, apply_invalidation},
        surface::{SurfaceBuildContext, plan_mounted_surface_cached},
    };

    fn publish(
        tree: &mut MountedTree<()>,
        context: &SurfaceBuildContext<'_>,
        cache: &mut Option<SurfaceCache>,
    ) -> super::SurfacePhaseReport {
        let planned = plan_mounted_surface_cached(tree, context, cache.as_ref())
            .unwrap_or_else(|_| unreachable!("reuse proof has valid semantic planning"));
        let commit = planned.commit_store();
        let (_, report) = commit.commit(tree, cache);
        report
    }

    fn retained(cache: Option<&SurfaceCache>) -> SurfaceCache {
        cache
            .unwrap_or_else(|| unreachable!("initial publication retains a cache"))
            .staged()
    }

    #[test]
    fn focus_derived_interaction_publication_reuses_unrelated_products() {
        let (mut tree, _) = MountedTree::<()>::mount(text("focus").key("root").into_element());
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let root = tree.publication_preorder_ids()[0].clone();
        let before = retained(cache.as_ref());

        let node = tree
            .node_mut(&root)
            .unwrap_or_else(|| unreachable!("focus proof root remains live"));
        apply_invalidation(node, WidgetInvalidation::INTERACTION);
        tree.finish_focus_validation();

        let report = publish(&mut tree, &context, &mut cache);
        assert_eq!(
            report.executed(),
            &[SurfacePhase::Paint, SurfacePhase::Semantics]
        );
        let after = cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("focus publication retains a cache"));
        assert_eq!(
            before.retained_product_reuse(after),
            [true, true, true, true, false, true, false]
        );
    }

    #[test]
    fn dropped_dirty_non_structural_plan_leaves_live_cache_and_dirty_work_unchanged() {
        let (mut tree, _) = MountedTree::<()>::mount(text("rollback").key("root").into_element());
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let root = tree.publication_preorder_ids()[0].clone();
        let before = retained(cache.as_ref());

        let node = tree
            .node_mut(&root)
            .unwrap_or_else(|| unreachable!("rollback proof root remains live"));
        apply_invalidation(node, WidgetInvalidation::PAINT);
        let dirty_before = tree.pending_phases();
        assert!(dirty_before.contains(DirtyPhases::PAINT));

        let planned = plan_mounted_surface_cached(&mut tree, &context, cache.as_ref())
            .unwrap_or_else(|_| unreachable!("dirty staged plan remains valid"));
        drop(planned);

        let still_live = cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("dropped plan leaves live cache retained"));
        assert_eq!(before.retained_product_reuse(still_live), [true; 7]);
        assert_eq!(tree.pending_phases(), dirty_before);

        let report = publish(&mut tree, &context, &mut cache);
        assert_eq!(report.executed(), &[SurfacePhase::Paint]);
        let after = cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("successful retry retains a cache"));
        assert_eq!(
            before.retained_product_reuse(after),
            [true, true, true, true, false, true, false]
        );
    }
}
