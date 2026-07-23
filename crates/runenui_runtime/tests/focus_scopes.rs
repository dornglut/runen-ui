#![cfg(feature = "internal-test-seams")]
#![allow(refining_impl_trait)]

use runenui_core::{
    Axis, ChildLayout, ChildLayoutWidget, CommandOrigin, Element, EventContext, EventPhase,
    FocusBoundaryPolicy, FocusEventKind, FocusReason, FocusScope, FocusScopePolicy, NoHostProtocol,
    SemanticCommand, StyleTokens, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, button, column, container,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, SurfaceBuildContext,
    TraceFocusBoundaryOutcome, TraceRecordKind,
};

#[derive(Clone, Copy)]
enum TreeShape {
    Root,
    Nested(FocusBoundaryPolicy),
    Empty(FocusBoundaryPolicy),
    Remembering,
}

struct State {
    order: Vec<&'static str>,
    shape: TreeShape,
    disabled: Option<&'static str>,
    hidden: Option<&'static str>,
    replacement: bool,
}

enum Action {
    ReplaceRemembered,
    RemoveNestedScope,
}

struct App;

type Rect = (f32, f32, f32, f32);
type NamedRect<'a> = (&'a str, Rect);

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> Element<Action> {
        match state.shape {
            TreeShape::Root => column(
                state
                    .order
                    .iter()
                    .map(|name| leaf(state, name))
                    .collect::<Vec<_>>(),
            )
            .key("root")
            .into_element(),
            TreeShape::Nested(policy) => {
                let nested =
                    column(vec![leaf(state, "n1"), leaf(state, "o")])
                        .key("scope")
                        .into_element()
                        .focus_scope(FocusScope::new().with_policy(FocusScopePolicy::new(
                            FocusBoundaryPolicy::Delegate,
                            policy,
                        )));
                column(vec![nested, leaf(state, "p")])
                    .key("root")
                    .into_element()
            }
            TreeShape::Empty(policy) => {
                let nested = column(Vec::<Element<Action>>::new())
                    .id("scope")
                    .key("scope")
                    .into_element()
                    .focus_scope(
                        FocusScope::new().with_policy(FocusScopePolicy::new(policy, policy)),
                    );
                column(vec![nested, leaf(state, "p")])
                    .key("root")
                    .into_element()
            }
            TreeShape::Remembering => {
                let children = if state.replacement {
                    vec![leaf(state, "b"), leaf(state, "a")]
                } else {
                    vec![leaf(state, "a"), leaf(state, "b")]
                };
                let nested = column(children)
                    .id("o")
                    .key("scope")
                    .into_element()
                    .focus_scope(FocusScope::new());
                column(vec![nested, leaf(state, "p")])
                    .key("root")
                    .into_element()
            }
        }
    }

    fn update(state: &mut State, action: Action) {
        match action {
            Action::ReplaceRemembered => state.replacement = true,
            Action::RemoveNestedScope => {
                state.shape = TreeShape::Root;
                state.order = vec!["p"];
            }
        }
    }
}

fn leaf(state: &State, name: &'static str) -> Element<Action> {
    let key = if name == "a" && state.replacement {
        "a-replacement"
    } else {
        name
    };
    let mut control = button(name)
        .id(name)
        .key(key)
        .on_activate(|| Action::ReplaceRemembered);
    if state.disabled == Some(name) {
        control = control.disabled();
    }
    let mut element = control.into_element();
    if state.hidden == Some(name) {
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
        .node_by_authored_id(&authored)
        .unwrap_or_else(|| unreachable!("named corpus node is mounted"))
        .id()
        .clone()
}

fn publish_geometry(runtime: &mut AppRuntime<App>, geometry: &[NamedRect<'_>]) {
    let tokens = StyleTokens::new();
    let _ = runtime.publish_surface(&SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::unbounded(),
    ));
    let issued = geometry
        .iter()
        .map(|(name, (x, y, width, height))| (id(runtime, name), [*x, *y, *width, *height]))
        .collect::<Vec<_>>();
    runtime.__replace_current_focus_geometry_for_test(&issued);
}

fn command(runtime: &mut AppRuntime<App>, target: MountedNodeId, command: SemanticCommand) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("live corpus command is accepted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

#[derive(Clone, Copy, Debug)]
struct Vector {
    name: &'static str,
    order: &'static [&'static str],
    geometry: &'static [NamedRect<'static>],
    origin: &'static str,
    command: SemanticCommand,
    expected: Option<&'static str>,
    disabled: Option<&'static str>,
    hidden: Option<&'static str>,
}

