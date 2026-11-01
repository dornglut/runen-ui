#![allow(refining_impl_trait)]
//! The descendant-targeted reveal protocol is an ordinary downstream Widget
//! capability: exact mounted descendants share M4/#344's existing FIFO and
//! surface geometry rather than relying on private `ComboBox` implementation.

use runenui_core::{
    ChildBearingWidget, CommandOrigin, ElementId, EventContext, EventPhase, Focusability,
    LayoutBound, LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength, NoHostProtocol,
    OverflowPolicy, OverflowStyle, SemanticCommand, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticPopupKind, SemanticReference,
    SemanticRelationship, SemanticRelationshipKind, SemanticRole, SemanticState, UiApp, UiEvent,
    View, Widget, WidgetEventOutput, container, list_box, option_item, scroll_container,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PublishSurfaceError, PumpBudget, RuntimeConfig,
    SubmitCommandErrorKind, SurfaceBuildContext,
};

#[derive(Clone, Copy, Debug)]
enum Case {
    Live,
    Absent,
    Disabled,
    Duplicate,
}

#[derive(Debug)]
struct RevealOwner {
    target: ElementId,
}
impl Widget<()> for RevealOwner {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Event => runenui_core::WidgetProducerBudget::new(1),
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

    fn event(
        &mut self,
        &mut (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && event
                .as_semantic_command()
                .is_some_and(|event| event.command() == SemanticCommand::FocusGroupLast)
        {
            let _ = context.try_emit_scroll_into_view_descendant(self.target.clone());
            context.prevent_default();
            context.stop_propagation();
        }
        WidgetEventOutput::none()
    }
}
impl ChildBearingWidget<()> for RevealOwner {}

struct App;
impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(21, 0)
            .with_application_outputs(0)
            .with_reconciliation(21, 0, 21)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_application_outputs(0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Four Event-producing owners (custom proxy, ListBox, scrollbar, and
        // thumb) can each receive four focus notifications; the ordinary
        // owner route contributes one more output.
        runenui_core::ProducerBudget::new(17, 17)
            .with_application_outputs(0)
            .with_reconciliation(0, 21, 0)
    }

    type State = Case;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(case: &Case) -> impl View<()> {
        let options = (0..18_u8).map(|index| {
            let duplicate = matches!(case, Case::Duplicate) && index == 16;
            let authored = if duplicate { 17 } else { index };
            option_item(format!("Option {index:02}"), index == 0)
                .id(format!("downstream.option.{authored}"))
                .enabled(!(matches!(case, Case::Disabled) && index == 17))
                .combo_box_proxy()
                .with_layout(
                    LayoutStyle::default()
                        .with_width(LayoutDimension::Length(LogicalLength::from(160_u16)))
                        .with_height(LayoutDimension::Length(LogicalLength::from(24_u8))),
                )
        });
        let overflow = OverflowStyle::new(OverflowPolicy::Clip, OverflowPolicy::Scroll);
        let viewport = scroll_container(
            list_box(options).id("downstream.list").into_element(),
            overflow,
        )
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Block)
                .with_max_height(LayoutBound::Length(LogicalLength::from(96_u8)))
                .with_overflow(overflow),
        )
        .into_element()
        .with_focusability(Focusability::NotFocusable);
        let requested = if matches!(case, Case::Absent) {
            "downstream.not-mounted"
        } else {
            "downstream.option.17"
        };
        container(
            RevealOwner {
                target: ElementId::new(requested).unwrap_or_else(|_| unreachable!()),
            },
            [viewport],
        )
        .id("downstream.owner")
        .into_element()
        .focusable(true)
    }

    fn update(_: &mut Case, (): ()) {}
}

fn settle(app: &mut AppRuntime<App>) {
    assert!(
        app.pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|error| unreachable!("downstream reveal pump: {error:?}"))
            .report()
            .is_quiescent()
    );
}

fn id(app: &mut AppRuntime<App>, requested: &str) -> MountedNodeId {
    let authored = ElementId::new(requested).unwrap_or_else(|_| unreachable!());
    app.index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("expected current mounted owner"))
        .id()
        .clone()
}

