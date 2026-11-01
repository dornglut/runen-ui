#![allow(refining_impl_trait)]
macro_rules! admitted {
    ($result:expr) => {{
        let result = $result;
        assert!(
            result.is_ok(),
            "expected accepted runtime admission, got {:?}",
            result.as_ref().err()
        );
        let Ok(value) = result else { return };
        value
    }};
}

use std::time::Duration;

use runenui_core::{
    CommandOrigin, Element, FocusGroup, FocusGroupActivationPolicy, FocusGroupBoundaryPolicy,
    FocusGroupTypeAhead, Focusability, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LayoutContainer, LayoutDimension, LayoutStyle, LogicalKey,
    LogicalLength, NoHostProtocol, OverflowPolicy, OverflowStyle, PhysicalKey, SemanticCommand,
    StyleEnvironment, UiApp, View, Widget, WidgetActivation, button, children, column, row,
};
use runenui_runtime::{
    AppRuntime, FocusReason, LogicalSize, MountedNodeId, PumpBudget, RuntimeConfig,
    SurfaceBuildContext, TraceConfig, TraceRecordKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    UseDiscoverablePolicy(bool),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct State {
    discoverable: bool,
}

#[derive(Debug)]
struct DisabledProbe;

impl Widget<Action> for DisabledProbe {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(false)
    }
}

struct App;

impl App {
    const fn producer_budget() -> runenui_core::ProducerBudget {
        // At most 4 mounted owners in every reachable view. Each event
        // callback declares at most 1 output: cover four focus and two
        // ordinary-route visits, activation/default, and reconciliation.
        runenui_core::ProducerBudget::new(31, 26)
            .with_reconciliation(4, 4, 4)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }
}

impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let focusability = if state.discoverable {
            Focusability::FocusableWhenDisabled
        } else {
            Focusability::Focusable
        };
        let item_layout = LayoutStyle::default()
            .with_width(LayoutDimension::Length(LogicalLength::from(20_u8)))
            .with_height(LayoutDimension::Length(LogicalLength::from(20_u8)));
        row(children![
            button("Before")
                .id("before")
                .key("before")
                .with_layout(item_layout.clone())
                .on_activate(|| Action::UseDiscoverablePolicy(true)),
            Element::new(DisabledProbe)
                .id("disabled")
                .key("disabled")
                .with_layout(item_layout.clone())
                .with_focusability(focusability),
            button("After")
                .id("after")
                .key("after")
                .with_layout(item_layout)
                .on_activate(|| Action::UseDiscoverablePolicy(true)),
        ])
        .key("root")
        .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        let Action::UseDiscoverablePolicy(discoverable) = action;
        state.discoverable = discoverable;
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    let _ = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
}

fn id(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("fixture node is mounted"))
        .id()
        .clone()
}

fn command(runtime: &mut AppRuntime<App>, target: MountedNodeId, command: SemanticCommand) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("fixture command is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .processed_envelopes(),
        1
    );
}

#[derive(Debug)]
struct FocusProbe;

impl Widget<()> for FocusProbe {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
}

#[derive(Debug)]
struct PolicyProbe {
    enabled: bool,
}

impl Widget<()> for PolicyProbe {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.enabled)
    }
}

struct GroupPolicyApp;

impl GroupPolicyApp {
    const fn producer_budget() -> runenui_core::ProducerBudget {
        // At most 5 mounted owners in every reachable view. Each event
        // callback declares at most 0 output: cover four focus and two
        // ordinary-route visits, activation/default, and reconciliation.
        runenui_core::ProducerBudget::new(8, 2)
            .with_reconciliation(5, 5, 5)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }
}

impl UiApp for GroupPolicyApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(discoverable: &Self::State) -> Element<Self::Action> {
        let disabled_focusability = if *discoverable {
            Focusability::FocusableWhenDisabled
        } else {
            Focusability::Focusable
        };
        let group = column(vec![
            Element::new(PolicyProbe { enabled: true })
                .id("group.a")
                .key("group.a"),
            Element::new(PolicyProbe { enabled: false })
                .id("group.b")
                .key("group.b")
                .with_focusability(disabled_focusability),
            Element::new(PolicyProbe { enabled: true })
                .id("group.c")
                .key("group.c"),
        ])
        .id("policy.group")
        .key("policy.group")
        .into_element()
        .focus_group(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Stop)
                .with_activation(FocusGroupActivationPolicy::Manual),
        );
        column(vec![group]).key("policy.root").into_element()
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn group_policy_id(runtime: &mut AppRuntime<GroupPolicyApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("group policy fixture node is mounted"))
        .id()
        .clone()
}

