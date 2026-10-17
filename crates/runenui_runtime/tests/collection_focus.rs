#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    CommandOrigin, Element, FocusEventKind, FocusGroup, FocusGroupActivationPolicy,
    FocusGroupBoundaryPolicy, FocusGroupTypeAhead, Focusability, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LayoutContainer, LayoutDimension,
    LayoutStyle, LogicalKey, LogicalLength, NoHostProtocol, OverflowPolicy, OverflowStyle,
    PhysicalKey, SemanticCommand, StyleEnvironment, UiApp, View, Widget, WidgetActivation, button,
    children, column, row,
};
use runenui_runtime::{
    AppRuntime, FocusReason, LogicalSize, MountedNodeId, PumpBudget, RuntimeConfig, RuntimeLimits,
    RuntimeStatus, RuntimeTerminalReason, SurfaceBuildContext, TraceConfig, TraceDeliveryOutcome,
    TraceRecordKind,
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
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(false)
    }
}

struct App;

impl UiApp for App {
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
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.enabled)
    }
}

struct GroupPolicyApp;

impl UiApp for GroupPolicyApp {
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
        let mut runtime = AppRuntime::<GroupPolicyApp>::mount(discoverable);
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

impl UiApp for ScrollGroupApp {
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
    let mut runtime = AppRuntime::<ScrollGroupApp>::mount_with_config(
        (),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(256)),
    );
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
    let mut runtime = AppRuntime::<ScrollGroupApp>::mount_with_config(
        (),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(256)),
    );
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
    let mut discoverable = AppRuntime::<App>::mount(State { discoverable: true });
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

    let mut ordinary = AppRuntime::<App>::mount(State {
        discoverable: false,
    });
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
        let mut runtime = AppRuntime::<App>::mount(State { discoverable });
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
    let mut runtime = AppRuntime::<App>::mount(State { discoverable: true });
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
    assert!(
        runtime.trace().records().any(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        }),
        "compatible disablement must route noncancelable FocusOut while the old route remains live"
    );
    let disabled = id(&mut runtime, "disabled");
    assert!(
        runtime
            .index()
            .node(&disabled)
            .is_some_and(|node| !node.is_focusable())
    );
}

#[test]
fn compatible_disablement_queue_saturation_does_not_claim_focus_out_delivery() {
    const CAPACITY: usize = 16;
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(CAPACITY)
        .with_transaction_outputs(1);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State { discoverable: true },
        RuntimeConfig::default().with_limits(limits),
    );
    settle(&mut runtime);
    let owner = id(&mut runtime, "disabled");
    command(&mut runtime, owner, SemanticCommand::RequestFocus);
    let trace_before = runtime.trace().len();

    for _ in 0..CAPACITY {
        runtime
            .submit_action(Action::UseDiscoverablePolicy(false))
            .unwrap_or_else(|_| unreachable!("the queue admits exactly its full capacity"));
    }
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("bounded pump must return the terminal outcome"));
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned),
        "missing capacity after application mutation must not leave a half-running runtime"
    );
    assert!(
        !runtime.trace().records().skip(trace_before).any(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        }),
        "a rejected routed FocusOut cannot be reported as delivered"
    );
}

#[test]
fn output_free_focus_retirement_has_exact_queue_admission_boundary() {
    const CAPACITY: usize = 16;
    // 15 queued actions leave 14 waiting after the initiating action is popped:
    // two conservative slots remain and the zero-output FocusOut is admitted.
    // 16 queued actions leave 15 waiting and only one free slot: this candidate
    // terminalizes *after* mutation instead of claiming a delivered callback.
    for (submitted, admitted) in [(15, true), (16, false)] {
        let mut runtime = AppRuntime::<App>::mount_with_config(
            State { discoverable: true },
            RuntimeConfig::default().with_limits(
                RuntimeLimits::default()
                    .with_waiting_envelopes(CAPACITY)
                    .with_transaction_outputs(1),
            ),
        );
        settle(&mut runtime);
        let owner = id(&mut runtime, "disabled");
        command(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
        assert_eq!(runtime.focus().focused_node(), Some(&owner));
        let trace_start = runtime.trace().len();
        for index in 0..submitted {
            runtime
                .submit_action(Action::UseDiscoverablePolicy(index != 0))
                .unwrap_or_else(|_| unreachable!("the configured waiting queue admits input"));
        }
        let one = runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("one-envelope pump reports completion or terminal"));
        assert_eq!(one.report().processed_envelopes(), 1);
        assert_eq!(
            runtime.focus().focused_node(),
            (!admitted).then_some(&owner),
            "admitted retirement clears focus; rejected preflight cannot claim a committed transition"
        );
        assert!(
            runtime.index().node(&owner).is_some(),
            "the original disabled owner is still mounted"
        );
        assert!(
            !runtime.state().discoverable,
            "the initiating application action has already mutated state"
        );
        let delivered = runtime.trace().records().skip(trace_start).any(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        });
        assert_eq!(delivered, admitted);
        if admitted {
            assert_eq!(runtime.status(), RuntimeStatus::Running);
        } else {
            assert_eq!(
                runtime.status(),
                RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
            );
            assert!(
                runtime.trace().records().skip(trace_start).any(|record| {
                    matches!(
                        record.kind(),
                        TraceRecordKind::RoutedEventAdmissionRejected {
                            capacity:
                                runenui_runtime::TraceRoutedAdmissionRejection::WaitingEnvelopes
                        }
                    )
                }),
                "checked queue pressure must report the precise rejected authority"
            );
        }
    }
}

