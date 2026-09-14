use std::sync::Arc;

#[cfg(test)]
use runenui_core::{FontFamilyName, GenericFontFamily};
use runenui_core::{LogicalLength, LogicalSize, MonotonicInstant, WidgetDiagnostic};
#[cfg(test)]
use runenui_text::FontSourcePolicy;
use runenui_text::{TextLayoutError, TextSystem};

use crate::mounted::{DirtyPhases, SemanticReconcileError, SurfaceCapabilityPlan};
use crate::style_debug::SurfaceStyleReport;

use super::cache::{CachedLayoutFacts, context_key};
use super::motion::{self, MotionPlanningError};
use super::resolve::{
    EffectiveEffects, PresentationGeometryError, ResolvedSurfaceTree, collect_topology,
    hit_contexts, paint_contexts, resolve_diagnostics, resolve_hit_test, resolve_paint,
    resolve_presentation, resolve_styles,
};
use super::taffy_layout::layout_resolved_surface;
use super::transaction::PlannedSurfacePublication;
use super::{
    SurfaceBuildContext, SurfaceCache, SurfaceFrame, SurfaceInteractionProjection,
    SurfaceLayoutReport, SurfaceMotionActivity, SurfaceMotionStore, SurfacePhase,
    SurfacePhaseReport, SurfacePublication, SurfaceWidgetDebug,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SurfacePlanningError {
    SemanticIntegrity,
    TextLayout(TextLayoutError),
    PresentationGeometry,
    Motion,
}

impl From<SemanticReconcileError> for SurfacePlanningError {
    fn from(_: SemanticReconcileError) -> Self {
        Self::SemanticIntegrity
    }
}

impl From<TextLayoutError> for SurfacePlanningError {
    fn from(error: TextLayoutError) -> Self {
        Self::TextLayout(error)
    }
}

impl From<PresentationGeometryError> for SurfacePlanningError {
    fn from(_: PresentationGeometryError) -> Self {
        Self::PresentationGeometry
    }
}

impl From<MotionPlanningError> for SurfacePlanningError {
    fn from(_: MotionPlanningError) -> Self {
        Self::Motion
    }
}

fn surface_capability_phases(entries: [(bool, DirtyPhases); 4]) -> DirtyPhases {
    let mut phases = DirtyPhases::default();
    for (is_dirty, phase) in entries {
        if is_dirty {
            phases.insert(phase);
        }
    }
    phases
}

fn initial_surface_capability_plan<Action>(
    tree: &crate::mounted::MountedTree<Action>,
    style_dirty: bool,
) -> SurfaceCapabilityPlan {
    let mut phases = DirtyPhases::default();
    if style_dirty {
        phases.insert(DirtyPhases::STYLE);
    }
    tree.plan_surface_publication_capabilities(phases)
}

const fn complete_non_structural_publication_phases(
    pending: DirtyPhases,
    presentation_dirty: bool,
    mut phases: DirtyPhases,
) -> (DirtyPhases, bool) {
    if presentation_dirty {
        phases.insert(DirtyPhases::HIT_TEST);
        phases.insert(DirtyPhases::PAINT);
    }
    let semantic_dirty = pending.contains(DirtyPhases::SEMANTICS)
        || presentation_dirty
        || pending.contains(DirtyPhases::FOCUS_VALIDATION);
    if semantic_dirty {
        phases.insert(DirtyPhases::SEMANTICS);
    }
    (phases, semantic_dirty)
}

fn style_product_is_dirty(
    pending: DirtyPhases,
    current: &SurfaceCache,
    next: &super::cache::SurfaceContextKey,
    interaction: &SurfaceInteractionProjection,
) -> bool {
    pending.contains(DirtyPhases::STYLE)
        || current
            .context_key
            .style_environment
            .content_differs(&next.style_environment)
        || current.interaction.content_differs(interaction)
}

fn layout_context_changed(current: &SurfaceCache, next: &super::cache::SurfaceContextKey) -> bool {
    current.context_key.constraints != next.constraints
        || current.context_key.font_source != next.font_source
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
    text_system: &mut TextSystem,
    publication_phases: DirtyPhases,
    report: &mut SurfacePhaseReport,
    completed: &mut DirtyPhases,
) -> bool {
    let paint_contexts = paint_contexts(&current.layout, &current.effective);
    let hit_contexts = hit_contexts(&current.layout);
    tree.plan_surface_publication_contributions(capability_plan, &paint_contexts, &hit_contexts);

    let mut scene_diagnostics_changed = false;
    if publication_phases.contains(DirtyPhases::HIT_TEST) {
        let resolved = resolve_hit_test(&current.topology, &current.presentation, capability_plan);
        current.hit_test = resolved.scene;
        scene_diagnostics_changed |= replace_scene_diagnostics_if_changed(
            &mut current.hit_diagnostics,
            resolved.diagnostics,
        );
        report.record(SurfacePhase::HitTesting);
        completed.insert(DirtyPhases::HIT_TEST);
    }
    if publication_phases.contains(DirtyPhases::PAINT) {
        let resolved = resolve_paint(
            &current.topology,
            &current.layout,
            &current.presentation,
            &current.effective,
            capability_plan,
            text_system,
        );
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
    context: &SurfaceBuildContext<'_>,
    text_system: &mut TextSystem,
) -> Result<CachedLayoutFacts, SurfacePlanningError> {
    let resolved = ResolvedSurfaceTree::for_layout(&current.topology, &current.effective);
    let (size, bounds, report, text_layouts) = layout_resolved_surface(
        &resolved,
        tree,
        context.root_constraints(),
        text_system,
        Some(current.layout.text_layouts.as_slice()),
    )?;
    Ok(CachedLayoutFacts {
        size,
        bounds,
        report,
        text_layouts,
    })
}

fn resolve_style_phase_if_dirty<Action>(
    tree: &crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    interaction: &SurfaceInteractionProjection,
    current: &mut SurfaceCache,
    capability_plan: &SurfaceCapabilityPlan,
    style_dirty: bool,
) -> bool {
    if !style_dirty {
        return false;
    }
    let next_styles = resolve_styles(
        tree,
        &current.topology,
        context.style_environment(),
        interaction,
        capability_plan,
    );
    current.interaction = Arc::new(interaction.clone());
    current.styles = Arc::new(next_styles);
    true
}

struct NonStructuralMotionStage {
    store: SurfaceMotionStore,
    activity: SurfaceMotionActivity,
    effects: EffectiveEffects,
    effective_changed: bool,
}

fn stage_non_structural_motion<Action>(
    tree: &crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    cache: Option<&SurfaceCache>,
    motion_store: &SurfaceMotionStore,
    instant: MonotonicInstant,
    current: &mut SurfaceCache,
) -> Result<NonStructuralMotionStage, SurfacePlanningError> {
    let planned = motion::plan_surface_motion(
        &motion_store.0,
        tree,
        &current.topology,
        &current.styles,
        cache,
        context.style_environment().preferences(),
        instant,
    )?;
    let (next_store, next_effective, activity) = planned.into_parts();
    let effects = current.effective.effects_against(&next_effective);
    let effective_changed = current.effective.as_ref() != &next_effective;
    if effective_changed {
        current.effective = Arc::new(next_effective);
    }
    Ok(NonStructuralMotionStage {
        store: SurfaceMotionStore(next_store),
        activity: SurfaceMotionActivity(activity),
        effects,
        effective_changed,
    })
}

const fn dirty_after_motion(
    effects: EffectiveEffects,
    layout_dirty: bool,
    paint_dirty: bool,
) -> (bool, bool, bool) {
    let layout_dirty = layout_dirty || effects.layout();
    let presentation_dirty = layout_dirty || effects.presentation();
    let paint_dirty = paint_dirty || effects.paint();
    (layout_dirty, presentation_dirty, paint_dirty)
}

fn publication_needs_recompose(
    effective_changed: bool,
    report: &SurfacePhaseReport,
    scene_diagnostics_changed: bool,
) -> bool {
    effective_changed
        || report.contains(SurfacePhase::Style)
        || report.contains(SurfacePhase::Layout)
        || report.contains(SurfacePhase::Diagnostics)
        || scene_diagnostics_changed
}

fn resolve_diagnostics_phase(
    pending: DirtyPhases,
    current: &mut SurfaceCache,
    capability_plan: &SurfaceCapabilityPlan,
    report: &mut SurfacePhaseReport,
    completed: &mut DirtyPhases,
) {
    if !pending.contains(DirtyPhases::DIAGNOSTICS) {
        return;
    }
    current.diagnostics = Arc::new(resolve_diagnostics(&current.topology, capability_plan));
    report.record(SurfacePhase::Diagnostics);
    completed.insert(DirtyPhases::DIAGNOSTICS);
}

fn placeholder_publication() -> SurfacePublication {
    SurfacePublication::new(
        SurfaceFrame::new(
            LogicalSize::new(LogicalLength::ZERO, LogicalLength::ZERO),
            Vec::new(),
        ),
        SurfaceStyleReport::default(),
        SurfaceLayoutReport::default(),
    )
}

pub(crate) fn plan_mounted_surface_cached_with_text<'tree, Action>(
    tree: &'tree mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    interaction: &SurfaceInteractionProjection,
    text_system: &mut TextSystem,
    cache: Option<&SurfaceCache>,
    motion_store: &SurfaceMotionStore,
    instant: MonotonicInstant,
) -> Result<PlannedSurfacePublication<'tree>, SurfacePlanningError> {
    let pending = tree.pending_phases();
    if cache.is_none() || pending.contains(DirtyPhases::TREE) {
        return plan_structural_surface(
            tree,
            context,
            interaction,
            text_system,
            cache,
            motion_store,
            instant,
        );
    }

    let next_context = context_key(context, text_system.source_snapshot());
    let mut current = stage_non_structural_cache(cache);
    let style_dirty = style_product_is_dirty(pending, &current, &next_context, interaction);
    let layout_dirty =
        pending.contains(DirtyPhases::LAYOUT) || layout_context_changed(&current, &next_context);
    let hit_dirty = pending.contains(DirtyPhases::HIT_TEST);
    let paint_dirty = pending.contains(DirtyPhases::PAINT);
    let mut report = SurfacePhaseReport::default();
    let mut completed = DirtyPhases::default();
    let mut capability_plan = initial_surface_capability_plan(tree, style_dirty);

    if resolve_style_phase_if_dirty(
        tree,
        context,
        interaction,
        &mut current,
        &capability_plan,
        style_dirty,
    ) {
        report.record(SurfacePhase::Style);
        completed.insert(DirtyPhases::STYLE);
    }

    let motion =
        stage_non_structural_motion(tree, context, cache, motion_store, instant, &mut current)?;
    completed.insert(DirtyPhases::MOTION);
    let (layout_dirty, presentation_dirty, paint_dirty) =
        dirty_after_motion(motion.effects, layout_dirty, paint_dirty);

    let (publication_phases, semantic_dirty) = complete_non_structural_publication_phases(
        pending,
        presentation_dirty,
        surface_capability_phases([
            (layout_dirty, DirtyPhases::LAYOUT),
            (hit_dirty, DirtyPhases::HIT_TEST),
            (paint_dirty, DirtyPhases::PAINT),
            (
                pending.contains(DirtyPhases::DIAGNOSTICS),
                DirtyPhases::DIAGNOSTICS,
            ),
        ]),
    );
    tree.extend_surface_publication_capabilities(&mut capability_plan, publication_phases);
    let semantic_capability_plan =
        semantic_dirty.then(|| tree.plan_semantic_publication_capabilities(&capability_plan));

    if layout_dirty {
        current.layout = Arc::new(resolve_layout_phase(tree, &current, context, text_system)?);
        report.record(SurfacePhase::Layout);
        completed.insert(DirtyPhases::LAYOUT);
    }
    if presentation_dirty {
        current.presentation = Arc::new(resolve_presentation(&current.layout, &current.effective)?);
    }

    let scene_diagnostics_changed = resolve_contribution_phases(
        tree,
        &mut current,
        &mut capability_plan,
        text_system,
        publication_phases,
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
    resolve_diagnostics_phase(
        pending,
        &mut current,
        &capability_plan,
        &mut report,
        &mut completed,
    );

    current.context_key = Arc::new(next_context);
    if publication_needs_recompose(motion.effective_changed, &report, scene_diagnostics_changed) {
        current.publication = compose_publication(&current);
    }
    Ok(PlannedSurfacePublication::new(
        current,
        motion.store,
        motion.activity,
        report,
        completed,
        capability_plan,
        finalized_semantics,
    ))
}

fn plan_structural_surface<'tree, Action>(
    tree: &'tree mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    interaction: &SurfaceInteractionProjection,
    text_system: &mut TextSystem,
    previous_cache: Option<&SurfaceCache>,
    motion_store: &SurfaceMotionStore,
    instant: MonotonicInstant,
) -> Result<PlannedSurfacePublication<'tree>, SurfacePlanningError> {
    let context_key = context_key(context, text_system.source_snapshot());
    let mut report = SurfacePhaseReport::default();
    let topology = collect_topology(tree);
    report.record(SurfacePhase::Tree);
    let mut capability_plan = tree.plan_surface_publication_capabilities(DirtyPhases::STYLE);
    let styles = resolve_styles(
        tree,
        &topology,
        context.style_environment(),
        interaction,
        &capability_plan,
    );
    report.record(SurfacePhase::Style);
    let planned_motion = motion::plan_surface_motion(
        &motion_store.0,
        tree,
        &topology,
        &styles,
        previous_cache,
        context.style_environment().preferences(),
        instant,
    )?;
    let (next_motion_store, effective, motion_activity) = planned_motion.into_parts();
    tree.extend_surface_publication_capabilities(&mut capability_plan, DirtyPhases::ALL);
    let semantic_capability_plan = tree.plan_semantic_publication_capabilities(&capability_plan);
    let resolved = ResolvedSurfaceTree::for_layout(&topology, &effective);
    let (size, bounds, layout_report, text_layouts) = layout_resolved_surface(
        &resolved,
        tree,
        context.root_constraints(),
        text_system,
        None,
    )?;
    let layout = CachedLayoutFacts {
        size,
        bounds,
        report: layout_report,
        text_layouts,
    };
    report.record(SurfacePhase::Layout);
    let presentation = resolve_presentation(&layout, &effective)?;

    let paint_contexts = paint_contexts(&layout, &effective);
    let hit_contexts = hit_contexts(&layout);
    tree.plan_surface_publication_contributions(
        &mut capability_plan,
        &paint_contexts,
        &hit_contexts,
    );
    let resolved_hit_test = resolve_hit_test(&topology, &presentation, &capability_plan);
    let hit_test = resolved_hit_test.scene;
    let hit_diagnostics = Arc::new(resolved_hit_test.diagnostics);
    report.record(SurfacePhase::HitTesting);
    let resolved_paint = resolve_paint(
        &topology,
        &layout,
        &presentation,
        &effective,
        &capability_plan,
        text_system,
    );
    let paint = resolved_paint.scene;
    let paint_diagnostics = Arc::new(resolved_paint.diagnostics);
    report.record(SurfacePhase::Paint);
    let finalized_semantics = tree.finalize_semantic_publication(semantic_capability_plan)?;
    #[cfg(test)]
    super::cache::note_semantics_phase_execution();
    report.record(SurfacePhase::Semantics);
    let diagnostics = resolve_diagnostics(&topology, &capability_plan);
    report.record(SurfacePhase::Diagnostics);
    let mut rebuilt = SurfaceCache {
        context_key: Arc::new(context_key),
        topology: Arc::new(topology),
        interaction: Arc::new(interaction.clone()),
        styles: Arc::new(styles),
        effective: Arc::new(effective),
        layout: Arc::new(layout),
        presentation: Arc::new(presentation),
        hit_test,
        paint,
        diagnostics: Arc::new(diagnostics),
        hit_diagnostics,
        paint_diagnostics,
        publication: placeholder_publication(),
    };
    rebuilt.publication = compose_publication(&rebuilt);
    Ok(PlannedSurfacePublication::new(
        rebuilt,
        SurfaceMotionStore(next_motion_store),
        SurfaceMotionActivity(motion_activity),
        report,
        DirtyPhases::ALL,
        capability_plan,
        Some(finalized_semantics),
    ))
}