#[test]
fn custom_child_bearing_widget_reveals_only_unique_enabled_exact_mounted_descendants() {
    for case in [Case::Live, Case::Absent, Case::Disabled, Case::Duplicate] {
        let mut app = AppRuntime::<App>::mount(case).unwrap_or_else(|error| {
            unreachable!("fixture initial admission must succeed: {error}")
        });
        settle(&mut app);
        let owner = id(&mut app, "downstream.owner");
        let list = id(&mut app, "downstream.list");
        let viewport = app
            .index()
            .node(&list)
            .and_then(runenui_runtime::MountedNodeRef::parent)
            .unwrap_or_else(|| unreachable!("standard scroll viewport"))
            .clone();
        let _ = app
            .publish_surface(&SurfaceBuildContext::tight(
                &runenui_core::StyleEnvironment::default(),
                LogicalSize::try_new(350.0, 320.0).unwrap_or_else(|_| unreachable!()),
            ))
            .unwrap_or_else(|error| unreachable!("mounted downstream viewport: {error:?}"));
        assert_eq!(
            app.index()
                .node(&viewport)
                .unwrap_or_else(|| unreachable!())
                .interaction()
                .scroll_offset()
                .1,
            0.0
        );
        app.submit_command(
            owner.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus current widget"));
        settle(&mut app);
        app.submit_command(
            owner.clone(),
            SemanticCommand::FocusGroupLast,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("downstream routed event"));
        settle(&mut app);
        let offset = app
            .index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1;
        assert_eq!(offset > 0.0, matches!(case, Case::Live), "{case:?}");
        assert_eq!(app.focus().focused_node(), Some(&owner), "{case:?}");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InitialMode {
    Enabled,
    DisabledOwner,
    InertOwner,
    DisabledActiveOption,
}

#[derive(Debug)]
struct InitialCompositeOwner {
    mode: InitialMode,
}

impl Widget<()> for InitialCompositeOwner {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Publication => {
                runenui_core::WidgetProducerBudget::new(0).with_publication_reveals(usize::from(
                    matches!(
                        self.mode,
                        InitialMode::Enabled | InitialMode::DisabledActiveOption
                    ),
                ))
            }
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

    fn semantics(
        &self,
        &(): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let reference = |name: &str| SemanticReference::Authored {
            element_id: ElementId::new(name).unwrap_or_else(|_| unreachable!("known authored ID")),
            semantic_key: None,
        };
        let semantic_state = match self.mode {
            InitialMode::Enabled | InitialMode::DisabledActiveOption => SemanticState::ENABLED,
            InitialMode::DisabledOwner => SemanticState::ENABLED.with_disabled(true),
            InitialMode::InertOwner => SemanticState::ENABLED.with_inert(true),
        };
        let mut owner = SemanticNodeContribution::primary(SemanticRole::ComboBox)
            .with_name("Independent downstream owner")
            .with_popup(SemanticPopupKind::ListBox)
            .with_state(semantic_state.with_expanded(true))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                reference("initial.custom.list"),
            ))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ActiveDescendant,
                reference("initial.custom.option.17"),
            ));
        if context.has_mounted_children() {
            owner = owner.with_mounted_children();
        }
        SemanticContribution::single(owner)
    }
}
impl ChildBearingWidget<()> for InitialCompositeOwner {}

struct InitialApp;
impl UiApp for InitialApp {
    fn initial_producer_budget(state: &Self::State) -> runenui_core::ProducerBudget {
        // Only the enabled active descendant publishes an initial reveal;
        // its ListBox ancestor reserves two event phases.
        let outputs = usize::from(matches!(state, InitialMode::Enabled)) * 2;
        runenui_core::ProducerBudget::new(21, outputs)
            .with_application_outputs(0)
            .with_reconciliation(21, 0, 21)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_application_outputs(0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The path reserves two ListBox route phases, four possible focus
        // notification phases for its Event budget, and the focus-scroll
        // output in both the routed allowance and the default allowance.
        // This route does not reconcile the application tree.
        runenui_core::ProducerBudget::new(3, 8)
            .with_application_outputs(0)
            .with_reconciliation(0, 0, 0)
    }

    type State = InitialMode;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(mode: &InitialMode) -> impl View<()> {
        let options = (0..18_u8).map(|index| {
            option_item(format!("Option {index:02}"), index == 17)
                .id(format!("initial.custom.option.{index}"))
                .enabled(!matches!(mode, InitialMode::DisabledActiveOption) || index != 17)
                .combo_box_proxy()
                .on_activate(|| ())
                .with_layout(
                    LayoutStyle::default()
                        .with_width(LayoutDimension::Length(LogicalLength::from(160_u16)))
                        .with_height(LayoutDimension::Length(LogicalLength::from(24_u8))),
                )
        });
        let overflow = OverflowStyle::new(OverflowPolicy::Clip, OverflowPolicy::Scroll);
        let viewport = scroll_container(
            list_box(options).id("initial.custom.list").into_element(),
            overflow,
        )
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Block)
                .with_max_height(LayoutBound::Length(LogicalLength::from(96_u8)))
                .with_overflow(overflow),
        )
        .into_element()
        .with_focusability(Focusability::NotFocusable);
        container(InitialCompositeOwner { mode: *mode }, [viewport])
            .id("initial.custom.owner")
            .into_element()
            .focusable(true)
    }

    fn update(_: &mut InitialMode, (): ()) {}
}

