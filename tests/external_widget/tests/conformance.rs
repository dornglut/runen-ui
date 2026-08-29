#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Color, CommandOrigin, Element, LogicalLength, LogicalRect, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, PaintPrimitive, SemanticAction,
    SemanticCommand, SemanticContribution, SemanticContributionContext, SemanticNodeContribution,
    SemanticRole, StyleEnvironment, UiApp, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetDiagnostic, WidgetInvalidation, WidgetMeasure,
    WidgetMountContext, WidgetUnmountContext, WidgetUpdateContext, column,
};
use runenui_runtime::{
    AppRuntime, FocusReason, LayoutConstraints, MountedNodeId, PumpBudget, SubmitCommandErrorKind,
    SurfaceBuildContext, SurfacePublication,
};

fn process_one<App: UiApp>(runtime: &mut AppRuntime<App>, action: App::Action) {
    runtime
        .submit_action(action)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

fn route_activate<App: UiApp>(runtime: &mut AppRuntime<App>, target: MountedNodeId) {
    settle_initial_mounted_declarations(runtime);
    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

fn route_focus<App: UiApp>(runtime: &mut AppRuntime<App>, target: MountedNodeId) {
    settle_initial_mounted_declarations(runtime);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live focus target is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX,))
            .processed_envelopes(),
        1
    );
}

fn settle_initial_mounted_declarations<App: UiApp>(runtime: &mut AppRuntime<App>) {
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn context(environment: &StyleEnvironment) -> SurfaceBuildContext<'_> {
    SurfaceBuildContext::new(environment, LayoutConstraints::unbounded())
}

fn publish<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    environment: &StyleEnvironment,
) -> SurfacePublication {
    runtime
        .publish_surface(&context(environment))
        .unwrap_or_else(|_| unreachable!("external conformance publication is admitted"))
}

#[derive(Debug)]
struct StatefulPulse {
    name: &'static str,
    log: Rc<RefCell<Vec<String>>>,
}

#[derive(Debug)]
struct PulseState {
    activations: u16,
}

impl Widget<()> for StatefulPulse {
    type State = PulseState;
    fn create_state(&self) -> Self::State {
        PulseState { activations: 0 }
    }
    fn mount(&self, _: &mut Self::State, _: &mut WidgetMountContext) {
        self.log.borrow_mut().push(format!("mount:{}", self.name));
    }
    fn update(&self, _: &mut Self::State, _: &mut WidgetUpdateContext) {
        self.log.borrow_mut().push(format!("update:{}", self.name));
    }
    fn unmount(&self, _: &mut Self::State, context: &mut WidgetUnmountContext) {
        self.log
            .borrow_mut()
            .push(format!("unmount:{}:{:?}", self.name, context.reason()));
    }
    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext,
    ) -> WidgetActivationOutput<()> {
        state.activations = state.activations.saturating_add(1);
        context.invalidate(
            WidgetInvalidation::PAINT
                | WidgetInvalidation::SEMANTICS
                | WidgetInvalidation::DIAGNOSTICS,
        );
        WidgetActivationOutput::changed()
    }
    fn measure(&self, state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(20_u16.saturating_add(state.activations)),
            height: LogicalLength::from(10_u16),
        }
    }
    fn paint(&self, state: &Self::State, context: PaintContributionContext) -> PaintContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a valid rectangle"));
        let activation_channel =
            u8::try_from(state.activations.min(u16::from(u8::MAX))).unwrap_or(u8::MAX);
        PaintContribution::single(PaintContributionItem::fill_rect(
            rect,
            Color::rgba(activation_channel, 0, 0, 255),
        ))
    }
    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name(format!("{}:{}", self.name, state.activations))
                .with_action(SemanticAction::Activate),
        )
    }
    fn diagnostics(&self, state: &Self::State) -> Vec<WidgetDiagnostic> {
        vec![WidgetDiagnostic::new(
            "external.stateful-pulse",
            state.activations.to_string(),
        )]
    }
}

#[derive(Clone, Copy, Debug)]
enum TreeAction {
    Swap,
    RemoveA,
    Noop,
}

#[derive(Debug)]
struct TreeState {
    order: [&'static str; 2],
    show_a: bool,
    log: Rc<RefCell<Vec<String>>>,
}

struct TreeApp;
impl UiApp for TreeApp {
    type State = TreeState;
    type Action = TreeAction;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> Element<Self::Action> {
        let children: Vec<Element<()>> = state
            .order
            .iter()
            .filter(|name| **name != "a" || state.show_a)
            .map(|name| {
                Element::new(StatefulPulse {
                    name,
                    log: Rc::clone(&state.log),
                })
                .id(format!("probe.{name}"))
                .key(*name)
            })
            .collect();
        column(children)
            .key("tree.root")
            .into_element()
            .map_action(|()| TreeAction::Noop)
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            TreeAction::Swap => state.order.swap(0, 1),
            TreeAction::RemoveA => state.show_a = false,
            TreeAction::Noop => {}
        }
    }
}

fn node_id(runtime: &mut AppRuntime<TreeApp>, authored: &str) -> MountedNodeId {
    let id = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!())
        .id()
        .clone()
}

