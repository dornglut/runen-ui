//! Renderer-facing surface-frame data model.
#![allow(clippy::redundant_pub_crate)]
//!
//! Surface frames are host-neutral snapshots that later renderer stages can
//! consume. This module owns explicit surface build inputs, a small row/column
//! layout pass, concrete computed-style delivery, and bounds hit testing.

mod arrange;
mod cache;
mod context;
mod measure;
mod resolve;

pub(crate) use cache::SurfaceCache;
use cache::{CachedLayoutFacts, build_hit_test_facts, context_key};
pub use cache::{SurfacePhase, SurfacePhaseReport};
pub use context::SurfaceBuildContext;
use measure::layout_resolved_surface;
use resolve::{
    ResolvedSurfaceTree, collect_topology, resolve_diagnostics, resolve_paint, resolve_semantics,
    resolve_styles,
};

use runenui_core::{
    ComputedStyle, ElementId, LogicalLength, LogicalRect, LogicalSize, SemanticContribution,
    WidgetDiagnostic, WidgetPaintProof, WidgetTypeId,
};

use crate::mounted::{DirtyPhases, SemanticReconcileError};
use crate::style_debug::SurfaceStyleReport;
use crate::{LayoutConstraints, LogicalPoint, MountedNodeId};

/// One ordered node in a renderer-facing surface frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceNode {
    id: MountedNodeId,
    parent: Option<MountedNodeId>,
    authored_id: Option<ElementId>,
    bounds: LogicalRect,
    widget_proof: SurfaceWidgetProof,
    computed_style: ComputedStyle,
}

#[derive(Clone, Debug, PartialEq)]
struct SurfaceWidgetProof {
    widget_type_id: WidgetTypeId,
    paint: WidgetPaintProof,
    semantics: SemanticContribution,
    diagnostics: Vec<WidgetDiagnostic>,
}

impl SurfaceNode {
    /// Creates a surface node.
    #[must_use]
    const fn new(
        id: MountedNodeId,
        parent: Option<MountedNodeId>,
        authored_id: Option<ElementId>,
        bounds: LogicalRect,
        widget_proof: SurfaceWidgetProof,
        computed_style: ComputedStyle,
    ) -> Self {
        Self {
            id,
            parent,
            authored_id,
            bounds,
            widget_proof,
            computed_style,
        }
    }

    /// Returns the generated runtime node ID.
    #[must_use]
    pub const fn id(&self) -> &MountedNodeId {
        &self.id
    }

    /// Returns the generated runtime parent ID, if present.
    #[must_use]
    pub const fn parent(&self) -> Option<&MountedNodeId> {
        self.parent.as_ref()
    }

    /// Returns the optional authored element ID.
    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }

    /// Returns the resolved logical outer bounds.
    #[must_use]
    pub const fn bounds(&self) -> LogicalRect {
        self.bounds
    }

    /// Returns the process-local concrete widget implementation identity.
    #[must_use]
    pub const fn widget_type_id(&self) -> WidgetTypeId {
        self.widget_proof.widget_type_id
    }

    /// Returns proof-level renderer-neutral paint/debug facts.
    #[must_use]
    pub const fn paint(&self) -> &WidgetPaintProof {
        &self.widget_proof.paint
    }

    /// Returns the temporary M5A canonical semantic contribution.
    ///
    /// M5B removes semantic authority from `SurfaceFrame` and publishes the
    /// independent renderer-neutral semantic product instead.
    #[must_use]
    pub const fn semantics(&self) -> &SemanticContribution {
        &self.widget_proof.semantics
    }

    /// Returns deterministic widget diagnostics.
    #[must_use]
    pub const fn diagnostics(&self) -> &[WidgetDiagnostic] {
        self.widget_proof.diagnostics.as_slice()
    }

    /// Returns the concrete resolved style consumed by layout and renderers.
    #[must_use]
    pub const fn computed_style(&self) -> ComputedStyle {
        self.computed_style
    }
}

/// Renderer-facing surface snapshot for one UI tree.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFrame {
    size: LogicalSize,
    nodes: Vec<SurfaceNode>,
}

impl SurfaceFrame {
    /// Creates a surface frame from a frame size and ordered nodes.
    #[must_use]
    pub(crate) const fn new(size: LogicalSize, nodes: Vec<SurfaceNode>) -> Self {
        Self { size, nodes }
    }

    /// Returns the logical frame size.
    #[must_use]
    pub const fn size(&self) -> LogicalSize {
        self.size
    }

