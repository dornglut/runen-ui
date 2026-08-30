use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Color, Element, LogicalLength, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticRole, StyleEnvironment, StyleIntent, StyleInteractionState,
    StyleProperties, StyleRecipe, StyleRecipeId, StyleTheme, StyleTokens, View, Widget,
    WidgetActivation, WidgetInvalidation, WidgetMeasure, children, column, text,
};

use super::{
    SurfaceBuildContext, SurfacePhaseReport, SurfacePublication, cache::SurfaceCache,
    cache::phase_function_counts, cache::reset_phase_function_counts, plan_mounted_surface_cached,
    publish_mounted_surface_cached,
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

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        self.semantic_callbacks
            .set(self.semantic_callbacks.get() + 1);
        SemanticContribution::single(SemanticNodeContribution::primary(SemanticRole::Button))
    }
}

#[derive(Debug)]
struct ActivationStyleProbe {
    enabled: Rc<Cell<bool>>,
    activation_callbacks: Rc<Cell<usize>>,
}

impl Widget<()> for ActivationStyleProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        self.activation_callbacks
            .set(self.activation_callbacks.get() + 1);
        WidgetActivation::actionable(self.enabled.get())
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

fn assert_retained_reuse(before: &SurfaceCache, cache: Option<&SurfaceCache>, expected: [bool; 7]) {
    let after = cache.unwrap_or_else(|| unreachable!("publication retains phase products"));
    assert_eq!(before.retained_product_reuse(after), expected);
}

#[test]
fn phase_function_counters_track_only_actual_execution_branches() {
    let (mut tree, _) = MountedTree::<()>::mount(text("phase").key("root").into_element());
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
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
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
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
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
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
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
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
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
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
fn disabled_style_uses_shared_activation_and_interaction_invalidation() {
    let enabled = Rc::new(Cell::new(true));
    let activation_callbacks = Rc::new(Cell::new(0));
    let (mut tree, _) = MountedTree::<()>::mount(
        Element::new(ActivationStyleProbe {
            enabled: Rc::clone(&enabled),
            activation_callbacks: Rc::clone(&activation_callbacks),
        })
        .key("root"),
    );
    let root = tree.publication_preorder_ids()[0].clone();
    let recipe_id = StyleRecipeId::from_static("control")
        .unwrap_or_else(|_| unreachable!("test recipe identifier is valid"));
    let mut recipe = StyleRecipe::new(StyleProperties::EMPTY.with_background(Color::BLACK));
    recipe
        .define_interaction(
            StyleInteractionState::Disabled,
            StyleProperties::EMPTY.with_background(Color::WHITE),
        )
        .unwrap_or_else(|_| unreachable!("test recipe defines disabled once"));
    let mut theme = StyleTheme::new(StyleTokens::new());
    theme
        .define_recipe(recipe_id.clone(), recipe)
        .unwrap_or_else(|_| unreachable!("test theme defines recipe once"));
    tree.node_mut(&root)
        .unwrap_or_else(|| unreachable!("activation style probe remains mounted"))
        .style = StyleIntent::EMPTY.with_recipe(recipe_id);

    let environment = StyleEnvironment::new(theme);
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
    let mut cache = None;
    let (initial, _) = publish(&mut tree, &context, &mut cache);
    assert_eq!(activation_callbacks.get(), 1);
    assert_eq!(
        initial
            .frame()
            .root()
            .unwrap_or_else(|| unreachable!("initial publication has root"))
            .computed_style()
            .background(),
        Some(Color::BLACK)
    );

    enabled.set(false);
    let node = tree
        .node_mut(&root)
        .unwrap_or_else(|| unreachable!("activation style probe remains mounted"));
    apply_invalidation(node, WidgetInvalidation::INTERACTION);
    let (disabled, report) = publish(&mut tree, &context, &mut cache);
    assert_eq!(activation_callbacks.get(), 2);
    assert_eq!(
        report.executed(),
        &[
            super::SurfacePhase::Style,
            super::SurfacePhase::Paint,
            super::SurfacePhase::Semantics,
        ]
    );
    assert_eq!(
        disabled
            .frame()
            .root()
            .unwrap_or_else(|| unreachable!("disabled publication has root"))
            .computed_style()
            .background(),
        Some(Color::WHITE)
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
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
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
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
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
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
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