#[test]
#[allow(clippy::too_many_lines)]
fn df_01_through_df_09_and_df_13_df_14_df_17_through_df_20_use_public_commands() {
    const VECTORS: &[Vector] = &[
        Vector {
            name: "DF-01",
            order: &["o", "a"],
            geometry: &[("o", (0., 0., 10., 10.)), ("a", (20., 0., 10., 10.))],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: Some("a"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-02",
            order: &["a", "o"],
            geometry: &[("o", (20., 0., 10., 10.)), ("a", (0., 0., 10., 10.))],
            origin: "o",
            command: SemanticCommand::FocusLeft,
            expected: Some("a"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-03",
            order: &["a", "o"],
            geometry: &[("o", (0., 20., 10., 10.)), ("a", (0., 0., 10., 10.))],
            origin: "o",
            command: SemanticCommand::FocusUp,
            expected: Some("a"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-04",
            order: &["o", "a"],
            geometry: &[("o", (0., 0., 10., 10.)), ("a", (0., 20., 10., 10.))],
            origin: "o",
            command: SemanticCommand::FocusDown,
            expected: Some("a"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-05",
            order: &["o", "b", "a"],
            geometry: &[
                ("o", (0., 0., 10., 10.)),
                ("a", (30., 0., 10., 10.)),
                ("b", (12., 20., 10., 10.)),
            ],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: Some("a"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-06",
            order: &["o", "b", "a"],
            geometry: &[
                ("o", (0., 0., 20., 20.)),
                ("a", (20., 15., 10., 10.)),
                ("b", (20., 25., 10., 10.)),
            ],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: Some("a"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-07",
            order: &["o", "b", "a"],
            geometry: &[
                ("o", (0., 0., 10., 40.)),
                ("a", (20., 10., 30., 5.)),
                ("b", (15., 50., 10., 10.)),
            ],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: Some("a"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-08",
            order: &["o", "b", "a"],
            geometry: &[
                ("o", (0., 0., 10., 10.)),
                ("a", (5., 0., 10., 10.)),
                ("b", (20., 0., 10., 10.)),
            ],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: Some("a"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-09",
            order: &["o", "b", "a"],
            geometry: &[
                ("o", (0., 10., 10., 10.)),
                ("a", (20., 0., 10., 10.)),
                ("b", (20., 20., 10., 10.)),
            ],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: Some("b"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-13",
            order: &["o", "a", "b"],
            geometry: &[
                ("o", (0., 0., 10., 10.)),
                ("a", (20., 0., 10., 10.)),
                ("b", (40., 0., 10., 10.)),
            ],
            origin: "b",
            command: SemanticCommand::FocusNext,
            expected: Some("o"),
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-14",
            order: &["a", "o"],
            geometry: &[("o", (20., 0., 10., 10.)), ("a", (0., 0., 10., 10.))],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: None,
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-17",
            order: &["o", "a", "b"],
            geometry: &[
                ("o", (0., 0., 10., 10.)),
                ("a", (12., 0., 10., 10.)),
                ("b", (30., 0., 10., 10.)),
            ],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: Some("b"),
            disabled: Some("a"),
            hidden: None,
        },
        Vector {
            name: "DF-18",
            order: &["o", "a", "b"],
            geometry: &[
                ("o", (0., 0., 10., 10.)),
                ("a", (12., 0., 10., 10.)),
                ("b", (30., 0., 10., 10.)),
            ],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: Some("b"),
            disabled: None,
            hidden: Some("a"),
        },
        Vector {
            name: "DF-19",
            order: &["a", "b", "c", "o"],
            geometry: &[
                ("o", (20., 20., 10., 10.)),
                ("a", (0., 20., 10., 10.)),
                ("b", (20., 0., 10., 10.)),
                ("c", (0., 0., 10., 10.)),
            ],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: None,
            disabled: None,
            hidden: None,
        },
        Vector {
            name: "DF-20",
            order: &["o", "b", "a"],
            geometry: &[
                ("o", (0., 0., 10., 10.)),
                ("a", (10., 0., 10., 10.)),
                ("b", (20., 0., 10., 10.)),
            ],
            origin: "o",
            command: SemanticCommand::FocusRight,
            expected: Some("a"),
            disabled: None,
            hidden: None,
        },
    ];

    for vector in VECTORS {
        let mut runtime = AppRuntime::<App>::mount(State {
            order: vector.order.to_vec(),
            shape: TreeShape::Root,
            disabled: vector.disabled,
            hidden: vector.hidden,
            replacement: false,
        });
        settle(&mut runtime);
        publish_geometry(&mut runtime, vector.geometry);
        let origin = id(&mut runtime, vector.origin);
        command(&mut runtime, origin.clone(), SemanticCommand::RequestFocus);
        command(&mut runtime, origin.clone(), vector.command);
        let expected = vector.expected.map(|name| id(&mut runtime, name));
        assert_eq!(
            runtime.focus().focused_node(),
            expected.as_ref().or(Some(&origin)),
            "{} selected the wrong exact mounted target",
            vector.name
        );
        if expected.is_some() {
            let reason = if matches!(
                vector.command,
                SemanticCommand::FocusNext | SemanticCommand::FocusPrevious
            ) {
                FocusReason::LinearNavigation
            } else {
                FocusReason::DirectionalNavigation
            };
            assert_eq!(runtime.focus().reason(), Some(reason), "{}", vector.name);
        }
        assert!(
            runtime.trace().records().any(|record| matches!(
                record.kind(),
                TraceRecordKind::FocusCommandEvaluated { command, .. } if *command == vector.command
            )),
            "{} retained no causal focus-command trace",
            vector.name
        );
    }
}

#[test]
fn df_10_df_11_and_df_12_enforce_nested_scope_boundaries() {
    let cases = [
        (
            "DF-10",
            FocusBoundaryPolicy::Delegate,
            (0., 0., 10., 10.),
            (20., 0., 10., 10.),
            Some("p"),
        ),
        (
            "DF-11",
            FocusBoundaryPolicy::Trap,
            (0., 0., 10., 10.),
            (20., 0., 10., 10.),
            None,
        ),
        (
            "DF-12",
            FocusBoundaryPolicy::Wrap,
            (20., 0., 10., 10.),
            (40., 0., 10., 10.),
            Some("n1"),
        ),
    ];
    for (name, policy, origin_rect, parent_rect, expected) in cases {
        let mut runtime = AppRuntime::<App>::mount(State {
            order: Vec::new(),
            shape: TreeShape::Nested(policy),
            disabled: None,
            hidden: None,
            replacement: false,
        });
        settle(&mut runtime);
        let n1_rect = if name == "DF-12" {
            (0., 0., 10., 10.)
        } else {
            (-20., 0., 10., 10.)
        };
        publish_geometry(
            &mut runtime,
            &[("n1", n1_rect), ("o", origin_rect), ("p", parent_rect)],
        );
        let origin = id(&mut runtime, "o");
        command(&mut runtime, origin.clone(), SemanticCommand::RequestFocus);
        command(&mut runtime, origin.clone(), SemanticCommand::FocusRight);
        let expected = expected.map(|expected| id(&mut runtime, expected));
        assert_eq!(
            runtime.focus().focused_node(),
            expected.as_ref().or(Some(&origin)),
            "{name}"
        );
        if expected.is_some() {
            assert_eq!(
                runtime.focus().reason(),
                Some(FocusReason::DirectionalNavigation),
                "{name}"
            );
        }
    }
}

#[test]
fn empty_scope_has_no_candidate_and_cannot_escape_or_wrap() {
    for policy in [FocusBoundaryPolicy::Trap, FocusBoundaryPolicy::Stop] {
        let mut runtime = AppRuntime::<App>::mount(State {
            order: Vec::new(),
            shape: TreeShape::Empty(policy),
            disabled: None,
            hidden: None,
            replacement: false,
        });
        settle(&mut runtime);
        let scope = id(&mut runtime, "scope");
        command(&mut runtime, scope, SemanticCommand::FocusNext);
        assert_eq!(runtime.focus().focused_node(), None);
        assert!(runtime.trace().records().any(|record| matches!(
            (policy, record.kind()),
            (
                FocusBoundaryPolicy::Trap,
                TraceRecordKind::FocusCandidateSelected {
                    outcome: TraceFocusBoundaryOutcome::Trapped,
                }
            ) | (
                FocusBoundaryPolicy::Stop,
                TraceRecordKind::FocusCandidateSelected {
                    outcome: TraceFocusBoundaryOutcome::Stopped,
                }
            )
        )));
    }
}

#[test]
fn removed_scope_clears_exact_focus_and_suppresses_stale_delivery() {
    let mut runtime = AppRuntime::<App>::mount(State {
        order: Vec::new(),
        shape: TreeShape::Nested(FocusBoundaryPolicy::Delegate),
        disabled: None,
        hidden: None,
        replacement: false,
    });
    settle(&mut runtime);
    let focused = id(&mut runtime, "o");
    command(&mut runtime, focused.clone(), SemanticCommand::RequestFocus);
    runtime
        .submit_action(Action::RemoveNestedScope)
        .unwrap_or_else(|_| unreachable!("scope removal action is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(runtime.focus().reason(), Some(FocusReason::Removal));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::FocusTransitionCommitted {
            reason: FocusReason::Removal,
            old_target: Some(old),
            new_target: None,
        } if old == &focused
    )));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::FocusNotificationSuppressed { .. }
    )));
}

#[test]
fn df_15_and_df_16_restore_only_exact_live_remembered_generations() {
    let mut runtime = AppRuntime::<App>::mount(State {
        order: Vec::new(),
        shape: TreeShape::Remembering,
        disabled: None,
        hidden: None,
        replacement: false,
    });
    settle(&mut runtime);
    publish_geometry(
        &mut runtime,
        &[
            ("o", (0., 0., 10., 10.)),
            ("a", (20., 0., 10., 10.)),
            ("b", (40., 0., 10., 10.)),
            ("p", (60., 0., 10., 10.)),
        ],
    );
    let scope = id(&mut runtime, "o");
    let a = id(&mut runtime, "a");
    let p = id(&mut runtime, "p");
    command(&mut runtime, a.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, p.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, scope.clone(), SemanticCommand::RestoreFocus);
    assert_eq!(runtime.focus().focused_node(), Some(&a), "DF-15");
    assert_eq!(
        runtime.focus().reason(),
        Some(FocusReason::RememberedRestoration)
    );

    command(&mut runtime, p, SemanticCommand::RequestFocus);
    runtime
        .submit_action(Action::ReplaceRemembered)
        .unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    assert_ne!(
        id(&mut runtime, "a"),
        a,
        "DF-16 must replace the generation"
    );
    publish_geometry(
        &mut runtime,
        &[
            ("o", (0., 0., 10., 10.)),
            ("b", (20., 0., 10., 10.)),
            ("a", (40., 0., 10., 10.)),
            ("p", (60., 0., 10., 10.)),
        ],
    );
    command(&mut runtime, scope, SemanticCommand::RestoreFocus);
    let b = id(&mut runtime, "b");
    assert_eq!(runtime.focus().focused_node(), Some(&b), "DF-16");
}

#[test]
fn logical_scroll_boundary_delegates_through_the_canonical_command_queue() {
    let mut runtime = AppRuntime::<App>::mount(State {
        order: Vec::new(),
        shape: TreeShape::Nested(FocusBoundaryPolicy::LogicalScroll),
        disabled: None,
        hidden: None,
        replacement: false,
    });
    settle(&mut runtime);
    publish_geometry(
        &mut runtime,
        &[
            ("n1", (-20., 0., 10., 10.)),
            ("o", (0., 0., 10., 10.)),
            ("p", (20., 0., 10., 10.)),
        ],
    );
    let origin = id(&mut runtime, "o");
    command(&mut runtime, origin.clone(), SemanticCommand::RequestFocus);
    runtime
        .submit_command(
            origin.clone(),
            SemanticCommand::FocusRight,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("directional request is accepted"));
    let report = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.focus().focused_node(), Some(&origin));
    assert_eq!(report.remaining_queued_envelopes(), 1);
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::FocusCandidateSelected {
            outcome: TraceFocusBoundaryOutcome::LogicalScroll,
        }
    )));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrderingFact {
    Notification(FocusEventKind, &'static str, EventPhase),
    InitiatingOutput,
}

#[derive(Debug)]
struct OrderingWidget {
    name: &'static str,
    focusable: bool,
}

impl Widget<OrderingFact> for OrderingWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.focusable)
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, OrderingFact>,
    ) -> WidgetEventOutput {
        if let Some(focus) = event.as_focus() {
            context.emit(OrderingFact::Notification(
                focus.kind(),
                self.name,
                context.phase(),
            ));
        } else if event.as_semantic_command().is_some_and(|command| {
            command.command() == SemanticCommand::RequestFocus
                && context.phase() == EventPhase::Target
        }) {
            context.emit(OrderingFact::InitiatingOutput);
        }
        WidgetEventOutput::none()
    }
}

