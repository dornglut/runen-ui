#![allow(refining_impl_trait)]

//! Public end-to-end eligibility and per-route preventDefault conformance.

use runenui_core::{
    EventContext, EventPhase, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, LogicalKey, Menu, MenuButton, NoHostProtocol, PhysicalKey,
    PresentationFocusPolicy, PresentationOutsidePointerPolicy, SemanticCommand, StyleEnvironment,
    SurfacePresentation, SurfacePresentationPlacement, SurfacePresentationSide, UiApp, UiEvent,
    View, Widget, WidgetEventOutput, WidgetMeasure, WidgetMeasureInput, column,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TraceRecordKind};

const FULL: PumpBudget = PumpBudget::new(128, 128, 128, 128);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);

#[derive(Clone, Copy, Debug)]
enum Action {
    DisableCancel,
    Dismiss,
}

#[derive(Clone, Debug)]
struct Model {
    cancel_enabled: bool,
    prevent_keyboard: bool,
    prevent_semantic: bool,
    dismissals: usize,
}

#[derive(Debug)]
struct Probe {
    prevent_keyboard: bool,
    prevent_semantic: bool,
}

impl Widget<Action> for Probe {
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

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target {
            if self.prevent_keyboard
                && event
                    .as_keyboard()
                    .is_some_and(|keyboard| keyboard.logical_key() == &LogicalKey::Escape)
            {
                context.prevent_default();
            }
            if self.prevent_semantic
                && event
                    .as_semantic_command()
                    .is_some_and(|command| command.command() == SemanticCommand::CancelOrBack)
            {
                context.prevent_default();
            }
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            runenui_core::LogicalLength::from(40_u8),
            runenui_core::LogicalLength::from(20_u8),
        )
    }
}

struct App;

impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(4, 0)
            .with_application_outputs(0)
            .with_reconciliation(4, 0, 4)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        let (retained_updates, dirty_owners) = match action {
            Action::DisableCancel => (1, 1),
            Action::Dismiss => (0, 0),
        };
        runenui_core::ProducerBudget::new(dirty_owners.max(1), 0)
            .with_application_outputs(0)
            .with_reconciliation(0, retained_updates, dirty_owners)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        let outputs = 4 * 4 + 8;
        runenui_core::ProducerBudget::new(outputs, outputs)
            .with_application_outputs(0)
            .with_application_outputs(1)
            .with_reconciliation(0, 1, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    type State = Model;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Model) -> impl View<Action> {
        let presentation = SurfacePresentation::new(SurfacePresentationPlacement::new(
            SurfacePresentationSide::Bottom,
        ))
        .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock)
        .dismiss_on_cancel_or_back(state.cancel_enabled)
        .with_focus_policy(PresentationFocusPolicy::EnterAndRestore);
        let menu = Menu::new(vec![
            runenui_core::Element::new(Probe {
                prevent_keyboard: state.prevent_keyboard,
                prevent_semantic: state.prevent_semantic,
            })
            .id("focus-probe")
            .focusable(true)
            .presentation_focus_preferred(true),
        ])
        .id("presentation")
        .surface_presentation(presentation)
        .on_dismiss(|_| Action::Dismiss);
        column(vec![
            MenuButton::new("Trigger", true)
                .id("trigger")
                .with_submenu(menu, true)
                .into_element(),
        ])
    }

    fn update(state: &mut Model, action: Action) {
        match action {
            Action::DisableCancel => state.cancel_enabled = false,
            Action::Dismiss => state.dismissals += 1,
        }
    }
}

const fn escape() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Escape,
        LogicalKey::Escape,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

const fn initial() -> Model {
    Model {
        cancel_enabled: true,
        prevent_keyboard: false,
        prevent_semantic: false,
        dismissals: 0,
    }
}

fn published(runtime: &mut AppRuntime<App>, context: &SurfaceBuildContext<'_>) {
    runtime
        .publish_surface(context)
        .unwrap_or_else(|_| unreachable!("presentation publication"));
}

fn drain(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("full drain"))
            .report()
            .is_quiescent()
    );
}

fn one(runtime: &mut AppRuntime<App>) {
    assert_eq!(
        runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("one envelope"))
            .report()
            .processed_envelopes(),
        1
    );
}

#[test]
fn c7_changed_presentation_cancel_eligibility_suppresses_bound_default() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<App>::mount(initial())
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    published(&mut runtime, &context);
    drain(&mut runtime);
    assert!(runtime.focus().focused_node().is_some());
    runtime
        .submit_keyboard(escape())
        .unwrap_or_else(|_| unreachable!("eligible-origin Escape"));
    runtime
        .submit_action(Action::DisableCancel)
        .unwrap_or_else(|_| unreachable!("eligibility changed behind Escape"));
    one(&mut runtime);
    one(&mut runtime);
    published(&mut runtime, &context);
    drain(&mut runtime);
    assert!(!runtime.state().cancel_enabled);
    assert_eq!(runtime.state().dismissals, 0);
}

#[test]
fn c8_keyboard_or_semantic_prevention_suppresses_presentation_dismissal() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    for prevent_keyboard in [true, false] {
        let mut state = initial();
        state.prevent_keyboard = prevent_keyboard;
        state.prevent_semantic = !prevent_keyboard;
        let mut runtime = AppRuntime::<App>::mount(state).unwrap_or_else(|error| {
            unreachable!("fixture initial admission must succeed: {error}")
        });
        published(&mut runtime, &context);
        drain(&mut runtime);
        assert!(runtime.focus().focused_node().is_some());
        let commands_before = runtime
            .trace()
            .records()
            .filter(|record| matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted))
            .count();
        runtime
            .submit_keyboard(escape())
            .unwrap_or_else(|_| unreachable!("preventable Escape"));
        drain(&mut runtime);
        assert_eq!(runtime.state().dismissals, 0);
        let commands_after = runtime
            .trace()
            .records()
            .filter(|record| matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted))
            .count();
        if prevent_keyboard {
            assert_eq!(
                commands_after, commands_before,
                "keyboard prevent_default must suppress command derivation"
            );
        } else {
            assert!(
                commands_after > commands_before,
                "semantic prevent_default must suppress the routed command, not its derivation"
            );
        }
    }
}