#[test]
fn disabled_discoverable_policy_participates_in_focus_group_membership() {
    for (discoverable, expected) in [(true, "group.b"), (false, "group.c")] {
        let mut runtime = admitted!(AppRuntime::<GroupPolicyApp>::mount(discoverable));
        let _ = runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned();
        let a = group_policy_id(&mut runtime, "group.a");
        let expected = group_policy_id(&mut runtime, expected);
        runtime
            .submit_command(
                a.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("group policy focus request is accepted"));
        let _ = runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned();
        runtime
            .submit_command(
                a,
                SemanticCommand::FocusGroupNext,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("group policy navigation is accepted"));
        let _ = runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned();
        assert_eq!(runtime.focus().focused_node(), Some(&expected));
    }
}

struct ScrollGroupApp;

impl ScrollGroupApp {
    const fn producer_budget() -> runenui_core::ProducerBudget {
        // At most 5 mounted owners in every reachable view. Each event
        // callback declares at most 0 output: cover four focus and two
        // ordinary-route visits, activation/default, and reconciliation.
        runenui_core::ProducerBudget::new(8, 2)
            .with_reconciliation(5, 5, 5)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }
}

impl UiApp for ScrollGroupApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        Self::producer_budget()
    }

    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        let item = |name: &'static str, search: &'static str| {
            Element::new(FocusProbe)
                .id(name)
                .key(name)
                .with_layout(
                    LayoutStyle::default()
                        .with_width(LayoutDimension::Length(LogicalLength::from(20_u8)))
                        .with_height(LayoutDimension::Length(LogicalLength::from(20_u8))),
                )
                .focus_group_search_text(search)
        };
        let group = column(vec![
            item("scroll.a", "alpha"),
            item("scroll.b", "beta"),
            item("scroll.c", "charlie"),
        ])
        .id("scroll.group")
        .key("scroll.group")
        .into_element()
        .focus_group(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Stop)
                .with_activation(FocusGroupActivationPolicy::Manual)
                .with_type_ahead(
                    FocusGroupTypeAhead::new(Duration::from_millis(500))
                        .unwrap_or_else(|_| unreachable!("fixture timeout is bounded")),
                ),
        );
        row(children![group])
            .id("scroll.viewport")
            .key("scroll.viewport")
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(LayoutDimension::Length(LogicalLength::from(20_u8)))
                    .with_height(LayoutDimension::Length(LogicalLength::from(20_u8)))
                    .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
            )
            .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn scroll_id(runtime: &mut AppRuntime<ScrollGroupApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("scroll fixture node is mounted"))
        .id()
        .clone()
}

#[test]
fn group_navigation_reveals_the_exact_new_focus_target() {
    let mut runtime = admitted!(AppRuntime::<ScrollGroupApp>::mount_with_config(
        (),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(256)),
    ));
    let _ = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(20.0, 20.0)
            .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
    );
    runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| unreachable!("scroll fixture publishes: {error:?}"));
    let a = scroll_id(&mut runtime, "scroll.a");
    let c = scroll_id(&mut runtime, "scroll.c");
    let viewport = scroll_id(&mut runtime, "scroll.viewport");

    runtime
        .submit_command(
            a.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("first group item accepts focus"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime
        .submit_command(
            a,
            SemanticCommand::FocusGroupLast,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("absolute group navigation is accepted"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(runtime.focus().focused_node(), Some(&c));
    let applied = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::LogicalScrollOwnerApplied { .. }
            )
        })
        .unwrap_or_else(|| unreachable!("group destination reveal is traced"));
    assert_eq!(
        applied
            .target()
            .and_then(|target| target.authored_id())
            .map(runenui_core::ElementId::as_str),
        Some("scroll.viewport")
    );
    assert!(matches!(
        applied.kind(),
        TraceRecordKind::LogicalScrollOwnerApplied {
            consumed,
            maximum,
            ..
        } if consumed.y().to_bits() == 40.0_f32.to_bits()
            && maximum.y().to_bits() == 40.0_f32.to_bits()
    ));

    assert_eq!(
        runtime
            .index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!("scroll viewport remains mounted"))
            .interaction()
            .scroll_offset(),
        (0.0, 40.0)
    );
    runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| unreachable!("revealed scroll fixture republishes: {error:?}"));
}

