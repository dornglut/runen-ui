#![allow(refining_impl_trait)]

use runenui_core::{
    ApplicationCommand, ApplicationCommandDisposition, ApplicationCommandId, ChildBearingWidget,
    CommandOrigin, Element, EventContext, EventPhase, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, SemanticCommand, SemanticCommandEvent, ShortcutBinding, ShortcutChord,
    ShortcutRepeatPolicy, UiApp, UiEvent, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetEventOutput, button, children, container,
};
use runenui_runtime::{AppRuntime, PumpBudget, TraceApplicationCommandOutcome, TraceRecordKind};

#[derive(Debug)]
struct EmitCommand {
    command: ApplicationCommandId,
}

impl Widget<ChildAction> for EmitCommand {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ChildAction>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && matches!(
                event
                    .as_semantic_command()
                    .map(SemanticCommandEvent::command),
                Some(SemanticCommand::OpenMenu)
            )
        {
            context.emit_application_command(self.command.clone());
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        context: &mut WidgetActivationContext<ChildAction>,
    ) -> WidgetActivationOutput<ChildAction> {
        context.emit_application_command(self.command.clone());
        WidgetActivationOutput::none()
    }
}

#[derive(Debug)]
struct CommandScopeProbe {
    command: ApplicationCommandId,
}

impl Widget<Action> for CommandScopeProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Capture
            && event
                .as_application_command()
                .is_some_and(|event| event.command() == &self.command)
        {
            context.consume_application_command(ApplicationCommandDisposition::Resolved);
            context.emit(Action::Handled);
        }
        WidgetEventOutput::none()
    }
}

impl ChildBearingWidget<Action> for CommandScopeProbe {}

#[derive(Debug)]
enum ChildAction {}

#[derive(Debug)]
enum Action {
    Handled,
    Child(ChildAction),
}

struct App;

impl UiApp for App {
    type State = usize;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        let command = ApplicationCommandId::from_static("document.save")
            .unwrap_or_else(|_| unreachable!("static command identity is valid"));
        container(
            CommandScopeProbe {
                command: command.clone(),
            },
            children![
                Element::new(EmitCommand { command })
                    .id("command.target")
                    .key("command-target")
                    .map_action(Action::Child)
            ],
        )
        .key("command-scope")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Handled => *state += 1,
            Action::Child(never) => match never {},
        }
    }
}

#[test]
fn downstream_widgets_emit_and_resolve_application_commands_through_public_contracts() {
    let mut runtime = AppRuntime::<App>::mount(0);
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
    let target = runtime.index().nodes()[1].id().clone();

    runtime
        .submit_command(
            target,
            SemanticCommand::OpenMenu,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("live target is accepted"));

    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &0, "emission is non-reentrant");

    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &0, "resolved action remains queued");

    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &1);

    assert!(runtime.trace().records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::ApplicationCommandResolution {
                outcome: TraceApplicationCommandOutcome::Resolved
            }
        )
    }));
}

#[test]
fn downstream_mapped_activation_emits_the_same_scoped_application_command() {
    let mut runtime = AppRuntime::<App>::mount(0);
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
    let authored = runenui_core::ElementId::from_static("command.target")
        .unwrap_or_else(|_| unreachable!("static authored id is valid"));
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("command target is mounted"))
        .id()
        .clone();

    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("live activation target is accepted"));

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
    assert_eq!(runtime.state(), &1);
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ApplicationCommandResolution {
            outcome: TraceApplicationCommandOutcome::Resolved
        }
    )));
}

#[derive(Debug)]
struct DownstreamShortcutScope {
    bindings: Vec<ShortcutBinding>,
}

impl Widget<ChildAction> for DownstreamShortcutScope {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ChildAction>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Bubble && matches!(event, UiEvent::Keyboard(_)) {
            self.bindings.clear();
        }
        WidgetEventOutput::none()
    }

    fn shortcuts(&self) -> &[ShortcutBinding] {
        self.bindings.as_slice()
    }
}

impl ChildBearingWidget<ChildAction> for DownstreamShortcutScope {}

struct ShortcutApp;

impl UiApp for ShortcutApp {
    type State = usize;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        let command = ApplicationCommandId::from_static("document.save")
            .unwrap_or_else(|_| unreachable!("static command identity is valid"));
        let downstream = container(
            DownstreamShortcutScope {
                bindings: vec![ShortcutBinding::new(
                    ShortcutChord::logical(
                        LogicalKey::Character(String::from("s")),
                        KeyModifiers::NONE.with_control(),
                    ),
                    ShortcutRepeatPolicy::IgnoreRepeat,
                    ApplicationCommand::new(command.clone(), true),
                )],
            },
            [button("shortcut target")
                .id("shortcut.target")
                .key("shortcut-target")
                .into_element()
                .focusable(true)],
        )
        .key("downstream-shortcut-scope")
        .into_element()
        .map_action(Action::Child);

        container(CommandScopeProbe { command }, [downstream]).key("downstream-command-scope")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Handled => *state += 1,
            Action::Child(never) => match never {},
        }
    }
}

#[test]
fn downstream_custom_widget_publishes_shortcuts_without_builtin_type_knowledge() {
    let mut runtime = AppRuntime::<ShortcutApp>::mount(0);
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
    let authored = runenui_core::ElementId::from_static("shortcut.target")
        .unwrap_or_else(|_| unreachable!("static authored id is valid"));
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("shortcut target is mounted"))
        .id()
        .clone();

    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("shortcut target focus is accepted"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyX")),
            LogicalKey::Character(String::from("x")),
            KeyModifiers::NONE.with_control(),
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("unmatched downstream key is accepted"));
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
    assert_eq!(
        runtime.state(),
        &0,
        "unmatched key mutates only the live callback object and triggers no rebuild"
    );

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyS")),
            LogicalKey::Character(String::from("s")),
            KeyModifiers::NONE.with_control(),
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("matching downstream shortcut key is accepted"));
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

    assert_eq!(
        runtime.state(),
        &1,
        "callback-local widget mutation cannot replace reconciliation-authored shortcut facts"
    );
    assert!(
        runtime
            .trace()
            .records()
            .any(|record| matches!(record.kind(), TraceRecordKind::KeyboardShortcutMatched))
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ApplicationCommandResolution {
            outcome: TraceApplicationCommandOutcome::Resolved
        }
    )));
}
