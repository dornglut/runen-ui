use std::sync::Arc;

use runenui_core::{StyleEnvironment, WidgetDiagnostic};
use runenui_text::{FontSourceSnapshot, TextLayoutState};

use crate::scene::{HitTestSceneContent, PaintScene};
use crate::{AxisConstraints, AxisLimit, LogicalRect, LogicalSize, MountedNodeId};

use super::{
    SurfaceBuildContext, SurfaceInteractionProjection, SurfaceLayoutReport, SurfacePublication,
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
// Publication context key: complete exact style-environment content.
pub(super) struct StyleEnvironmentCacheKey {
    pub(super) snapshot: StyleEnvironment,
}

impl StyleEnvironmentCacheKey {
    pub(super) fn content_differs(&self, other: &Self) -> bool {
        self.snapshot != other.snapshot
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SurfaceContextKey {
    // Every field is a context key, not a mounted or phase-owned authored fact.
    pub(super) constraints: RootConstraintKey,
    pub(super) style_environment: StyleEnvironmentCacheKey,
    pub(super) font_source: FontSourceSnapshot,
}

#[derive(Clone, Debug)]
pub(super) struct CachedLayoutFacts {
    // Layout-phase facts: invalid whenever layout executes.
    pub(super) size: LogicalSize,
    pub(super) bounds: Vec<LogicalRect>,
    pub(super) report: SurfaceLayoutReport,
    // Runtime-owned reusable logical text state aligned exactly with topology.
    // Each state is cheap COW sharing so a staged reflow cannot mutate accepted
    // shaping/layout state before publication commit.
    pub(super) text_layouts: Vec<TextLayoutState>,
}

/// Sole retained renderer/input-side publication substrate.
///
/// Every phase product is immutable once retained. Non-structural planning
/// stages by cloning these handles and replaces only products owned by phases
/// that actually execute. Canonical paint and hit scene content are retained
/// directly; there is no proof-era paint vector or layout-derived hit snapshot.
pub(crate) struct SurfaceCache {
    // Context key.
    pub(super) context_key: Arc<SurfaceContextKey>,
    // Topology facts.
    pub(super) topology: Arc<SurfaceTopologySnapshot>,
    // Last runtime-derived interaction projection consumed by the style phase.
    // This is cache compatibility only, never pointer/focus authority.
    pub(super) interaction: Arc<SurfaceInteractionProjection>,
    // Style-phase facts.
    pub(super) styles: Arc<CachedStyleFacts>,
    // Layout-phase facts. This is the single retained geometry storage owner
    // used by layout publication and current directional-focus projection.
    pub(super) layout: Arc<CachedLayoutFacts>,
    // Canonical physical-hit content; displayed context is added only by the
    // runtime-owned publication state when a generation is committed.
    pub(super) hit_test: HitTestSceneContent,
    // Canonical renderer-neutral paint scene content.
    pub(super) paint: PaintScene,
    // Widget diagnostic-phase facts.
    pub(super) diagnostics: Arc<Vec<Vec<WidgetDiagnostic>>>,
    // Hit-composition diagnostics are owned and replaced with the hit phase.
    pub(super) hit_diagnostics: Arc<Vec<Vec<WidgetDiagnostic>>>,
    // Paint-composition diagnostics are owned and replaced with the paint phase.
    pub(super) paint_diagnostics: Arc<Vec<Vec<WidgetDiagnostic>>>,
    // Derived layout/debug materialization of aligned phase facts above, never
    // renderer or pointer authority. Its clone is cheap immutable sharing.
    // No authored StyleIntent or LayoutStyle is retained here.
    pub(super) publication: SurfacePublication,
}

impl SurfaceCache {
    /// Creates a staged non-structural candidate by sharing every retained
    /// product. Dirty phase execution must replace the corresponding product
    /// explicitly before this candidate can commit.
    pub(super) fn staged(&self) -> Self {
        Self {
            context_key: Arc::clone(&self.context_key),
            topology: Arc::clone(&self.topology),
            interaction: Arc::clone(&self.interaction),
            styles: Arc::clone(&self.styles),
            layout: Arc::clone(&self.layout),
            hit_test: self.hit_test.clone(),
            paint: self.paint.clone(),
            diagnostics: Arc::clone(&self.diagnostics),
            hit_diagnostics: Arc::clone(&self.hit_diagnostics),
            paint_diagnostics: Arc::clone(&self.paint_diagnostics),
            publication: self.publication.clone(),
        }
    }

    /// Projects current directional-focus geometry from the retained layout
    /// phase, independently of physical hit participation.
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
            self.hit_test.shares_storage_with(&other.hit_test),
            self.paint.shares_storage_with(&other.paint),
            Arc::ptr_eq(&self.diagnostics, &other.diagnostics)
                && Arc::ptr_eq(&self.hit_diagnostics, &other.hit_diagnostics)
                && Arc::ptr_eq(&self.paint_diagnostics, &other.paint_diagnostics),
            self.publication.shares_storage_with(&other.publication),
        ]
    }
}

pub(super) fn context_key(
    context: &SurfaceBuildContext<'_>,
    font_source: FontSourceSnapshot,
) -> SurfaceContextKey {
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
        style_environment: StyleEnvironmentCacheKey {
            snapshot: context.style_environment().clone(),
        },
        font_source,
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{StyleEnvironment, View, WidgetInvalidation, text};

    use super::{SurfaceCache, SurfacePhase};
    use crate::{
        LayoutConstraints,
        mounted::{DirtyPhases, MountedTree, apply_invalidation},
        surface::{
            SurfaceBuildContext, SurfaceInteractionProjection,
            planning::plan_mounted_surface_cached_with_test_text,
        },
    };

    fn publish(
        tree: &mut MountedTree<()>,
        context: &SurfaceBuildContext<'_>,
        cache: &mut Option<SurfaceCache>,
    ) -> super::SurfacePhaseReport {
        let interaction = SurfaceInteractionProjection::default();
        let planned = plan_mounted_surface_cached_with_test_text(
            tree,
            context,
            &interaction,
            cache.as_ref(),
        )
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
    fn focus_only_publication_reuses_all_renderer_products() {
        let (mut tree, _) = MountedTree::<()>::mount(text("focus").key("root").into_element());
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let before = retained(cache.as_ref());

        tree.mark_semantic_focus_product_dirty();
        let report = publish(&mut tree, &context, &mut cache);
        let after = cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("focus publication retains a cache"));

        assert_eq!(report.executed(), &[SurfacePhase::Semantics]);
        assert_eq!(before.retained_product_reuse(after), [true; 7]);
    }

    #[test]
    fn dropped_dirty_non_structural_plan_leaves_live_cache_and_dirty_work_unchanged() {
        let (mut tree, _) = MountedTree::<()>::mount(text("rollback").key("root").into_element());
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
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

        let interaction = SurfaceInteractionProjection::default();
        let planned = plan_mounted_surface_cached_with_test_text(
            &mut tree,
            &context,
            &interaction,
            cache.as_ref(),
        )
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
            [true, true, true, true, false, true, true]
        );
    }
}