#[test]
fn keyed_reorder_preserves_mounted_state_focus_and_slots() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<TreeApp>::mount(TreeState {
        order: ["a", "b"],
        show_a: true,
        log: Rc::clone(&log),
    });
    settle_initial_mounted_declarations(&mut runtime);
    let a = node_id(&mut runtime, "probe.a");
    route_focus(&mut runtime, a.clone());
    route_activate(&mut runtime, a.clone());
    process_one(&mut runtime, TreeAction::Swap);
    let after = node_id(&mut runtime, "probe.a");
    assert_eq!(after, a);
    assert_eq!(runtime.focus().focused_node(), Some(&a));
    assert!(
        !runtime
            .index()
            .node(&a)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .pressed()
    );
    let environment = StyleEnvironment::default();
    let publication = publish(&mut runtime, &environment);
    assert!(publication.paint_scene().items().iter().any(|item| {
        matches!(
            item.primitive(),
            PaintPrimitive::FillRect { rect, color }
                if (rect.width() - 21.0).abs() <= f32::EPSILON
                    && *color == Color::rgba(1, 0, 0, 255)
        )
    }));
    assert_eq!(runtime.reconciliation_report().moved_count(), 2);
    assert!(log.borrow().iter().any(|entry| entry == "update:a"));
    assert!(
        !log.borrow()
            .iter()
            .any(|entry| entry.starts_with("unmount:a"))
    );
}

#[test]
fn removal_makes_ids_stale_clears_focus_and_shutdown_unmounts_once() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<TreeApp>::mount(TreeState {
        order: ["a", "b"],
        show_a: true,
        log: Rc::clone(&log),
    });
    settle_initial_mounted_declarations(&mut runtime);
    let a = node_id(&mut runtime, "probe.a");
    route_focus(&mut runtime, a.clone());
    process_one(&mut runtime, TreeAction::RemoveA);
    let Err(error) = runtime.submit_command(
        a.clone(),
        SemanticCommand::Activate,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("the removed mounted lifetime is stale")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::StaleTarget);
    let Err(error) = runtime.submit_command(
        a,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("the removed focus target is stale")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::StaleTarget);
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.focus().reason(), Some(FocusReason::Removal));
    let _state = runtime.into_state();
    let entries = log.borrow();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.starts_with("unmount:a"))
            .count(),
        1
    );
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.starts_with("unmount:b"))
            .count(),
        1
    );
}

#[test]
fn foreign_runtime_targets_are_rejected() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut first = AppRuntime::<TreeApp>::mount(TreeState {
        order: ["a", "b"],
        show_a: true,
        log: Rc::clone(&log),
    });
    let mut second = AppRuntime::<TreeApp>::mount(TreeState {
        order: ["a", "b"],
        show_a: true,
        log,
    });
    let id = node_id(&mut first, "probe.a");
    let Err(error) = second.submit_command(
        id.clone(),
        SemanticCommand::Activate,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("a target from another runtime is foreign")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::ForeignTarget);
    let Err(error) = second.submit_command(
        id,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("a focus target from another runtime is foreign")
    };
    assert_eq!(error.kind(), SubmitCommandErrorKind::ForeignTarget);
}

#[test]
fn mounted_publication_products_are_exactly_aligned() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<TreeApp>::mount(TreeState {
        order: ["a", "b"],
        show_a: true,
        log,
    });
    let index_ids: Vec<_> = runtime
        .index()
        .nodes()
        .iter()
        .map(|node| (node.id().clone(), node.parent().cloned()))
        .collect();
    let environment = StyleEnvironment::default();
    let publication = publish(&mut runtime, &environment);
    let frame_ids: Vec<_> = publication
        .frame()
        .nodes()
        .iter()
        .map(|node| (node.id().clone(), node.parent().cloned()))
        .collect();
    let style_ids: Vec<_> = publication
        .style_report()
        .nodes()
        .iter()
        .map(|node| (node.id().clone(), node.parent().cloned()))
        .collect();
    let layout_ids: Vec<_> = publication
        .layout_report()
        .nodes()
        .iter()
        .map(|node| (node.id().clone(), node.parent().cloned()))
        .collect();
    assert_eq!(index_ids, frame_ids);
    assert_eq!(index_ids, style_ids);
    assert_eq!(index_ids, layout_ids);
}

#[derive(Debug)]
struct SelfDisabling;

#[derive(Debug)]
struct SelfDisablingState {
    enabled: bool,
}

impl Widget<()> for SelfDisabling {
    type State = SelfDisablingState;

    fn create_state(&self) -> Self::State {
        SelfDisablingState { enabled: true }
    }

    fn activation(&self, state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(state.enabled)
    }

    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext,
    ) -> WidgetActivationOutput<()> {
        state.enabled = false;
        context.invalidate(WidgetInvalidation::INTERACTION);
        WidgetActivationOutput::changed()
    }
}

struct SelfDisablingApp;

impl UiApp for SelfDisablingApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(SelfDisabling).key("self-disabling")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn state_only_interaction_invalidation_validates_focus_immediately() {
    let mut runtime = AppRuntime::<SelfDisablingApp>::mount(());
    let id = runtime.index().nodes()[0].id().clone();
    route_focus(&mut runtime, id.clone());
    route_activate(&mut runtime, id);
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.focus().reason(), Some(FocusReason::Disablement));
    assert!(!runtime.index().nodes()[0].is_focusable());
}

#[derive(Debug)]
struct SelfRetaining;

impl Widget<()> for SelfRetaining {
    type State = usize;

    fn create_state(&self) -> Self::State {
        0
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext,
    ) -> WidgetActivationOutput<()> {
        *state += 1;
        context.invalidate(WidgetInvalidation::INTERACTION);
        WidgetActivationOutput::changed()
    }
}

struct SelfRetainingApp;

impl UiApp for SelfRetainingApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(SelfRetaining).key("self-retaining")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn state_only_interaction_invalidation_preserves_still_valid_focus() {
    let mut runtime = AppRuntime::<SelfRetainingApp>::mount(());
    let id = runtime.index().nodes()[0].id().clone();
    route_focus(&mut runtime, id.clone());
    route_activate(&mut runtime, id.clone());
    assert_eq!(runtime.focus().focused_node(), Some(&id));
    assert!(runtime.index().nodes()[0].is_focusable());
}