#[test]
fn downstream_semantic_composite_reveals_deep_initial_option_without_builtin_widget() {
    let mut app = AppRuntime::<InitialApp>::mount(InitialMode::Enabled)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    assert!(
        app.pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|error| unreachable!("initial downstream pump: {error:?}"))
            .report()
            .is_quiescent()
    );
    let list_id = ElementId::new("initial.custom.list").unwrap_or_else(|_| unreachable!());
    let list = app
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&list_id))
        .unwrap_or_else(|| unreachable!("custom ListBox mounted"))
        .id()
        .clone();
    let viewport = app
        .index()
        .node(&list)
        .and_then(runenui_runtime::MountedNodeRef::parent)
        .unwrap_or_else(|| unreachable!("bounded standard viewport"))
        .clone();
    let first = app
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(350.0, 320.0).unwrap_or_else(|_| unreachable!()),
        ))
        .unwrap_or_else(|_| unreachable!("first semantic publication"));
    let snapshot = first.semantic_publication().snapshot();
    assert!(snapshot.nodes().iter().any(|node| {
        node.role() == SemanticRole::ComboBox
            && node.relationships().iter().any(|relationship| {
                relationship.kind() == SemanticRelationshipKind::ActiveDescendant
            })
    }));
    assert_eq!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1,
        0.0,
        "post-publication reveal must enter the existing FIFO"
    );
    assert!(
        app.pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|_| unreachable!("generic reveal pump"))
            .report()
            .is_quiescent()
    );
    assert!(
        app.index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!())
            .interaction()
            .scroll_offset()
            .1
            > 0.0,
        "public downstream Widget must receive the same post-publication reveal"
    );
    assert_eq!(app.focus().focused_node(), None);
}

// Saturate only with ordinary accepted commands; no special reveal queue.
fn fill_command_fifo(app: &mut AppRuntime<InitialApp>, owner: &MountedNodeId, roots: usize) {
    for _ in 0..roots {
        app.submit_command(
            owner.clone(),
            SemanticCommand::ScrollIntoView,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("each ordinary FIFO slot must admit a command"));
    }
}

fn scroll_offset(app: &mut AppRuntime<InitialApp>, viewport: &MountedNodeId) -> f32 {
    app.index()
        .node(viewport)
        .unwrap_or_else(|| unreachable!("standard scrolling ancestor"))
        .interaction()
        .scroll_offset()
        .1
}

fn assert_full_command_fifo_rejects_reveal(
    app: &mut AppRuntime<InitialApp>,
    owner: &MountedNodeId,
) {
    let Err(rejected) = app.submit_command(
        owner.clone(),
        SemanticCommand::ScrollIntoView,
        CommandOrigin::programmatic(),
    ) else {
        unreachable!("full command FIFO rejects another reveal request")
    };
    assert_eq!(rejected.kind(), SubmitCommandErrorKind::Full);
    assert_eq!(rejected.unaccepted().target(), owner);
    assert_eq!(
        rejected.unaccepted().command(),
        SemanticCommand::ScrollIntoView
    );
}

fn drain_and_assert_reveal_not_retried(app: &mut AppRuntime<InitialApp>, viewport: &MountedNodeId) {
    assert!(
        app.pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|error| unreachable!("draining existing commands: {error:?}"))
            .report()
            .is_quiescent()
    );
    assert_eq!(
        scroll_offset(app, viewport),
        0.0,
        "draining old commands does not retry the reveal"
    );
}