#[test]
fn type_ahead_navigation_reveals_the_exact_new_focus_target() {
    let mut runtime = admitted!(AppRuntime::<ScrollGroupApp>::mount_with_config(
        (),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(256)),
    ));
    let _ = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(20.0, 20.0)
            .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
    );
    runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| unreachable!("scroll fixture publishes: {error:?}"));

    let a = scroll_id(&mut runtime, "scroll.a");
    let c = scroll_id(&mut runtime, "scroll.c");
    let viewport = scroll_id(&mut runtime, "scroll.viewport");
    runtime
        .submit_command(
            a,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("first group item accepts focus"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyC")),
            LogicalKey::Character(String::from("c")),
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("type-ahead keyboard input is admitted"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(runtime.focus().focused_node(), Some(&c));
    assert_eq!(
        runtime
            .index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!("scroll viewport remains mounted"))
            .interaction()
            .scroll_offset(),
        (0.0, 40.0)
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::LogicalScrollOwnerApplied { consumed, .. }
            if consumed.y().to_bits() == 40.0_f32.to_bits()
    )));
}

#[test]
fn disabled_discoverable_policy_participates_while_ordinary_focusable_skips() {
    let mut discoverable = admitted!(AppRuntime::<App>::mount(State { discoverable: true }));
    settle(&mut discoverable);
    let before = id(&mut discoverable, "before");
    let disabled = id(&mut discoverable, "disabled");
    command(
        &mut discoverable,
        before.clone(),
        SemanticCommand::RequestFocus,
    );
    command(&mut discoverable, before, SemanticCommand::FocusNext);
    assert_eq!(discoverable.focus().focused_node(), Some(&disabled));
    assert!(
        discoverable
            .index()
            .node(&disabled)
            .is_some_and(runenui_runtime::MountedNodeRef::is_focusable)
    );

    let mut ordinary = admitted!(AppRuntime::<App>::mount(State {
        discoverable: false,
    }));
    settle(&mut ordinary);
    let before = id(&mut ordinary, "before");
    let after = id(&mut ordinary, "after");
    let disabled = id(&mut ordinary, "disabled");
    command(&mut ordinary, before.clone(), SemanticCommand::RequestFocus);
    command(&mut ordinary, before, SemanticCommand::FocusNext);
    assert_eq!(ordinary.focus().focused_node(), Some(&after));
    assert!(
        ordinary
            .index()
            .node(&disabled)
            .is_some_and(|node| !node.is_focusable())
    );
}

#[test]
fn disabled_discoverable_policy_participates_in_directional_navigation() {
    for (discoverable, expected) in [(true, "disabled"), (false, "after")] {
        let mut runtime = admitted!(AppRuntime::<App>::mount(State { discoverable }));
        settle(&mut runtime);
        let environment = StyleEnvironment::default();
        runtime
            .publish_surface(&SurfaceBuildContext::tight(
                &environment,
                LogicalSize::try_new(60.0, 20.0)
                    .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
            ))
            .unwrap_or_else(|error| unreachable!("directional fixture publishes: {error:?}"));

        let before = id(&mut runtime, "before");
        let expected = id(&mut runtime, expected);
        command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
        command(&mut runtime, before, SemanticCommand::FocusRight);
        assert_eq!(runtime.focus().focused_node(), Some(&expected));
    }
}

#[test]
fn removing_disabled_discoverability_clears_existing_focus_as_disablement() {
    let mut runtime = admitted!(AppRuntime::<App>::mount(State { discoverable: true }));
    settle(&mut runtime);
    let disabled = id(&mut runtime, "disabled");
    command(
        &mut runtime,
        disabled.clone(),
        SemanticCommand::RequestFocus,
    );
    assert_eq!(runtime.focus().focused_node(), Some(&disabled));

    runtime
        .submit_action(Action::UseDiscoverablePolicy(false))
        .unwrap_or_else(|_| unreachable!("fixture state update is accepted"));
    settle(&mut runtime);

    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.focus().reason(), Some(FocusReason::Disablement));
    let disabled = id(&mut runtime, "disabled");
    assert!(
        runtime
            .index()
            .node(&disabled)
            .is_some_and(|node| !node.is_focusable())
    );
}
