//! Surface layout/debug products and canonical scene planning.
#![allow(clippy::redundant_pub_crate)]
//!
//! `SurfaceFrame` remains an aligned layout/debug snapshot. Canonical renderer
//! paint and pointer-hit authority live in `PaintScene`/`PaintPublication` and
//! `HitTestScene`, not in this debug product.

mod arrange;
mod cache;
mod context;
mod measure;
mod resolve;
mod transaction;

use std::sync::Arc;

pub(crate) use cache::SurfaceCache;
use cache::{CachedLayoutFacts, context_key};
pub use cache::{SurfacePhase, SurfacePhaseReport};
pub use context::SurfaceBuildContext;
use measure::layout_resolved_surface;
use resolve::{
    ResolvedSurfaceTree, collect_topology, hit_contexts, paint_contexts, resolve_diagnostics,
    resolve_hit_test, resolve_paint, resolve_styles,
};
pub(crate) use transaction::PlannedSurfacePublication;

use runenui_core::{
    ComputedStyle, ElementId, LogicalLength, LogicalRect, LogicalSize, WidgetDiagnostic,
    WidgetTypeId,
};

use crate::mounted::{DirtyPhases, SemanticReconcileError, SurfaceCapabilityPlan};
use crate::style_debug::SurfaceStyleReport;
use crate::{LayoutConstraints, MountedNodeId};

/// One ordered node in the non-renderer layout/debug surface frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceNode {
    id: MountedNodeId,
    parent: Option<MountedNodeId>,
    authored_id: Option<ElementId>,
    bounds: LogicalRect,
    widget_debug: SurfaceWidgetDebug,
    computed_style: ComputedStyle,
}

#[derive(Clone, Debug, PartialEq)]
struct SurfaceWidgetDebug {
    widget_type_id: WidgetTypeId,
    diagnostics: Vec<WidgetDiagnostic>,
}

impl SurfaceNode {
    #[must_use]
    const fn new(
        id: MountedNodeId,
        parent: Option<MountedNodeId>,
        authored_id: Option<ElementId>,
        bounds: LogicalRect,
        widget_debug: SurfaceWidgetDebug,
        computed_style: ComputedStyle,
    ) -> Self {
        Self {
            id,
            parent,
            authored_id,
            bounds,
            widget_debug,
            computed_style,
        }
    }

    #[must_use]
    pub const fn id(&self) -> &MountedNodeId {
        &self.id
    }

    #[must_use]
    pub const fn parent(&self) -> Option<&MountedNodeId> {
        self.parent.as_ref()
    }

    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }

    #[must_use]
    pub const fn bounds(&self) -> LogicalRect {
        self.bounds
    }

    /// Returns process-local widget identity for diagnostics only.
    #[must_use]
    pub const fn widget_type_id(&self) -> WidgetTypeId {
        self.widget_debug.widget_type_id
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &[WidgetDiagnostic] {
        self.widget_debug.diagnostics.as_slice()
    }

    #[must_use]
    pub const fn computed_style(&self) -> ComputedStyle {
        self.computed_style
    }
}

/// Non-renderer layout/debug snapshot for one UI tree.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFrame {
    size: LogicalSize,
    nodes: Vec<SurfaceNode>,
}

impl SurfaceFrame {
    #[must_use]
    pub(crate) const fn new(size: LogicalSize, nodes: Vec<SurfaceNode>) -> Self {
        Self { size, nodes }
    }

    #[must_use]
    pub const fn size(&self) -> LogicalSize {
        self.size
    }

