//! Event-time text ownership and post-event capability are independent facts.
use runenui_core::{
    CommandOrigin, Element, EventContext, EventPhase, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetEventOutput,
    WidgetInvalidation, WidgetTextInput,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, PumpBudget, RuntimeConfig, TraceConfig, UiInputClaimReason,
    UiInputConflict, UiInputFinality,
};
struct App;
impl UiApp for App {
    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn root(initial_text: &bool) -> impl View<()> {
        Element::new(WithdrawText(*initial_text)).focusable(true)
    }
    fn update(_: &mut bool, (): ()) {
        unreachable!("no application actions")
    }
}
#[derive(Debug)]
struct WithdrawText(bool);
impl Widget<()> for WithdrawText {
    type State = bool;
    fn create_state(&self) -> bool {
        self.0
    }
    fn text_input(&self, active: &bool) -> WidgetTextInput {
        WidgetTextInput::new(*active, *active)
    }
    fn event(
        &mut self,
        active: &mut bool,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target && matches!(event, UiEvent::Keyboard(_)) && *active
        {
            *active = false;
            context.invalidate(WidgetInvalidation::INTERACTION);
            return WidgetEventOutput::changed();
        }
        WidgetEventOutput::none()
    }
}
#[test]
fn keyboard_text_owner_claim_survives_capability_withdrawal_in_its_own_callback() {
    for capacity in [0, 1, 512] {
        for initial_text in [false, true] {
            let mut runtime = AppRuntime::<App>::mount_with_config(
                initial_text,
                RuntimeConfig::default().with_trace_config(TraceConfig::new(capacity)),
            );
            let full = PumpBudget::new(64, 64, 64, 64);
            runtime.pump(full).unwrap_or_else(|_| unreachable!());
            let target = runtime.index().nodes()[0].id().clone();
            runtime
                .submit_command(
                    target,
                    SemanticCommand::RequestFocus,
                    CommandOrigin::programmatic(),
                )
                .unwrap_or_else(|_| unreachable!());
            runtime.pump(full).unwrap_or_else(|_| unreachable!());
            let before = runtime.input_ownership().unwrap_or_else(|_| unreachable!());
            assert_eq!(
                before
                    .keyboard()
                    .text_input_capability()
                    .accepts_committed_text(),
                initial_text
            );
            let receipt = runtime
                .submit_keyboard(KeyboardEvent::new(
                    KeyboardPhase::Down,
                    PhysicalKey::Code("Fixture".into()),
                    LogicalKey::Character("x".into()),
                    KeyModifiers::NONE,
                    false,
                    KeyLocation::Standard,
                    KeyboardCompositionState::Inactive,
                    None,
                ))
                .unwrap_or_else(|_| unreachable!());
            let batch = runtime
                .pump(PumpBudget::new(1, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!());
            let input = batch
                .ordered_records()
                .iter()
                .find_map(|record| match record {
                    InputArbitrationRecord::InputSettled(input) => Some(input),
                    _ => None,
                })
                .unwrap_or_else(|| unreachable!());
            assert_eq!(input.sequence(), receipt.sequence());
            let UiInputFinality::Committed(facts) = input.finality() else {
                unreachable!()
            };
            assert_eq!(
                facts.conflict(),
                if initial_text {
                    UiInputConflict::ExclusiveUi
                } else {
                    UiInputConflict::ObservedNonexclusive
                }
            );
            assert_eq!(
                facts.reasons(),
                if initial_text {
                    &[UiInputClaimReason::TextOwner][..]
                } else {
                    &[]
                }
            );
            assert!(
                !batch
                    .final_ownership()
                    .keyboard()
                    .text_input_capability()
                    .accepts_committed_text()
            );
            if initial_text {
                assert!(
                    matches!(batch.ordered_records(), [InputArbitrationRecord::OwnershipChanged(change), InputArbitrationRecord::InputSettled(input)]
                    if change.before_revision() == before.revision() && change.after().revision() == input.ownership_revision())
                );
            } else {
                assert_eq!(batch.ordered_records().len(), 1);
            }
            assert_eq!(
                runtime
                    .pump(full)
                    .unwrap_or_else(|_| unreachable!())
                    .ordered_records(),
                []
            );
        }
    }
}
