#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;
use runenui_core::{
    ApplicationCommand, ApplicationCommandId, CommandBinding, CommandScope, Element, EventContext,
    EventPhase, Focusability, NoHostProtocol, SemanticAction, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticNodeContribution, SemanticRole,
    UiApp, UiEvent, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetEventOutput, button, toolbar,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug)]
enum Action {
    Saved,
    Opened,
}

#[derive(Debug)]
struct CommandButton(ApplicationCommandId);

impl Widget<Action> for CommandButton {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        _: &mut Self::State,
        context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        context.emit_application_command(self.0.clone());
        WidgetActivationOutput::none()
    }

    fn event(
        &mut self,
        _: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && event
                .as_semantic_command()
                .is_some_and(|event| event.command() == SemanticCommand::OpenMenu)
        {
            context.emit_application_command(self.0.clone());
        }
        WidgetEventOutput::none()
    }

    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name("Custom Save")
                .with_action(SemanticAction::RequestFocus)
                .with_action(SemanticAction::Activate),
        )
    }
}

fn id() -> ApplicationCommandId {
    ApplicationCommandId::new("toolbar.save")
        .unwrap_or_else(|_| unreachable!("fixture command valid"))
}

struct App;
impl UiApp for App {
    type State = (u32, u32);
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        CommandScope::new(
            [CommandBinding::new(ApplicationCommand::new(id(), true), || Action::Saved)],
            [
                toolbar([
                    button("Open")
                        .id("external.open")
                        .on_activate(|| Action::Opened)
                        .into_element(),
                    Element::new(CommandButton(id()))
                        .with_focusability(Focusability::Focusable)
                        .id("external.save")
                        .into_element(),
                ])
                .accessible_name("External editor tools")
                .id("external.toolbar")
                .into_element(),
            ],
        )
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Saved => state.0 += 1,
            Action::Opened => state.1 += 1,
        }
    }
}

fn settle(h: &mut TestHarness<App>) {
    assert_eq!(
        h.run_until_idle(SettleBudget::new(
            NonZeroUsize::new(12).unwrap_or(NonZeroUsize::MIN),
            PumpBudget::new(64, 64, 64, 64),
        ))
        .outcome(),
        SettleOutcome::Idle
    );
}

fn command(h: &mut TestHarness<App>, id: &str, command: SemanticCommand) {
    h.submit_automation_command(
        runenui_core::ElementId::new(id).unwrap_or_else(|_| unreachable!("fixture id")),
        command,
    )
    .unwrap_or_else(|error| unreachable!("command admitted: {error:?}"));
    settle(h);
    assert!(h.publish().is_ok());
}

#[test]
fn external_widget_in_toolbar_uses_public_focus_and_command_scope() {
    let mut h = TestHarness::<App>::mount((0, 0));
    assert!(h.publish().is_ok());
    let semantic = h.semantic_snapshot().unwrap();
    let bar = semantic.nodes().iter()
        .find(|n| n.role() == SemanticRole::Toolbar)
        .unwrap_or_else(|| unreachable!("toolbar semantic node"));
    assert_eq!(bar.name(), Some("External editor tools"));
    assert_eq!(h.state(), &(0, 0));

    command(&mut h, "external.open", SemanticCommand::RequestFocus);
    command(&mut h, "external.open", SemanticCommand::FocusRight);
    let focused = h.semantic_snapshot().unwrap().focused().cloned();
    assert_eq!(
        focused,
        h.semantic_snapshot().unwrap().nodes().iter()
            .find(|n| n.name() == Some("Custom Save"))
            .map(|node| node.id().clone())
    );
    assert_eq!(h.state(), &(0, 0), "movement cannot execute the command");
    command(&mut h, "external.save", SemanticCommand::Activate);
    assert_eq!(h.state(), &(1, 0), "activation resolved through ordinary scope");
    command(&mut h, "external.open", SemanticCommand::Activate);
    assert_eq!(h.state(), &(1, 1));
}
