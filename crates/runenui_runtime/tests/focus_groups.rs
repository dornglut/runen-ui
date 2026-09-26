#![cfg(feature = "internal-test-seams")]
#![allow(refining_impl_trait)]

use runenui_core::{
    ChildBearingWidget, CommandOrigin, Element, EventContext, EventPhase, FocusBoundaryPolicy,
    FocusGroup, FocusGroupActivationPolicy, FocusGroupBoundaryPolicy, FocusReason, FocusScope,
    FocusScopePolicy, NoHostProtocol, SemanticCommand, StyleEnvironment, UiApp, UiEvent, View,
    Widget, WidgetEventOutput, button, column, container,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, ReconciliationDiagnostic,
    RuntimeConfig, RuntimeLimits, RuntimeStatus, SurfaceBuildContext, TraceRecordKind,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct State {
    disable_preferred: bool,
    disable_all: bool,
    hide_preferred: bool,
    duplicate_preferred: bool,
    manual_activation: bool,
    stop_boundary: bool,
    activations: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Activated(&'static str),
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> Element<Action> {
        let boundary = if state.stop_boundary {
            FocusGroupBoundaryPolicy::Stop
        } else {
            FocusGroupBoundaryPolicy::Wrap
        };
        let activation = if state.manual_activation {
            FocusGroupActivationPolicy::Manual
        } else {
            FocusGroupActivationPolicy::ActivateTarget
        };
        let group = column(vec![
            member(state, "a"),
            member(state, "b"),
            member(state, "c"),
        ])
        .id("group")
        .key("group")
        .into_element()
        .focus_group(
            FocusGroup::new()
                .with_boundary(boundary)
                .with_activation(activation),
        );
        column(vec![member(state, "before"), group, member(state, "after")])
            .key("root")
            .into_element()
    }

    fn update(state: &mut State, action: Action) {
        let Action::Activated(name) = action;
        state.activations.push(name);
    }
}

fn member(state: &State, name: &'static str) -> Element<Action> {
    let mut control = button(name)
        .id(name)
        .key(name)
        .on_activate(move || Action::Activated(name));
    if state.disable_all || (state.disable_preferred && name == "b") {
        control = control.disabled();
    }
    let mut element = control.into_element();
    if name == "b" || (state.duplicate_preferred && name == "a") {
        element = element.focus_group_preferred(true);
    }
    if state.hide_preferred && name == "b" {
        element = element.focus_hidden(true);
    }
    element
}

fn settle(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn id(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(name).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("named focus-group corpus node is mounted"))
        .id()
        .clone()
}

fn command(runtime: &mut AppRuntime<App>, target: MountedNodeId, command: SemanticCommand) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("live focus-group command is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

fn publish_geometry(runtime: &mut AppRuntime<App>) {
    let style_environment = StyleEnvironment::default();
    let _ = runtime.publish_surface(&SurfaceBuildContext::new(
        &style_environment,
        LayoutConstraints::unbounded(),
    ));
    let issued = [
        ("before", [0.0, 0.0, 10.0, 10.0]),
        ("group", [20.0, 0.0, 30.0, 10.0]),
        ("a", [20.0, 0.0, 10.0, 10.0]),
        ("b", [30.0, 0.0, 10.0, 10.0]),
        ("c", [40.0, 0.0, 10.0, 10.0]),
        ("after", [60.0, 0.0, 10.0, 10.0]),
    ]
    .into_iter()
    .map(|(name, rect)| (id(runtime, name), rect))
    .collect::<Vec<_>>();
    runtime.__replace_current_focus_geometry_for_test(&issued);
}

#[test]
fn external_traversal_collapses_group_and_uses_preferred_entry() {
    let mut runtime = AppRuntime::<App>::mount(State::default());
    settle(&mut runtime);
    publish_geometry(&mut runtime);

    let before = id(&mut runtime, "before");
    let preferred = id(&mut runtime, "b");
    let after = id(&mut runtime, "after");

    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before.clone(), SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&preferred));
    assert_eq!(
        runtime.focus().reason(),
        Some(FocusReason::LinearNavigation)
    );

    command(&mut runtime, preferred.clone(), SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&after));

    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before, SemanticCommand::FocusRight);
    assert_eq!(runtime.focus().focused_node(), Some(&preferred));
    assert_eq!(
        runtime.focus().reason(),
        Some(FocusReason::DirectionalNavigation)
    );
}

