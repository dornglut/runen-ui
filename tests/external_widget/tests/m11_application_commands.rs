#![allow(refining_impl_trait)]

use runenui_core::{
    ApplicationCommandDisposition, ApplicationCommandId, ChildBearingWidget, CommandOrigin,
    Element, EventContext, EventPhase, NoHostProtocol, SemanticCommand, SemanticCommandEvent,
    UiApp, UiEvent, View, Widget, WidgetEventOutput, children, container,
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
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let target = runtime.index().nodes()[1].id().clone();

    runtime
        .submit_command(
            target,
            SemanticCommand::OpenMenu,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("live target is accepted"));

    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state(), &0, "emission is non-reentrant");

    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state(), &0, "resolved action remains queued");

    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
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