#[cfg(test)]
fn test_text_system() -> TextSystem {
    const CANTARELL: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
    ));

    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    system
        .register_font_bytes(CANTARELL.to_vec())
        .unwrap_or_else(|_| unreachable!("controlled Cantarell test fixture is registerable"));
    let family = FontFamilyName::new("Cantarell")
        .unwrap_or_else(|_| unreachable!("controlled Cantarell family name is canonical"));
    system
        .set_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
        .unwrap_or_else(|_| unreachable!("controlled Cantarell generic mapping is valid"));
    system
}

#[cfg(test)]
std::thread_local! {
    static TEST_TEXT_SYSTEM: std::cell::RefCell<TextSystem> =
        std::cell::RefCell::new(test_text_system());
}

#[cfg(test)]
pub(super) fn plan_mounted_surface_cached_with_test_text<'tree, Action>(
    tree: &'tree mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    interaction: &SurfaceInteractionProjection,
    cache: Option<&SurfaceCache>,
) -> Result<PlannedSurfacePublication<'tree>, SurfacePlanningError> {
    TEST_TEXT_SYSTEM.with(|text_system| {
        let motion_store = SurfaceMotionStore::default();
        plan_mounted_surface_cached_with_text(
            tree,
            context,
            interaction,
            &mut text_system.borrow_mut(),
            cache,
            &motion_store,
            MonotonicInstant::ZERO,
        )
    })
}