#[test]
fn preferred_entry_falls_back_when_preferred_member_is_disabled() {
    let mut runtime = AppRuntime::<App>::mount(State {
        disable_preferred: true,
        ..State::default()
    });
    settle(&mut runtime);
    let before = id(&mut runtime, "before");
    let fallback = id(&mut runtime, "a");
    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before, SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&fallback));
}

#[test]
fn zero_eligible_group_contributes_no_external_focus_stop() {
    let mut runtime = AppRuntime::<App>::mount(State {
        disable_all: true,
        ..State::default()
    });
    settle(&mut runtime);

    let before = id(&mut runtime, "before");
    let after = id(&mut runtime, "after");
    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before, SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&after));
}

#[test]
fn internal_navigation_wraps_and_activation_is_deferred_until_after_focus() {
    let mut runtime = AppRuntime::<App>::mount(State::default());
    settle(&mut runtime);
    let b = id(&mut runtime, "b");
    let c = id(&mut runtime, "c");
    let a = id(&mut runtime, "a");

    command(&mut runtime, b.clone(), SemanticCommand::RequestFocus);
    let trace_start = runtime.trace().len();
    command(&mut runtime, b, SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&c));
    assert_eq!(runtime.focus().reason(), Some(FocusReason::GroupNavigation));
    assert!(runtime.state().activations.is_empty());

    let records = runtime
        .trace()
        .records()
        .skip(trace_start)
        .collect::<Vec<_>>();
    let transition = records
        .iter()
        .position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusTransitionCommitted {
                    reason: FocusReason::GroupNavigation,
                }
            )
        })
        .unwrap_or_else(|| unreachable!("group focus transition is traced"));
    let delegated_activation = records
        .iter()
        .position(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.original_target() == Some(&c)
        })
        .unwrap_or_else(|| unreachable!("delegated activation is accepted for the new member"));
    assert!(transition < delegated_activation);
    settle(&mut runtime);
    assert_eq!(runtime.state().activations, vec!["c"]);

    command(&mut runtime, c, SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&a));
    settle(&mut runtime);
    assert_eq!(runtime.state().activations, vec!["c", "a"]);
}

#[test]
fn hidden_preferred_entry_falls_back_and_internal_navigation_skips_ineligible_members() {
    let mut runtime = AppRuntime::<App>::mount(State {
        hide_preferred: true,
        ..State::default()
    });
    settle(&mut runtime);

    let before = id(&mut runtime, "before");
    let a = id(&mut runtime, "a");
    let c = id(&mut runtime, "c");
    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before, SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&a));

    command(&mut runtime, a.clone(), SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&c));

    let mut runtime = AppRuntime::<App>::mount(State {
        disable_preferred: true,
        ..State::default()
    });
    settle(&mut runtime);
    let a = id(&mut runtime, "a");
    let c = id(&mut runtime, "c");
    command(&mut runtime, a.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, a, SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&c));
}

#[test]
fn manual_stop_policy_moves_without_activation_and_stops_at_boundary() {
    let mut runtime = AppRuntime::<App>::mount(State {
        manual_activation: true,
        stop_boundary: true,
        ..State::default()
    });
    settle(&mut runtime);

    let b = id(&mut runtime, "b");
    let c = id(&mut runtime, "c");
    command(&mut runtime, b.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, b, SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&c));
    settle(&mut runtime);
    assert!(runtime.state().activations.is_empty());

    command(&mut runtime, c.clone(), SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&c));
    settle(&mut runtime);
    assert!(runtime.state().activations.is_empty());
}

#[test]
fn programmatic_focus_can_target_any_exact_eligible_group_member() {
    let mut runtime = AppRuntime::<App>::mount(State::default());
    settle(&mut runtime);
    let c = id(&mut runtime, "c");
    command(&mut runtime, c.clone(), SemanticCommand::RequestFocus);
    assert_eq!(runtime.focus().focused_node(), Some(&c));
    assert_eq!(
        runtime.focus().reason(),
        Some(FocusReason::ProgrammaticRequest)
    );
}

