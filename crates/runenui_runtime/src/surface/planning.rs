use std::sync::Arc;

use runenui_core::{LogicalLength, LogicalSize, WidgetDiagnostic};

use crate::mounted::{DirtyPhases, SemanticReconcileError, SurfaceCapabilityPlan};
use crate::style_debug::SurfaceStyleReport;

use super::cache::{CachedLayoutFacts, context_key};
use super::measure::layout_resolved_surface;
use super::resolve::{
    ResolvedSurfaceTree, collect_topology, hit_contexts, paint_contexts, resolve_diagnostics,
    resolve_hit_test, resolve_paint, resolve_styles,
};
use super::transaction::PlannedSurfacePublication;
use super::{
    SurfaceBuildContext, SurfaceCache, SurfaceFrame, SurfaceLayoutReport, SurfacePhase,
    SurfacePhaseReport, SurfacePublication, SurfaceWidgetDebug,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SurfacePlanningError {
    SemanticIntegrity,
}

impl From<SemanticReconcileError> for SurfacePlanningError {
    fn from(_: SemanticReconcileError) -> Self {
        Self::SemanticIntegrity
    }
}

fn surface_capability_phases(entries: [(bool, DirtyPhases); 5]) -> DirtyPhases {
    let mut phases = DirtyPhases::default();
    for (is_dirty, phase) in entries {
        if is_dirty {
            phases.insert(phase);
        }
    }
    phases
}

fn layout_context_changed(current: &SurfaceCache, next: &super::cache::SurfaceContextKey) -> bool {
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

fn replace_scene_diagnostics_if_changed(
    current: &mut Arc<Vec<Vec<WidgetDiagnostic>>>,
    next: Vec<Vec<WidgetDiagnostic>>,
) -> bool {
    if current.as_ref() == &next {
        false
    } else {
        *current = Arc::new(next);
        true
    }
}

fn resolve_contribution_phases<Action>(
    tree: &crate::mounted::MountedTree<Action>,
    current: &mut SurfaceCache,
    capability_plan: &mut SurfaceCapabilityPlan,
    hit_dirty: bool,
    paint_dirty: bool,
    report: &mut SurfacePhaseReport,
    completed: &mut DirtyPhases,
) -> bool {
    let paint_contexts = paint_contexts(&current.layout, &current.styles);
    let hit_contexts = hit_contexts(&current.layout);
    tree.plan_surface_publication_contributions(capability_plan, &paint_contexts, &hit_contexts);

    let mut scene_diagnostics_changed = false;
    if hit_dirty {
        let resolved = resolve_hit_test(&current.topology, &current.layout, capability_plan);
        current.hit_test = resolved.scene;
        scene_diagnostics_changed |= replace_scene_diagnostics_if_changed(
            &mut current.hit_diagnostics,
            resolved.diagnostics,
        );
        report.record(SurfacePhase::HitTesting);
        completed.insert(DirtyPhases::HIT_TEST);
    }
    if paint_dirty {
        let resolved = resolve_paint(&current.topology, &current.layout, capability_plan);
        current.paint = resolved.scene;
        scene_diagnostics_changed |= replace_scene_diagnostics_if_changed(
            &mut current.paint_diagnostics,
            resolved.diagnostics,
        );
        report.record(SurfacePhase::Paint);
        completed.insert(DirtyPhases::PAINT);
    }
    scene_diagnostics_changed
}

fn resolve_layout_phase<Action>(
    tree: &crate::mounted::MountedTree<Action>,
    current: &SurfaceCache,
    capability_plan: &SurfaceCapabilityPlan,
    context: &SurfaceBuildContext<'_>,
) -> CachedLayoutFacts {
    let resolved =
        ResolvedSurfaceTree::for_layout(tree, &current.topology, &current.styles, capability_plan);
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
            .style_environment
            .content_differs(&next_context.style_environment);
    let mut layout_dirty =
        pending.contains(DirtyPhases::LAYOUT) || layout_context_changed(&current, &next_context);
    let mut hit_dirty = pending.contains(DirtyPhases::HIT_TEST);
    let mut paint_dirty = pending.contains(DirtyPhases::PAINT);
    let semantics_dirty = pending.contains(DirtyPhases::SEMANTICS);
    let diagnostics_dirty = pending.contains(DirtyPhases::DIAGNOSTICS);
    let mut report = SurfacePhaseReport::default();
    let mut completed = DirtyPhases::default();
    let mut initial_capability_phases = DirtyPhases::default();
    if style_dirty {
        initial_capability_phases.insert(DirtyPhases::STYLE);
    }
    let mut capability_plan = tree.plan_surface_publication_capabilities(initial_capability_phases);

    if style_dirty {
        let next_styles = resolve_styles(
            tree,
            &current.topology,
            context.style_environment(),
            &capability_plan,
        );
        let effects = current.styles.effects_against(&next_styles);
        layout_dirty |= effects.layout();
        paint_dirty |= effects.paint();
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
    tree.extend_surface_publication_capabilities(
        &mut capability_plan,
        surface_capability_phases([
            (layout_dirty, DirtyPhases::LAYOUT),
            (hit_dirty, DirtyPhases::HIT_TEST),
            (paint_dirty, DirtyPhases::PAINT),
            (semantic_product_dirty, DirtyPhases::SEMANTICS),
            (diagnostics_dirty, DirtyPhases::DIAGNOSTICS),
        ]),
    );
    let semantic_capability_plan = semantic_product_dirty
        .then(|| tree.plan_semantic_publication_capabilities(&capability_plan));

    if layout_dirty {
        current.layout = Arc::new(resolve_layout_phase(
            tree,
            &current,
            &capability_plan,
            context,
        ));
        report.record(SurfacePhase::Layout);
        completed.insert(DirtyPhases::LAYOUT);
    }

    let scene_diagnostics_changed = resolve_contribution_phases(
        tree,
        &mut current,
        &mut capability_plan,
        hit_dirty,
        paint_dirty,
        &mut report,
        &mut completed,
    );

    let finalized_semantics = semantic_capability_plan
        .map(|plan| tree.finalize_semantic_publication(plan))
        .transpose()?;
    if finalized_semantics.is_some() {
        #[cfg(test)]
        super::cache::note_semantics_phase_execution();
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
        || scene_diagnostics_changed
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
    context_key: super::cache::SurfaceContextKey,
) -> Result<PlannedSurfacePublication<'tree>, SurfacePlanningError> {
    let mut report = SurfacePhaseReport::default();
    let topology = collect_topology(tree);
    report.record(SurfacePhase::Tree);
    let mut capability_plan = tree.plan_surface_publication_capabilities(DirtyPhases::STYLE);
    let styles = resolve_styles(
        tree,
        &topology,
        context.style_environment(),
        &capability_plan,
    );
    report.record(SurfacePhase::Style);
    tree.extend_surface_publication_capabilities(&mut capability_plan, DirtyPhases::ALL);
    let semantic_capability_plan = tree.plan_semantic_publication_capabilities(&capability_plan);
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
    let resolved_hit_test = resolve_hit_test(&topology, &layout, &capability_plan);
    let hit_test = resolved_hit_test.scene;
    let hit_diagnostics = Arc::new(resolved_hit_test.diagnostics);
    report.record(SurfacePhase::HitTesting);
    let resolved_paint = resolve_paint(&topology, &layout, &capability_plan);
    let paint = resolved_paint.scene;
    let paint_diagnostics = Arc::new(resolved_paint.diagnostics);
    report.record(SurfacePhase::Paint);
    let finalized_semantics = tree.finalize_semantic_publication(semantic_capability_plan)?;
    #[cfg(test)]
    super::cache::note_semantics_phase_execution();
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
        hit_diagnostics,
        paint_diagnostics,
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
pub(super) fn publish_mounted_surface_cached<Action>(
    tree: &mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    cache: &mut Option<SurfaceCache>,
) -> Result<(SurfacePublication, SurfacePhaseReport), SurfacePlanningError> {
    let planned = plan_mounted_surface_cached(tree, context, cache.as_ref())?;
    let commit = planned.commit_store();
    Ok(commit.commit(tree, cache))
}

fn combined_node_diagnostics(cache: &SurfaceCache, index: usize) -> Vec<WidgetDiagnostic> {
    let mut diagnostics = Vec::with_capacity(
        cache.diagnostics[index].len()
            + cache.hit_diagnostics[index].len()
            + cache.paint_diagnostics[index].len(),
    );
    diagnostics.extend(cache.diagnostics[index].iter().cloned());
    diagnostics.extend(cache.hit_diagnostics[index].iter().cloned());
    diagnostics.extend(cache.paint_diagnostics[index].iter().cloned());
    diagnostics
}

fn compose_publication(cache: &SurfaceCache) -> SurfacePublication {
    validate_cache_alignment(cache).unwrap_or_else(|error| unreachable!("{error}"));
    let nodes = cache
        .topology
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            super::SurfaceNode::new(
                node.id.clone(),
                node.parent.clone(),
                node.authored_id.clone(),
                cache.layout.bounds[index],
                SurfaceWidgetDebug {
                    widget_type_id: node.widget_type_id,
                    diagnostics: combined_node_diagnostics(cache, index),
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
        || cache.hit_diagnostics.len() != expected
        || cache.paint_diagnostics.len() != expected
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