    /// Returns the ordered surface nodes.
    #[must_use]
    pub const fn nodes(&self) -> &[SurfaceNode] {
        self.nodes.as_slice()
    }

    /// Returns whether this frame contains no surface nodes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the surface node for the provided runtime node ID.
    #[must_use]
    pub fn node(&self, id: &MountedNodeId) -> Option<&SurfaceNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    /// Returns the root surface node, when present.
    #[must_use]
    pub fn root(&self) -> Option<&SurfaceNode> {
        self.nodes.first()
    }

    /// Returns the topmost surface node containing the provided point.
    ///
    /// Nodes are checked in reverse surface order so later/deeper nodes win over
    /// parent containers whose bounds also contain the point.
    #[must_use]
    pub fn hit_test(&self, point: LogicalPoint) -> Option<&SurfaceNode> {
        self.nodes
            .iter()
            .rev()
            .find(|node| node.bounds().contains(point))
    }

    /// Returns the runtime node ID for the topmost node containing the point.
    #[must_use]
    pub fn hit_test_id(&self, point: LogicalPoint) -> Option<MountedNodeId> {
        self.hit_test(point).map(|node| node.id().clone())
    }
}

/// Per-axis overflow pressure recorded during surface layout.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutOverflow {
    width: bool,
    height: bool,
}

impl LayoutOverflow {
    const fn new(width: bool, height: bool) -> Self {
        Self { width, height }
    }

    /// Returns whether horizontal layout pressure exceeded a finite maximum.
    #[must_use]
    pub const fn width(&self) -> bool {
        self.width
    }

    /// Returns whether vertical layout pressure exceeded a finite maximum.
    #[must_use]
    pub const fn height(&self) -> bool {
        self.height
    }

    /// Returns whether either axis overflowed.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.width || self.height
    }
}

/// One runtime-node-aligned diagnostic result from surface measurement.
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

    /// Returns the generated runtime node ID.
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

    /// Returns the outer constraints supplied to this node.
    #[must_use]
    pub const fn outer_constraints(&self) -> LayoutConstraints {
        self.outer_constraints
    }

    /// Returns this node's padding-adjusted content-box constraints.
    #[must_use]
    pub const fn content_constraints(&self) -> LayoutConstraints {
        self.content_constraints
    }

    /// Returns the sanitized content size desired before content constraints.
    #[must_use]
    pub const fn desired_content_size(&self) -> LogicalSize {
        self.desired_content_size
    }

    /// Returns the desired outer size after content constraints and box policy.
    #[must_use]
    pub const fn desired_outer_size(&self) -> LogicalSize {
        self.desired_outer_size
    }

    /// Returns the final outer size used by arrangement.
    #[must_use]
    pub const fn constrained_outer_size(&self) -> LogicalSize {
        self.constrained_outer_size
    }

    /// Returns deterministic overflow pressure for this node.
    #[must_use]
    pub const fn overflow(&self) -> LayoutOverflow {
        self.overflow
    }

    /// Returns ordered proof-level layout capability diagnostics.
    #[must_use]
    pub const fn diagnostics(&self) -> &[WidgetDiagnostic] {
        self.diagnostics.as_slice()
    }
}

/// Runtime-node-aligned layout diagnostics from one surface publication.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceLayoutReport {
    nodes: Vec<SurfaceLayoutNode>,
}

impl SurfaceLayoutReport {
    const fn new(nodes: Vec<SurfaceLayoutNode>) -> Self {
        Self { nodes }
    }

    /// Returns measured nodes in the same order as the surface frame.
    #[must_use]
    pub const fn nodes(&self) -> &[SurfaceLayoutNode] {
        self.nodes.as_slice()
    }

    /// Returns whether this report contains no layout nodes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the layout node for the provided runtime node ID.
    #[must_use]
    pub fn node(&self, id: &MountedNodeId) -> Option<&SurfaceLayoutNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    /// Returns the root layout node, when present.
    #[must_use]
    pub fn root(&self) -> Option<&SurfaceLayoutNode> {
        self.nodes.first()
    }
}

/// Aligned renderer-facing and diagnostic products from one surface preparation.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePublication {
    frame: SurfaceFrame,
    style_report: SurfaceStyleReport,
    layout_report: SurfaceLayoutReport,
}

impl SurfacePublication {
    const fn new(
        frame: SurfaceFrame,
        style_report: SurfaceStyleReport,
        layout_report: SurfaceLayoutReport,
    ) -> Self {
        Self {
            frame,
            style_report,
            layout_report,
        }
    }