struct NestedApp;

impl UiApp for NestedApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &State) -> Element<Action> {
        let nested = column(vec![nested_member("x", false), nested_member("y", true)])
            .id("inner")
            .key("inner")
            .into_element()
            .focus_group(
                FocusGroup::new()
                    .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                    .with_activation(FocusGroupActivationPolicy::Manual),
            );
        let outer = column(vec![
            nested_member("a", false),
            nested,
            nested_member("c", false),
        ])
        .id("outer")
        .key("outer")
        .into_element()
        .focus_group(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                .with_activation(FocusGroupActivationPolicy::Manual),
        );
        column(vec![
            nested_member("before", false),
            outer,
            nested_member("after", false),
        ])
        .key("root")
        .into_element()
    }

    fn update(state: &mut State, action: Action) {
        let Action::Activated(name) = action;
        state.activations.push(name);
    }
}

fn nested_member(name: &'static str, preferred: bool) -> Element<Action> {
    let mut element = button(name)
        .id(name)
        .key(name)
        .on_activate(move || Action::Activated(name))
        .into_element();
    if preferred {
        element = element.focus_group_preferred(true);
    }
    element
}

fn nested_id(runtime: &mut AppRuntime<NestedApp>, name: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(name).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("named nested-group corpus node is mounted"))
        .id()
        .clone()
}

fn nested_command(
    runtime: &mut AppRuntime<NestedApp>,
    target: MountedNodeId,
    command: SemanticCommand,
) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("live nested focus-group command is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

#[test]
fn nested_groups_use_nearest_ownership_and_outer_group_treats_inner_as_one_member() {
    let mut runtime = AppRuntime::<NestedApp>::mount(State::default());
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));

    let a = nested_id(&mut runtime, "a");
    let outer = nested_id(&mut runtime, "outer");
    let inner = nested_id(&mut runtime, "inner");
    let y = nested_id(&mut runtime, "y");
    let x = nested_id(&mut runtime, "x");
    let c = nested_id(&mut runtime, "c");

    nested_command(&mut runtime, a.clone(), SemanticCommand::RequestFocus);
    nested_command(&mut runtime, outer.clone(), SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&y));

    nested_command(&mut runtime, y.clone(), SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&x));

    nested_command(&mut runtime, inner, SemanticCommand::FocusGroupPrevious);
    assert_eq!(runtime.focus().focused_node(), Some(&y));

    nested_command(&mut runtime, outer, SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&c));
}

#[test]
fn invalid_multiple_preferred_members_diagnose_and_fail_closed() {
    let mut runtime = AppRuntime::<App>::mount(State {
        duplicate_preferred: true,
        ..State::default()
    });
    settle(&mut runtime);

    assert!(
        runtime
            .reconciliation_report()
            .diagnostics()
            .iter()
            .any(|diagnostic| {
                matches!(
                        diagnostic,
                        ReconciliationDiagnostic::MultiplePreferredFocusGroupMembers {
                    group_path,
                    preferred_member_paths,
                } if group_path == "root/1"
                    && preferred_member_paths == &vec![
                        String::from("root/1/0"),
                        String::from("root/1/1"),
                    ]
                    )
            })
    );

    let before = id(&mut runtime, "before");
    let after = id(&mut runtime, "after");
    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before, SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&after));
}

struct ScopeBoundaryApp;

impl UiApp for ScopeBoundaryApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &State) -> Element<Action> {
        let inner_scope = column(vec![
            nested_member("scope.x", true),
            nested_member("scope.y", false),
        ])
        .id("scope.inner")
        .key("scope.inner")
        .into_element()
        .focus_scope(FocusScope::new().with_policy(FocusScopePolicy::new(
            FocusBoundaryPolicy::Trap,
            FocusBoundaryPolicy::Trap,
        )));
        let outer_group = column(vec![
            nested_member("scope.a", true),
            inner_scope,
            nested_member("scope.c", false),
        ])
        .id("scope.outer")
        .key("scope.outer")
        .into_element()
        .focus_group(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                .with_activation(FocusGroupActivationPolicy::Manual),
        );
        column(vec![outer_group]).key("scope.root").into_element()
    }

    fn update(state: &mut State, action: Action) {
        let Action::Activated(name) = action;
        state.activations.push(name);
    }
}