#[cfg(test)]
pub(super) fn plan_mounted_surface_cached<'tree, Action>(
    tree: &'tree mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    interaction: &SurfaceInteractionProjection,
    cache: Option<&SurfaceCache>,
) -> Result<PlannedSurfacePublication<'tree>, SurfacePlanningError> {
    plan_mounted_surface_cached_with_test_text(tree, context, interaction, cache)
}

#[cfg(test)]
pub(super) fn publish_mounted_surface_cached<Action>(
    tree: &mut crate::mounted::MountedTree<Action>,
    context: &SurfaceBuildContext<'_>,
    cache: &mut Option<SurfaceCache>,
) -> Result<(SurfacePublication, SurfacePhaseReport), SurfacePlanningError> {
    let interaction = SurfaceInteractionProjection::default();
    let planned = plan_mounted_surface_cached(tree, context, &interaction, cache.as_ref())?;
    let commit = planned.commit_store();
    let mut motion_store = SurfaceMotionStore::default();
    let (publication, report, _activity) = commit.commit(tree, cache, &mut motion_store);
    Ok((publication, report))
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
                cache.effective.node(index).computed_style(),
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
        || cache.effective.nodes.len() != expected
        || cache.layout.bounds.len() != expected
        || cache.layout.report.nodes().len() != expected
        || cache.layout.text_layouts.len() != expected
        || cache.presentation.nodes.len() != expected
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