    /// Returns the renderer-facing surface frame.
    #[must_use]
    pub const fn frame(&self) -> &SurfaceFrame {
        &self.frame
    }

    /// Returns style provenance and diagnostics aligned to the frame nodes.
    #[must_use]
    pub const fn style_report(&self) -> &SurfaceStyleReport {
        &self.style_report
    }

    /// Returns layout diagnostics aligned to the frame nodes.
    #[must_use]
    pub const fn layout_report(&self) -> &SurfaceLayoutReport {
        &self.layout_report
    }

    /// Consumes the publication and returns its aligned products.
    #[must_use]
    pub fn into_parts(self) -> (SurfaceFrame, SurfaceStyleReport, SurfaceLayoutReport) {
        (self.frame, self.style_report, self.layout_report)
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

fn surface_capability_phases(
    layout_dirty: bool,
    paint_dirty: bool,
    diagnostics_dirty: bool,
) -> DirtyPhases {
    let mut phases = DirtyPhases::default();
    if layout_dirty {
        phases.insert(DirtyPhases::LAYOUT);
    }
    if paint_dirty {
        phases.insert(DirtyPhases::PAINT);
    }
    if diagnostics_dirty {
        phases.insert(DirtyPhases::DIAGNOSTICS);
    }
    phases
}

fn layout_context_changed(current: &SurfaceCache, next: &cache::SurfaceContextKey) -> bool {
    current.context_key.constraints != next.constraints
        || current.context_key.measurement_identity != next.measurement_identity
        || current.context_key.measurement_revision != next.measurement_revision
}

/// Resolves style once per node and publishes aligned frame and diagnostic products.
///
/// The row/column layout consumes concrete computed padding for intrinsic outer
/// sizing, container content origins, root child placement, and hit testing.
pub(crate) fn publish_mounted_surface_cached<Action>(
    tree: &mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    cache: &mut Option<SurfaceCache>,
) -> Result<(SurfacePublication, SurfacePhaseReport), SurfacePlanningError> {
    let next_context = context_key(context);
    let pending = tree.pending_phases();
    let tree_dirty = cache.is_none() || pending.contains(DirtyPhases::TREE);
    if tree_dirty {
        return rebuild_structural_surface(tree, context, next_context, cache);
    }

    let mut current = cache
        .clone()
        .unwrap_or_else(|| unreachable!("non-structural publication has a cache"));
    let style_dirty = pending.contains(DirtyPhases::STYLE)
        || current
            .context_key
            .style_tokens
            .content_differs(&next_context.style_tokens);
    let mut layout_dirty =
        pending.contains(DirtyPhases::LAYOUT) || layout_context_changed(&current, &next_context);
    let mut paint_dirty = pending.contains(DirtyPhases::PAINT);
    let semantics_dirty = pending.contains(DirtyPhases::SEMANTICS);
    let diagnostics_dirty = pending.contains(DirtyPhases::DIAGNOSTICS);
    let mut report = SurfacePhaseReport::default();
    let mut completed = DirtyPhases::default();

    if style_dirty {
        let next_styles = resolve_styles(tree, &current.topology, context.style_tokens());
        layout_dirty |= current.styles.padding_changed(&next_styles);
        paint_dirty |= current.styles.paint_changed(&next_styles);
        current.styles = next_styles;
        report.record(SurfacePhase::Style);
        completed.insert(DirtyPhases::STYLE);
    }

    let capability_plan = tree.plan_surface_publication_capabilities(surface_capability_phases(
        layout_dirty,
        paint_dirty,
        diagnostics_dirty,
    ));
    let semantic_capability_plan =
        semantics_dirty.then(|| tree.plan_semantic_publication_capabilities());

    if layout_dirty {
        let resolved = ResolvedSurfaceTree::for_layout(
            tree,
            &current.topology,
            &current.styles,
            &capability_plan,
        );
        let (size, bounds, layout_report) = layout_resolved_surface(
            &resolved,
            context.root_constraints(),
            context.measurement_provider(),
        );
        current.layout = CachedLayoutFacts {
            size,
            bounds,
            report: layout_report,
        };
        report.record(SurfacePhase::Layout);
        completed.insert(DirtyPhases::LAYOUT);

        current.hit_test = build_hit_test_facts(&current.layout);
        report.record(SurfacePhase::HitTesting);
        completed.insert(DirtyPhases::HIT_TEST);
    } else if pending.contains(DirtyPhases::HIT_TEST) {
        current.hit_test = build_hit_test_facts(&current.layout);
        report.record(SurfacePhase::HitTesting);
        completed.insert(DirtyPhases::HIT_TEST);
    }
    if paint_dirty {
        current.paint = resolve_paint(&current.topology, &capability_plan);
        report.record(SurfacePhase::Paint);
        completed.insert(DirtyPhases::PAINT);
    }

    let finalized_semantics = semantic_capability_plan
        .map(|plan| tree.finalize_semantic_publication(plan))
        .transpose()?;
    if let Some(finalized) = finalized_semantics.as_ref() {
        current.semantics = resolve_semantics(finalized);
        report.record(SurfacePhase::Semantics);
        completed.insert(DirtyPhases::SEMANTICS);
    }
    if diagnostics_dirty {
        current.diagnostics = resolve_diagnostics(&current.topology, &capability_plan);
        report.record(SurfacePhase::Diagnostics);
        completed.insert(DirtyPhases::DIAGNOSTICS);
    }

    if report.executed().is_empty() {
        let publication = current.publication.clone();
        current.context_key = next_context;
        *cache = Some(current);
        return Ok((publication, report));
    }
    current.context_key = next_context;
    current.publication = compose_publication(&current);
    if let Some(finalized) = finalized_semantics {
        let semantic_commit = finalized.commit_store();
        tree.commit_semantic_publication(semantic_commit);
    }
    tree.commit_surface_publication_capabilities(capability_plan);
    tree.finish_publication(completed);
    let publication = current.publication.clone();
    *cache = Some(current);
    Ok((publication, report))
}

fn rebuild_structural_surface<Action>(
    tree: &mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    context_key: cache::SurfaceContextKey,
    cache: &mut Option<SurfaceCache>,
) -> Result<(SurfacePublication, SurfacePhaseReport), SurfacePlanningError> {
    let mut report = SurfacePhaseReport::default();
    let topology = collect_topology(tree);
    report.record(SurfacePhase::Tree);
    let styles = resolve_styles(tree, &topology, context.style_tokens());
    report.record(SurfacePhase::Style);
    let capability_plan = tree.plan_surface_publication_capabilities(DirtyPhases::ALL);
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
    let hit_test = build_hit_test_facts(&layout);
    report.record(SurfacePhase::HitTesting);
    let paint = resolve_paint(&topology, &capability_plan);
    report.record(SurfacePhase::Paint);
    let finalized_semantics = tree.finalize_semantic_publication(semantic_capability_plan)?;
    let semantics = resolve_semantics(&finalized_semantics);
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
        context_key,
        topology,
        styles,
        layout,
        hit_test,
        paint,
        semantics,
        diagnostics,
        publication: placeholder,
    };
    rebuilt.publication = compose_publication(&rebuilt);
    let semantic_commit = finalized_semantics.commit_store();
    tree.commit_semantic_publication(semantic_commit);
    tree.commit_surface_publication_capabilities(capability_plan);
    tree.finish_publication(DirtyPhases::ALL);
    let publication = rebuilt.publication.clone();
    *cache = Some(rebuilt);
    Ok((publication, report))
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
                cache.hit_test.bounds[index],
                SurfaceWidgetProof {
                    widget_type_id: node.widget_type_id,
                    paint: cache.paint[index].clone(),
                    semantics: cache.semantics[index].clone(),
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
        || cache.hit_test.bounds.len() != expected
        || cache.paint.len() != expected
        || cache.semantics.len() != expected
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
        {
            return Err("surface cache node identity is not topology-aligned");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use runenui_core::{Color, StyleTokens, View, WidgetInvalidation, children, column, text};

    use super::{
        SurfaceBuildContext, SurfacePhaseReport, SurfacePublication, cache::SurfaceCache,
        cache::phase_function_counts, cache::reset_phase_function_counts,
        publish_mounted_surface_cached,
    };
    use crate::{LayoutConstraints, mounted::MountedTree, mounted::apply_invalidation};

    fn publish<Action>(
        tree: &mut MountedTree<Action>,
        context: &SurfaceBuildContext<'_>,
        cache: &mut Option<SurfaceCache>,
    ) -> (SurfacePublication, SurfacePhaseReport) {
        publish_mounted_surface_cached(tree, context, cache)
            .unwrap_or_else(|_| unreachable!("surface test semantic planning remains valid"))
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
                vec![super::SurfacePhase::Layout, super::SurfacePhase::HitTesting],
                [0, 0, 1, 1, 0, 0, 0],
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