fn scope_boundary_id(runtime: &mut AppRuntime<ScopeBoundaryApp>, name: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(name).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("named scope-boundary node is mounted"))
        .id()
        .clone()
}

#[test]
fn nested_focus_scope_is_not_absorbed_by_or_escaped_through_outer_focus_group() {
    let mut runtime = AppRuntime::<ScopeBoundaryApp>::mount(State::default());
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));

    let a = scope_boundary_id(&mut runtime, "scope.a");
    let c = scope_boundary_id(&mut runtime, "scope.c");
    let x = scope_boundary_id(&mut runtime, "scope.x");

    runtime
        .submit_command(
            a.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("outer member focus request is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_command(
            a,
            SemanticCommand::FocusGroupNext,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("outer group navigation is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.focus().focused_node(), Some(&c));
    assert!(
        !runtime
            .reconciliation_report()
            .diagnostics()
            .iter()
            .any(|diagnostic| matches!(
                diagnostic,
                ReconciliationDiagnostic::MultiplePreferredFocusGroupMembers { .. }
            ))
    );

    runtime
        .submit_command(
            x.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("inner-scope focus request is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.focus().focused_node(), Some(&x));

    runtime
        .submit_command(
            x.clone(),
            SemanticCommand::FocusGroupNext,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("inner-scope group command routes normally"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.focus().focused_node(), Some(&x));
}

#[derive(Debug)]
struct OutputPressureGroup;

impl Widget<Action> for OutputPressureGroup {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Bubble
            && event.as_semantic_command().is_some_and(|command| {
                matches!(
                    command.command(),
                    SemanticCommand::FocusGroupNext | SemanticCommand::FocusGroupPrevious
                )
            })
        {
            context.emit(Action::Activated("routed"));
        }
        WidgetEventOutput::none()
    }
}

impl ChildBearingWidget<Action> for OutputPressureGroup {}

struct OutputPressureApp;

impl UiApp for OutputPressureApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &State) -> Element<Action> {
        let group = container(
            OutputPressureGroup,
            vec![
                button("a")
                    .id("pressure.a")
                    .key("pressure.a")
                    .on_activate(|| Action::Activated("a"))
                    .into_element(),
                button("b")
                    .id("pressure.b")
                    .key("pressure.b")
                    .on_activate(|| Action::Activated("b"))
                    .into_element(),
            ],
        )
        .id("pressure.group")
        .key("pressure.group")
        .into_element()
        .focus_group(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                .with_activation(FocusGroupActivationPolicy::ActivateTarget),
        );
        column(vec![group]).key("pressure.root").into_element()
    }

    fn update(state: &mut State, action: Action) {
        let Action::Activated(name) = action;
        state.activations.push(name);
    }
}

fn pressure_id(runtime: &mut AppRuntime<OutputPressureApp>, name: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(name).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("named output-pressure node is mounted"))
        .id()
        .clone()
}

#[test]
fn activate_target_reserves_default_command_capacity_beyond_routed_callback_outputs() {
    let config =
        RuntimeConfig::default().with_limits(RuntimeLimits::default().with_transaction_outputs(1));
    let mut runtime = AppRuntime::<OutputPressureApp>::mount_with_config(State::default(), config);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));

    let a = pressure_id(&mut runtime, "pressure.a");
    let b = pressure_id(&mut runtime, "pressure.b");

    runtime
        .submit_command(
            a.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("pressure focus request is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );

    runtime
        .submit_command(
            a,
            SemanticCommand::FocusGroupNext,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("pressure group navigation is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );

    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.focus().focused_node(), Some(&b));
    assert!(runtime.state().activations.is_empty());

    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.state().activations, vec!["routed", "b"]);
}