#[test]
fn default_limits_focusout_requires_the_full_conservative_queue_headroom() {
    // Production defaults: 4096 waiting envelopes and 1024 possible
    // routed outputs, conservatively reserving 2048 queue positions.
    // These are synthetic bursts, not representative throughput measurements.
    const REQUIRED_FREE_SLOTS: usize = 2 * 1024;
    const WAITING_CAPACITY: usize = 4096;
    for (submitted, delivered) in [
        (WAITING_CAPACITY - REQUIRED_FREE_SLOTS + 1, true),
        (WAITING_CAPACITY - REQUIRED_FREE_SLOTS + 2, false),
    ] {
        let mut runtime = AppRuntime::<App>::mount(State { discoverable: true });
        settle(&mut runtime);
        let owner = id(&mut runtime, "disabled");
        command(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
        let trace_start = runtime.trace().len();

        for index in 0..submitted {
            runtime
                .submit_action(Action::UseDiscoverablePolicy(index != 0))
                .unwrap_or_else(|_| unreachable!("default queue accepts the synthetic burst"));
        }
        let batch = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("one-envelope pump remains observable"));
        assert_eq!(batch.report().processed_envelopes(), 1);
        assert!(!runtime.state().discoverable);
        let focused = runtime.focus().focused_node();
        assert_eq!(focused, (!delivered).then_some(&owner));
        let delivered_trace = runtime.trace().records().skip(trace_start).any(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        });
        assert_eq!(delivered_trace, delivered);
        if delivered {
            assert_eq!(runtime.status(), RuntimeStatus::Running);
            assert_eq!(batch.report().remaining_queued_envelopes(), submitted - 1);
        } else {
            assert_eq!(
                runtime.status(),
                RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
            );
            assert_eq!(
                batch.report().outcome(),
                runenui_runtime::PumpOutcome::Terminal(RuntimeTerminalReason::Poisoned)
            );
            assert!(
                runtime
                    .trace()
                    .records()
                    .skip(trace_start)
                    .any(|record| matches!(
                        record.kind(),
                        TraceRecordKind::RoutedEventAdmissionRejected {
                            capacity:
                                runenui_runtime::TraceRoutedAdmissionRejection::WaitingEnvelopes
                        }
                    ))
            );
        }
    }
}

#[test]
fn queue_pressure_without_focus_retirement_does_not_reserve_optional_focus_out() {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State { discoverable: true },
        RuntimeConfig::default().with_limits(
            RuntimeLimits::default()
                .with_waiting_envelopes(16)
                .with_transaction_outputs(1),
        ),
    );
    settle(&mut runtime);
    let owner = id(&mut runtime, "disabled");
    command(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    for _ in 0..16 {
        runtime
            .submit_action(Action::UseDiscoverablePolicy(true))
            .unwrap_or_else(|_| unreachable!("all queue positions are admitted"));
    }
    let one = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("the first action is processed"));
    assert_eq!(one.report().processed_envelopes(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn reconciled_focusout_trace_exhaustion_reports_one_terminal_reason() {
    let mut runtime = AppRuntime::<App>::mount(State { discoverable: true });
    settle(&mut runtime);
    let owner = id(&mut runtime, "disabled");
    command(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    assert_eq!(runtime.focus().focused_node(), Some(&owner));

    runtime
        .submit_action(Action::UseDiscoverablePolicy(false))
        .unwrap_or_else(|_| unreachable!("application action admitted before trace exhaustion"));
    // Leave enough trace IDs for the base application transaction but not
    // the maximum routed FocusOut plan after opaque state reconciliation.
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 30);
    let batch = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("terminal pump must remain observable"));
    assert_eq!(batch.report().processed_envelopes(), 1);
    assert!(!runtime.state().discoverable);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert_eq!(
        batch.report().outcome(),
        runenui_runtime::PumpOutcome::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: runenui_runtime::TraceRoutedAdmissionRejection::TraceSequenceExhausted
        }
    )));
    assert!(!runtime.trace().records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::FocusNotificationResolved {
                kind: FocusEventKind::Out
            }
        ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
    }));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn reconciled_focusout_work_sequence_exhaustion_has_one_terminal_reason() {
    let mut runtime = AppRuntime::<App>::mount(State { discoverable: true });
    settle(&mut runtime);
    let owner = id(&mut runtime, "disabled");
    command(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);

    runtime
        .submit_action(Action::UseDiscoverablePolicy(false))
        .unwrap_or_else(|_| unreachable!("action admitted before sequence limit"));
    runtime.__seed_next_work_sequence_for_test(u64::MAX);
    let batch = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("terminal sequence exhaustion remains observable"));
    assert_eq!(batch.report().processed_envelopes(), 1);
    assert!(!runtime.state().discoverable);
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert_eq!(
        batch.report().outcome(),
        runenui_runtime::PumpOutcome::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: runenui_runtime::TraceRoutedAdmissionRejection::WorkSequenceExhausted
        }
    )));
    assert!(!runtime.trace().records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::FocusNotificationResolved {
                kind: FocusEventKind::Out
            }
        ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
    }));
}