impl ChildLayoutWidget<OrderingFact> for OrderingWidget {
    fn child_layout(&self, (): &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Horizontal,
        }
    }
}

struct OrderingApp;

impl UiApp for OrderingApp {
    type State = Vec<OrderingFact>;
    type Action = OrderingFact;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        container(
            OrderingWidget {
                name: "root",
                focusable: false,
            },
            vec![
                Element::new(OrderingWidget {
                    name: "a",
                    focusable: true,
                })
                .id("order.a")
                .key("a")
                .focusable(true),
                Element::new(OrderingWidget {
                    name: "b",
                    focusable: true,
                })
                .id("order.b")
                .key("b")
                .focusable(true),
            ],
        )
        .key("root")
        .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state.push(action);
    }
}

#[test]
fn focus_notification_outputs_precede_the_initiating_command_output() {
    let mut runtime = AppRuntime::<OrderingApp>::mount(Vec::new());
    let a = runtime.index().nodes()[1].id().clone();
    let b = runtime.index().nodes()[2].id().clone();
    for target in [a, b] {
        runtime
            .submit_command(
                target,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("ordering target is live"));
        runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
        runtime.pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ));
    }
    assert_eq!(
        &runtime.state()[4..],
        &[
            OrderingFact::Notification(FocusEventKind::Out, "root", EventPhase::Capture),
            OrderingFact::Notification(FocusEventKind::Out, "a", EventPhase::Target),
            OrderingFact::Notification(FocusEventKind::Out, "root", EventPhase::Bubble),
            OrderingFact::Notification(FocusEventKind::In, "root", EventPhase::Capture),
            OrderingFact::Notification(FocusEventKind::In, "b", EventPhase::Target),
            OrderingFact::Notification(FocusEventKind::In, "root", EventPhase::Bubble),
            OrderingFact::InitiatingOutput,
        ]
    );
}