#[test]
fn initial_generic_reveal_respects_full_command_fifo_without_a_private_retry() {
    // The composite's initial work must fit before the FIFO can be saturated.
    // Keep callback output allowance bounded independently of queue capacity.
    const CAPACITY: usize = 64;
    let profile = InitialApp::interaction_producer_budget(&InitialMode::Enabled);
    assert_eq!(profile.queue_credits(), 3);
    assert_eq!(profile.outputs(), 8);
    let accepted_roots = CAPACITY / profile.queue_credits();
    assert_eq!(accepted_roots * profile.queue_credits(), 63);
    let limits = runenui_runtime::RuntimeLimits::default()
        .with_waiting_envelopes(CAPACITY)
        .with_transaction_outputs(profile.outputs());
    let mut app = AppRuntime::<InitialApp>::mount_with_config(
        InitialMode::Enabled,
        RuntimeConfig::default().with_limits(limits),
    )
    .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    let initial_work = app
        .pump(PumpBudget::new(256, 256, 256, 256))
        .unwrap_or_else(|error| unreachable!("initial work: {error:?}"));
    assert!(
        initial_work.report().is_quiescent(),
        "initial work must be quiescent before saturating FIFO; status={:?}, remaining={}",
        app.status(),
        initial_work.report().remaining_queued_envelopes(),
    );
    let authored = ElementId::new("initial.custom.owner").unwrap_or_else(|_| unreachable!());
    let owner = app
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("current downstream owner"))
        .id()
        .clone();
    fill_command_fifo(&mut app, &owner, accepted_roots);
    assert_full_command_fifo_rejects_reveal(&mut app, &owner);
    let environment = runenui_core::StyleEnvironment::default();
    let frame = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(350.0, 320.0).unwrap_or_else(|_| unreachable!()),
    );
    assert_eq!(*app.state(), InitialMode::Enabled);
    let list_authored = ElementId::new("initial.custom.list").unwrap_or_else(|_| unreachable!());
    let list = app
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&list_authored))
        .unwrap_or_else(|| unreachable!("current bounded list"))
        .id()
        .clone();
    let viewport = app
        .index()
        .node(&list)
        .and_then(runenui_runtime::MountedNodeRef::parent)
        .unwrap_or_else(|| unreachable!("standard scrolling ancestor"))
        .clone();
    assert_eq!(scroll_offset(&mut app, &viewport), 0.0);
    assert_eq!(app.publish_surface(&frame), Err(PublishSurfaceError::Full));
    assert_eq!(*app.state(), InitialMode::Enabled);
    assert_eq!(
        scroll_offset(&mut app, &viewport),
        0.0,
        "rejected publication leaves scroll unchanged"
    );
    drain_and_assert_reveal_not_retried(&mut app, &viewport);
    let publication = app
        .publish_surface(&frame)
        .unwrap_or_else(|error| unreachable!("explicit publication retry: {error:?}"));
    assert!(
        publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .any(|node| node.role() == SemanticRole::ComboBox
                && node.state().expanded() == Some(true))
    );
    assert_eq!(
        scroll_offset(&mut app, &viewport),
        0.0,
        "reveal is queued rather than applied by publication"
    );
    assert!(
        app.pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|error| unreachable!("canonical reveal FIFO: {error:?}"))
            .report()
            .is_quiescent()
    );
    assert!(
        scroll_offset(&mut app, &viewport) > 0.0,
        "explicit publication queues the reveal normally"
    );
}

#[test]
fn first_publication_reveal_rejects_semantically_disabled_inert_or_ineligible_targets() {
    for mode in [
        InitialMode::DisabledOwner,
        InitialMode::InertOwner,
        InitialMode::DisabledActiveOption,
    ] {
        let mut app = AppRuntime::<InitialApp>::mount(mode).unwrap_or_else(|error| {
            unreachable!("fixture initial admission must succeed: {error}")
        });
        assert!(
            app.pump(PumpBudget::new(256, 256, 256, 256))
                .unwrap_or_else(|error| unreachable!("initial downstream work: {error:?}"))
                .report()
                .is_quiescent()
        );
        let list_authored =
            ElementId::new("initial.custom.list").unwrap_or_else(|_| unreachable!());
        let list = app
            .index()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&list_authored))
            .unwrap_or_else(|| unreachable!("bounded option list is mounted"))
            .id()
            .clone();
        let viewport = app
            .index()
            .node(&list)
            .and_then(runenui_runtime::MountedNodeRef::parent)
            .unwrap_or_else(|| unreachable!("standard scroll viewport"))
            .clone();
        let _ = app
            .publish_surface(&SurfaceBuildContext::tight(
                &runenui_core::StyleEnvironment::default(),
                LogicalSize::try_new(350.0, 320.0).unwrap_or_else(|_| unreachable!()),
            ))
            .unwrap_or_else(|error| unreachable!("semantic publication: {error:?}"));
        assert_eq!(
            app.index()
                .node(&viewport)
                .unwrap_or_else(|| unreachable!())
                .interaction()
                .scroll_offset()
                .1,
            0.0,
            "publication must not directly scroll an ineligible owner: {mode:?}"
        );
        assert!(
            app.pump(PumpBudget::new(256, 256, 256, 256))
                .unwrap_or_else(|error| unreachable!("no invalid reveal: {error:?}"))
                .report()
                .is_quiescent()
        );
        assert_eq!(
            app.index()
                .node(&viewport)
                .unwrap_or_else(|| unreachable!())
                .interaction()
                .scroll_offset()
                .1,
            0.0,
            "invalid semantic or disabled target must not emit queued scroll: {mode:?}"
        );
        assert_eq!(app.focus().focused_node(), None);
    }
}