    #[must_use]
    pub const fn nodes(&self) -> &[SurfaceNode] {
        self.nodes.as_slice()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn node(&self, id: &MountedNodeId) -> Option<&SurfaceNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    #[must_use]
    pub fn root(&self) -> Option<&SurfaceNode> {
        self.nodes.first()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutOverflow {
    width: bool,
    height: bool,
}

impl LayoutOverflow {
    const fn new(width: bool, height: bool) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(&self) -> bool {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> bool {
        self.height
    }

    #[must_use]
    pub const fn any(&self) -> bool {
        self.width || self.height
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceLayoutNode {
    id: MountedNodeId,
    parent: Option<MountedNodeId>,
    authored_id: Option<ElementId>,
    outer_constraints: LayoutConstraints,
    content_constraints: LayoutConstraints,
    desired_content_size: LogicalSize,
    desired_outer_size: LogicalSize,
    constrained_outer_size: LogicalSize,
    overflow: LayoutOverflow,
    diagnostics: Vec<WidgetDiagnostic>,
}

impl SurfaceLayoutNode {
    fn placeholder(id: MountedNodeId) -> Self {
        let zero = LogicalSize::new(LogicalLength::ZERO, LogicalLength::ZERO);
        Self::new(
            id,
            None,
            None,
            [LayoutConstraints::unbounded(); 2],
            [zero; 3],
            LayoutOverflow::default(),
        )
    }

    const fn new(
        id: MountedNodeId,
        parent: Option<MountedNodeId>,
        authored_id: Option<ElementId>,
        constraints: [LayoutConstraints; 2],
        sizes: [LogicalSize; 3],
        overflow: LayoutOverflow,
    ) -> Self {
        Self {
            id,
            parent,
            authored_id,
            outer_constraints: constraints[0],
            content_constraints: constraints[1],
            desired_content_size: sizes[0],
            desired_outer_size: sizes[1],
            constrained_outer_size: sizes[2],
            overflow,
            diagnostics: Vec::new(),
        }
    }

    fn with_diagnostics(mut self, diagnostics: Vec<WidgetDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    #[must_use]
    pub const fn id(&self) -> &MountedNodeId {
        &self.id
    }

    #[must_use]
    pub const fn parent(&self) -> Option<&MountedNodeId> {
        self.parent.as_ref()
    }

    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }

    #[must_use]
    pub const fn outer_constraints(&self) -> LayoutConstraints {
        self.outer_constraints
    }

    #[must_use]
    pub const fn content_constraints(&self) -> LayoutConstraints {
        self.content_constraints
    }

    #[must_use]
    pub const fn desired_content_size(&self) -> LogicalSize {
        self.desired_content_size
    }

    #[must_use]
    pub const fn desired_outer_size(&self) -> LogicalSize {
        self.desired_outer_size
    }

    #[must_use]
    pub const fn constrained_outer_size(&self) -> LogicalSize {
        self.constrained_outer_size
    }

    #[must_use]
    pub const fn overflow(&self) -> LayoutOverflow {
        self.overflow
    }

    #[must_use]
    pub const fn diagnostics(&self) -> &[WidgetDiagnostic] {
        self.diagnostics.as_slice()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceLayoutReport {
    nodes: Vec<SurfaceLayoutNode>,
}

impl SurfaceLayoutReport {
    const fn new(nodes: Vec<SurfaceLayoutNode>) -> Self {
        Self { nodes }
    }

    #[must_use]
    pub const fn nodes(&self) -> &[SurfaceLayoutNode] {
        self.nodes.as_slice()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn node(&self, id: &MountedNodeId) -> Option<&SurfaceLayoutNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    #[must_use]
    pub fn root(&self) -> Option<&SurfaceLayoutNode> {
        self.nodes.first()
    }
}

/// Aligned layout/debug products from one surface preparation.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePublication {
    products: Arc<SurfacePublicationProducts>,
}

#[derive(Clone, Debug, PartialEq)]
struct SurfacePublicationProducts {
    frame: SurfaceFrame,
    style_report: SurfaceStyleReport,
    layout_report: SurfaceLayoutReport,
}

impl SurfacePublication {
    fn new(
        frame: SurfaceFrame,
        style_report: SurfaceStyleReport,
        layout_report: SurfaceLayoutReport,
    ) -> Self {
        Self {
            products: Arc::new(SurfacePublicationProducts {
                frame,
                style_report,
                layout_report,
            }),
        }
    }

    #[must_use]
    pub fn frame(&self) -> &SurfaceFrame {
        &self.products.frame
    }

    #[must_use]
    pub fn style_report(&self) -> &SurfaceStyleReport {
        &self.products.style_report
    }

    #[must_use]
    pub fn layout_report(&self) -> &SurfaceLayoutReport {
        &self.products.layout_report
    }

    #[must_use]
    pub fn into_parts(self) -> (SurfaceFrame, SurfaceStyleReport, SurfaceLayoutReport) {
        let products = match Arc::try_unwrap(self.products) {
            Ok(products) => products,
            Err(shared) => (*shared).clone(),
        };
        (
            products.frame,
            products.style_report,
            products.layout_report,
        )
    }

    #[cfg(test)]
    fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.products, &other.products)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SurfacePlanningError {
    SemanticIntegrity,
}

impl From<SemanticReconcileError> for SurfacePlanningError {
    fn from(_: SemanticReconcileError) -> Self {
        Self::SemanticIntegrity
    }
}

#[derive(Clone, Copy)]
struct SurfaceCapabilityNeeds {
    layout: bool,
    hit_test: bool,
    paint: bool,
    diagnostics: bool,
}

impl SurfaceCapabilityNeeds {
    fn dirty_phases(self) -> DirtyPhases {
        let mut phases = DirtyPhases::default();
        if self.layout {
            phases.insert(DirtyPhases::LAYOUT);
        }
        if self.hit_test {
            phases.insert(DirtyPhases::HIT_TEST);
        }
        if self.paint {
            phases.insert(DirtyPhases::PAINT);
        }
        if self.diagnostics {
            phases.insert(DirtyPhases::DIAGNOSTICS);
        }
        phases
    }
}

fn layout_context_changed(current: &SurfaceCache, next: &cache::SurfaceContextKey) -> bool {
    current.context_key.constraints != next.constraints
        || current.context_key.measurement_identity != next.measurement_identity
        || current.context_key.measurement_revision != next.measurement_revision
}

fn stage_non_structural_cache(cache: Option<&SurfaceCache>) -> SurfaceCache {
    cache.map_or_else(
        || unreachable!("non-structural publication has a cache"),
        SurfaceCache::staged,
    )
}

fn resolve_layout_phase<Action>(
    tree: &crate::mounted::MountedTree<Action>,
    current: &SurfaceCache,
    capability_plan: &SurfaceCapabilityPlan,
    context: &SurfaceBuildContext<'_>,
) -> CachedLayoutFacts {
    let resolved = ResolvedSurfaceTree::for_layout(
        tree,
        &current.topology,
        &current.styles,
        capability_plan,
    );
    let (size, bounds, report) = layout_resolved_surface(
        &resolved,
        context.root_constraints(),
        context.measurement_provider(),
    );
    CachedLayoutFacts {
        size,
        bounds,
        report,
    }
}

pub(crate) fn plan_mounted_surface_cached<'tree, Action>(
    tree: &'tree mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    cache: Option<&SurfaceCache>,
) -> Result<PlannedSurfacePublication<'tree>, SurfacePlanningError> {
    let next_context = context_key(context);
    let pending = tree.pending_phases();
    let tree_dirty = cache.is_none() || pending.contains(DirtyPhases::TREE);
    if tree_dirty {
        return plan_structural_surface(tree, context, next_context);
    }

    let mut current = stage_non_structural_cache(cache);
    let style_dirty = pending.contains(DirtyPhases::STYLE)
        || current
            .context_key
            .style_tokens
            .content_differs(&next_context.style_tokens);
    let mut layout_dirty =
        pending.contains(DirtyPhases::LAYOUT) || layout_context_changed(&current, &next_context);
    let mut hit_dirty = pending.contains(DirtyPhases::HIT_TEST);
    let mut paint_dirty = pending.contains(DirtyPhases::PAINT);
    let semantics_dirty = pending.contains(DirtyPhases::SEMANTICS);
    let diagnostics_dirty = pending.contains(DirtyPhases::DIAGNOSTICS);
    let mut report = SurfacePhaseReport::default();
    let mut completed = DirtyPhases::default();

    if style_dirty {
        let next_styles = resolve_styles(tree, &current.topology, context.style_tokens());
        layout_dirty |= current.styles.padding_changed(&next_styles);
        paint_dirty |= current.styles.paint_changed(&next_styles);
        current.styles = Arc::new(next_styles);
        report.record(SurfacePhase::Style);
        completed.insert(DirtyPhases::STYLE);
    }

    if layout_dirty {
        hit_dirty = true;
        paint_dirty = true;
    }

    let semantic_product_dirty =
        semantics_dirty || layout_dirty || pending.contains(DirtyPhases::FOCUS_VALIDATION);
    let mut capability_plan = tree.plan_surface_publication_capabilities(
        SurfaceCapabilityNeeds {
            layout: layout_dirty,
            hit_test: hit_dirty,
            paint: paint_dirty,
            diagnostics: diagnostics_dirty,
        }
        .dirty_phases(),
    );
    let semantic_capability_plan =
        semantic_product_dirty.then(|| tree.plan_semantic_publication_capabilities());

    if layout_dirty {
        current.layout = Arc::new(resolve_layout_phase(tree, &current, &capability_plan, context));
        report.record(SurfacePhase::Layout);
        completed.insert(DirtyPhases::LAYOUT);
    }

    let paint_contexts = paint_contexts(&current.layout, &current.styles);
    let hit_contexts = hit_contexts(&current.layout);
    tree.plan_surface_publication_contributions(
        &mut capability_plan,
        &paint_contexts,
        &hit_contexts,
    );

    if hit_dirty {
        current.hit_test = resolve_hit_test(&current.topology, &current.layout, &capability_plan);
        report.record(SurfacePhase::HitTesting);
        completed.insert(DirtyPhases::HIT_TEST);
    }
    if paint_dirty {
        current.paint = resolve_paint(&current.topology, &current.layout, &capability_plan);
        report.record(SurfacePhase::Paint);
        completed.insert(DirtyPhases::PAINT);
    }

    let finalized_semantics = semantic_capability_plan
        .map(|plan| tree.finalize_semantic_publication(plan))
        .transpose()?;
    if finalized_semantics.is_some() {
        #[cfg(test)]
        cache::note_semantics_phase_execution();
        report.record(SurfacePhase::Semantics);
        completed.insert(DirtyPhases::SEMANTICS);
    }
    if diagnostics_dirty {
        current.diagnostics = Arc::new(resolve_diagnostics(&current.topology, &capability_plan));
        report.record(SurfacePhase::Diagnostics);
        completed.insert(DirtyPhases::DIAGNOSTICS);
    }

    current.context_key = Arc::new(next_context);
    if report.contains(SurfacePhase::Style)
        || report.contains(SurfacePhase::Layout)
        || report.contains(SurfacePhase::Diagnostics)
    {
        current.publication = compose_publication(&current);
    }
    Ok(PlannedSurfacePublication::new(
        current,
        report,
        completed,
        capability_plan,
        finalized_semantics,
    ))
}

fn plan_structural_surface<'tree, Action>(
    tree: &'tree mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    context_key: cache::SurfaceContextKey,
) -> Result<PlannedSurfacePublication<'tree>, SurfacePlanningError> {
    let mut report = SurfacePhaseReport::default();
    let topology = collect_topology(tree);
    report.record(SurfacePhase::Tree);
    let styles = resolve_styles(tree, &topology, context.style_tokens());
    report.record(SurfacePhase::Style);
    let mut capability_plan = tree.plan_surface_publication_capabilities(DirtyPhases::ALL);
    let semantic_capability_plan = tree.plan_semantic_publication_capabilities();
    let resolved = ResolvedSurfaceTree::for_layout(tree, &topology, &styles, &capability_plan);
    let (size, bounds, layout_report) = layout_resolved_surface(
        &resolved,
        context.root_constraints(),
        context.measurement_provider(),
    );
    let layout = CachedLayoutFacts {
        size,
        bounds,
        report: layout_report,
    };
    report.record(SurfacePhase::Layout);

    let paint_contexts = paint_contexts(&layout, &styles);
    let hit_contexts = hit_contexts(&layout);
    tree.plan_surface_publication_contributions(
        &mut capability_plan,
        &paint_contexts,
        &hit_contexts,
    );
    let hit_test = resolve_hit_test(&topology, &layout, &capability_plan);
    report.record(SurfacePhase::HitTesting);
    let paint = resolve_paint(&topology, &layout, &capability_plan);
    report.record(SurfacePhase::Paint);
    let finalized_semantics = tree.finalize_semantic_publication(semantic_capability_plan)?;
    #[cfg(test)]
    cache::note_semantics_phase_execution();
    report.record(SurfacePhase::Semantics);
    let diagnostics = resolve_diagnostics(&topology, &capability_plan);
    report.record(SurfacePhase::Diagnostics);

    let placeholder = SurfacePublication::new(
        SurfaceFrame::new(
            LogicalSize::new(LogicalLength::ZERO, LogicalLength::ZERO),
            Vec::new(),
        ),
        SurfaceStyleReport::default(),
        SurfaceLayoutReport::default(),
    );
    let mut rebuilt = SurfaceCache {
        context_key: Arc::new(context_key),
        topology: Arc::new(topology),
        styles: Arc::new(styles),
        layout: Arc::new(layout),
        hit_test,
        paint,
        diagnostics: Arc::new(diagnostics),
        publication: placeholder,
    };
    rebuilt.publication = compose_publication(&rebuilt);
    Ok(PlannedSurfacePublication::new(
        rebuilt,
        report,
        DirtyPhases::ALL,
        capability_plan,
        Some(finalized_semantics),
    ))
}

#[cfg(test)]
fn publish_mounted_surface_cached<Action>(
    tree: &mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    cache: &mut Option<SurfaceCache>,
) -> Result<(SurfacePublication, SurfacePhaseReport), SurfacePlanningError> {
    let planned = plan_mounted_surface_cached(tree, context, cache.as_ref())?;
    let commit = planned.commit_store();
    Ok(commit.commit(tree, cache))
}

fn compose_publication(cache: &SurfaceCache) -> SurfacePublication {
    validate_cache_alignment(cache).unwrap_or_else(|error| unreachable!("{error}"));
    let nodes = cache
        .topology
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            SurfaceNode::new(
                node.id.clone(),
                node.parent.clone(),
                node.authored_id.clone(),
                cache.layout.bounds[index],
                SurfaceWidgetDebug {
                    widget_type_id: node.widget_type_id,
                    diagnostics: cache.diagnostics[index].clone(),
                },
                cache.styles.resolutions[index].computed_style(),
            )
        })
        .collect();
    SurfacePublication::new(
        SurfaceFrame::new(cache.layout.size, nodes),
        cache.styles.report.clone(),
        cache.layout.report.clone(),
    )
}

fn validate_cache_alignment(cache: &SurfaceCache) -> Result<(), &'static str> {
    let expected = cache.topology.nodes.len();
    if cache.styles.resolutions.len() != expected
        || cache.styles.report.nodes().len() != expected
        || cache.layout.bounds.len() != expected
        || cache.layout.report.nodes().len() != expected
        || cache.hit_test.membership().len() != expected
        || cache.diagnostics.len() != expected
    {
        return Err("surface cache fact vectors are not topology-aligned");
    }
    for (index, topology) in cache.topology.nodes.iter().enumerate() {
        let style = &cache.styles.report.nodes()[index];
        let layout = &cache.layout.report.nodes()[index];
        if style.id() != &topology.id
            || style.parent() != topology.parent.as_ref()
            || style.authored_id() != topology.authored_id.as_ref()
            || layout.id() != &topology.id
            || layout.parent() != topology.parent.as_ref()
            || layout.authored_id() != topology.authored_id.as_ref()
            || cache.hit_test.membership()[index] != topology.id
        {
            return Err("surface cache node identity is not topology-aligned");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use runenui_core::{
        Color, Element, LogicalLength, SemanticContribution, SemanticContributionContext,
        SemanticNodeContribution, SemanticRole, StyleTokens, View, Widget, WidgetInvalidation,
        WidgetMeasure, children, column, text,
    };

    use super::{
        SurfaceBuildContext, SurfacePhaseReport, SurfacePublication, cache::SurfaceCache,
        cache::phase_function_counts, cache::reset_phase_function_counts,
        plan_mounted_surface_cached, publish_mounted_surface_cached,
    };
    use crate::{LayoutConstraints, mounted::MountedTree, mounted::apply_invalidation};

    #[derive(Debug)]
    struct SemanticLayoutProbe {
        width: Rc<Cell<u16>>,
        semantic_callbacks: Rc<Cell<usize>>,
    }

    impl Widget<()> for SemanticLayoutProbe {
        type State = ();

        fn create_state(&self) -> Self::State {}

        fn measure(&self, (): &Self::State) -> WidgetMeasure {
            WidgetMeasure::Fixed {
                width: LogicalLength::from(self.width.get()),
                height: LogicalLength::from(10_u16),
            }
        }

        fn semantics(
            &self,
            (): &Self::State,
            _: SemanticContributionContext,
        ) -> SemanticContribution {
            self.semantic_callbacks
                .set(self.semantic_callbacks.get() + 1);
            SemanticContribution::single(SemanticNodeContribution::primary(SemanticRole::Button))
        }
    }

    fn publish<Action>(
        tree: &mut MountedTree<Action>,
        context: &SurfaceBuildContext<'_>,
        cache: &mut Option<SurfaceCache>,
    ) -> (SurfacePublication, SurfacePhaseReport) {
        publish_mounted_surface_cached(tree, context, cache)
            .unwrap_or_else(|_| unreachable!("surface test semantic planning remains valid"))
    }

    fn reuse_tree() -> MountedTree<()> {
        let (tree, _) = MountedTree::mount(
            text("reuse")
                .foreground(Color::BLACK)
                .key("root")
                .into_element(),
        );
        tree
    }

    fn staged_cache(cache: Option<&SurfaceCache>) -> SurfaceCache {
        cache
            .unwrap_or_else(|| unreachable!("publication retains phase products"))
            .staged()
    }

    fn assert_retained_reuse(
        before: &SurfaceCache,
        cache: Option<&SurfaceCache>,
        expected: [bool; 7],
    ) {
        let after = cache.unwrap_or_else(|| unreachable!("publication retains phase products"));
        assert_eq!(before.retained_product_reuse(after), expected);
    }

    #[test]
    fn phase_function_counters_track_only_actual_execution_branches() {
        let (mut tree, _) = MountedTree::<()>::mount(text("phase").key("root").into_element());
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        reset_phase_function_counts();

        let (_, initial) = publish(&mut tree, &context, &mut cache);
        assert_eq!(initial.executed().len(), 7);
        assert_eq!(phase_function_counts(), [1, 1, 1, 1, 1, 1, 1]);

        let (_, clean) = publish(&mut tree, &context, &mut cache);
        assert!(clean.executed().is_empty());
        assert_eq!(phase_function_counts(), [1, 1, 1, 1, 1, 1, 1]);

        let root = tree.publication_preorder_ids()[0].clone();
        let node = tree
            .node_mut(&root)
            .unwrap_or_else(|| unreachable!("test root remains live"));
        apply_invalidation(node, WidgetInvalidation::PAINT);
        let (_, paint) = publish(&mut tree, &context, &mut cache);
        assert_eq!(paint.executed(), &[super::SurfacePhase::Paint]);
        assert_eq!(phase_function_counts(), [1, 1, 1, 1, 2, 1, 1]);
    }

    #[test]
    fn clean_and_semantic_publications_reuse_all_retained_products() {
        let mut tree = reuse_tree();
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let root = tree.publication_preorder_ids()[0].clone();

        let retained = staged_cache(cache.as_ref());
        let (_, clean) = publish(&mut tree, &context, &mut cache);
        assert!(clean.executed().is_empty());
        assert_retained_reuse(&retained, cache.as_ref(), [true; 7]);

        let retained = staged_cache(cache.as_ref());
        let node = tree
            .node_mut(&root)
            .unwrap_or_else(|| unreachable!("test root remains live"));
        apply_invalidation(node, WidgetInvalidation::SEMANTICS);
        let (_, semantic) = publish(&mut tree, &context, &mut cache);
        assert_eq!(semantic.executed(), &[super::SurfacePhase::Semantics]);
        assert_retained_reuse(&retained, cache.as_ref(), [true; 7]);
    }

    #[test]
    fn paint_and_diagnostic_publications_replace_only_owned_products() {
        let mut tree = reuse_tree();
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let root = tree.publication_preorder_ids()[0].clone();

        let retained = staged_cache(cache.as_ref());
        let node = tree
            .node_mut(&root)
            .unwrap_or_else(|| unreachable!("test root remains live"));
        apply_invalidation(node, WidgetInvalidation::PAINT);
        let (_, paint) = publish(&mut tree, &context, &mut cache);
        assert_eq!(paint.executed(), &[super::SurfacePhase::Paint]);
        assert_retained_reuse(
            &retained,
            cache.as_ref(),
            [true, true, true, true, false, true, true],
        );

        let retained = staged_cache(cache.as_ref());
        let node = tree
            .node_mut(&root)
            .unwrap_or_else(|| unreachable!("test root remains live"));
        apply_invalidation(node, WidgetInvalidation::DIAGNOSTICS);
        let (_, diagnostics) = publish(&mut tree, &context, &mut cache);
        assert_eq!(diagnostics.executed(), &[super::SurfacePhase::Diagnostics]);
        assert_retained_reuse(
            &retained,
            cache.as_ref(),
            [true, true, true, true, true, false, false],
        );
    }

    #[test]
    fn layout_publication_replaces_layout_hit_paint_and_debug_products() {
        let mut tree = reuse_tree();
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let root = tree.publication_preorder_ids()[0].clone();
        let retained = staged_cache(cache.as_ref());

        let node = tree
            .node_mut(&root)
            .unwrap_or_else(|| unreachable!("test root remains live"));
        apply_invalidation(node, WidgetInvalidation::LAYOUT);
        let (_, layout) = publish(&mut tree, &context, &mut cache);
        assert_eq!(
            layout.executed(),
            &[
                super::SurfacePhase::Layout,
                super::SurfacePhase::HitTesting,
                super::SurfacePhase::Paint,
                super::SurfacePhase::Semantics,
            ]
        );
        assert_retained_reuse(
            &retained,
            cache.as_ref(),
            [true, true, false, false, false, true, false],
        );
    }

    #[test]
    fn style_publication_replaces_style_paint_and_debug_products() {
        let mut tree = reuse_tree();
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let retained = staged_cache(cache.as_ref());

        tree.reconcile(
            text("reuse")
                .foreground(Color::WHITE)
                .key("root")
                .into_element(),
        );
        let (_, style) = publish(&mut tree, &context, &mut cache);
        assert_eq!(
            style.executed(),
            &[super::SurfacePhase::Style, super::SurfacePhase::Paint]
        );
        assert_retained_reuse(
            &retained,
            cache.as_ref(),
            [true, false, true, true, false, true, false],
        );
    }

    #[test]
    fn report_bookkeeping_is_independent_from_phase_execution_counters() {
        reset_phase_function_counts();
        let mut report = super::SurfacePhaseReport::default();
        report.record(super::SurfacePhase::Tree);
        report.record(super::SurfacePhase::Paint);
        assert_eq!(
            report.executed(),
            &[super::SurfacePhase::Tree, super::SurfacePhase::Paint]
        );
        assert_eq!(phase_function_counts(), [0; 7]);
    }

    #[test]
    fn isolated_phase_entry_points_match_truthful_reports() {
        let (mut tree, _) = MountedTree::<()>::mount(
            text("phase")
                .foreground(Color::BLACK)
                .key("root")
                .into_element(),
        );
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let root = tree.publication_preorder_ids()[0].clone();

        let cases = [
            (
                WidgetInvalidation::PAINT,
                vec![super::SurfacePhase::Paint],
                [0, 0, 0, 0, 1, 0, 0],
            ),
            (
                WidgetInvalidation::SEMANTICS,
                vec![super::SurfacePhase::Semantics],
                [0, 0, 0, 0, 0, 1, 0],
            ),
            (
                WidgetInvalidation::DIAGNOSTICS,
                vec![super::SurfacePhase::Diagnostics],
                [0, 0, 0, 0, 0, 0, 1],
            ),
            (
                WidgetInvalidation::LAYOUT,
                vec![
                    super::SurfacePhase::Layout,
                    super::SurfacePhase::HitTesting,
                    super::SurfacePhase::Paint,
                    super::SurfacePhase::Semantics,
                ],
                [0, 0, 1, 1, 1, 1, 0],
            ),
        ];

        for (invalidation, expected_report, expected_counts) in cases {
            reset_phase_function_counts();
            let node = tree
                .node_mut(&root)
                .unwrap_or_else(|| unreachable!("test root remains live"));
            apply_invalidation(node, invalidation);
            let (_, report) = publish(&mut tree, &context, &mut cache);
            assert_eq!(report.executed(), expected_report);
            assert_eq!(phase_function_counts(), expected_counts);
        }

        reset_phase_function_counts();
        tree.reconcile(
            text("phase")
                .foreground(Color::WHITE)
                .key("root")
                .into_element(),
        );
        let (_, style) = publish(&mut tree, &context, &mut cache);
        assert_eq!(
            style.executed(),
            &[super::SurfacePhase::Style, super::SurfacePhase::Paint]
        );
        assert_eq!(phase_function_counts(), [0, 1, 0, 0, 1, 0, 0]);

        tree.reconcile(
            text("phase")
                .foreground(Color::WHITE)
                .key("root")
                .into_element(),
        );
        reset_phase_function_counts();
        let (_, clean) = publish(&mut tree, &context, &mut cache);
        assert!(clean.executed().is_empty());
        assert_eq!(phase_function_counts(), [0; 7]);
    }

    #[test]
    fn layout_recomposes_semantic_bounds_without_semantic_callback_reentry() {
        let width = Rc::new(Cell::new(10_u16));
        let semantic_callbacks = Rc::new(Cell::new(0));
        let (mut tree, _) = MountedTree::<()>::mount(
            Element::new(SemanticLayoutProbe {
                width: Rc::clone(&width),
                semantic_callbacks: Rc::clone(&semantic_callbacks),
            })
            .key("root"),
        );
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;

        let planned = plan_mounted_surface_cached(&mut tree, &context, cache.as_ref())
            .unwrap_or_else(|_| unreachable!("initial semantic layout plan is valid"));
        let (first, _) = planned
            .semantic_candidate(None)
            .unwrap_or_else(|_| unreachable!("initial semantic candidate is aligned"))
            .unwrap_or_else(|| unreachable!("initial structural plan includes semantics"));
        assert_eq!(semantic_callbacks.get(), 1);
        assert_eq!(first.nodes.len(), 1);
        assert!((first.nodes[0].bounds.width() - 10.0).abs() <= f32::EPSILON);
        let semantic_id = first.nodes[0].id.clone();
        let commit = planned.commit_store();
        let (_, initial_report) = commit.commit(&mut tree, &mut cache);
        assert!(
            initial_report
                .executed()
                .contains(&super::SurfacePhase::Semantics)
        );

        width.set(20);
        let root = tree.publication_preorder_ids()[0].clone();
        let node = tree
            .node_mut(&root)
            .unwrap_or_else(|| unreachable!("semantic layout probe remains mounted"));
        apply_invalidation(node, WidgetInvalidation::LAYOUT);

        let planned = plan_mounted_surface_cached(&mut tree, &context, cache.as_ref())
            .unwrap_or_else(|_| unreachable!("layout semantic plan is valid"));
        let (second, _) = planned
            .semantic_candidate(None)
            .unwrap_or_else(|_| unreachable!("layout semantic candidate is aligned"))
            .unwrap_or_else(|| unreachable!("layout dirtiness recomposes semantics"));
        assert_eq!(semantic_callbacks.get(), 1);
        assert_eq!(second.nodes.len(), 1);
        assert_eq!(second.nodes[0].id, semantic_id);
        assert!((second.nodes[0].bounds.width() - 20.0).abs() <= f32::EPSILON);
        let commit = planned.commit_store();
        let (_, report) = commit.commit(&mut tree, &mut cache);
        assert_eq!(
            report.executed(),
            &[
                super::SurfacePhase::Layout,
                super::SurfacePhase::HitTesting,
                super::SurfacePhase::Paint,
                super::SurfacePhase::Semantics,
            ]
        );
        assert_eq!(semantic_callbacks.get(), 1);
    }

    #[test]
    fn structural_rebuild_enters_every_conservative_phase() {
        let (mut tree, _) = MountedTree::<()>::mount(text("old").key("root").into_element());
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        tree.reconcile(
            column(children![text("new").key("child")])
                .key("root")
                .into_element(),
        );
        reset_phase_function_counts();

        let (_, report) = publish(&mut tree, &context, &mut cache);
        assert_eq!(
            report.executed(),
            &[
                super::SurfacePhase::Tree,
                super::SurfacePhase::Style,
                super::SurfacePhase::Layout,
                super::SurfacePhase::HitTesting,
                super::SurfacePhase::Paint,
                super::SurfacePhase::Semantics,
                super::SurfacePhase::Diagnostics,
            ]
        );
        assert_eq!(phase_function_counts(), [1; 7]);
    }
}
