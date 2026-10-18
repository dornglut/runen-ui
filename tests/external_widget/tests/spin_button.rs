#![allow(refining_impl_trait)]

//! External application proof: one TextField/M10 editor, one #310 range,
//! one #311 command FIFO, no durable numeric state in `RunenUI`.
use runenui_core::{
    CommandOrigin, CommittedTextEvent, EditIntent, EditResolution, Effects, IntoUpdateOutput,
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    NoHostProtocol, PhysicalKey, SemanticAction, SemanticActionRequest, SemanticCommand,
    SemanticNumber, SemanticRole, SpinButtonRange, SpinButtonRequest, StyleEnvironment,
    TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextPosition,
    TextSelection, UiApp, UpdateOutput, View, spin_button,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LayoutConstraints, PumpBudget,
    SurfaceBuildContext,
};

fn number(value: f64) -> SemanticNumber {
    SemanticNumber::new(value).unwrap_or_else(|_| unreachable!("finite fixture number"))
}
const fn snapshot(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(
        TextDocumentId::new(324),
        TextDocumentRevision::new(revision),
    )
}

enum Action {
    Edit(Box<EditIntent>),
    Adjust(SpinButtonRequest),
    Text(String),
    ReadOnly(bool),
    Disabled(bool),
    ShowStepControls(bool),
    Remove,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent application-owned readiness and presence facts in the external fixture"
)]
struct State {
    text: String,
    revision: u64,
    read_only: bool,
    disabled: bool,
    present: bool,
    minimum: Option<f64>,
    maximum: Option<f64>,
    show_step_controls: bool,
    requests: Vec<SpinButtonRequest>,
}

impl State {
    fn value(&self) -> Option<f64> {
        self.text.parse::<f64>().ok().filter(|value| {
            value.is_finite()
                && self.minimum.is_none_or(|minimum| *value >= minimum)
                && self.maximum.is_none_or(|maximum| *value <= maximum)
        })
    }

    fn apply_request(&mut self, request: SpinButtonRequest) {
        let next = match request {
            SpinButtonRequest::AdjustBy(delta) => self.value().and_then(|current| {
                let sum = current + delta.get();
                sum.is_finite().then_some(sum)
            }),
            SpinButtonRequest::SetValue(value) => Some(value.get()),
        };
        if let Some(next) = next {
            let next = self.minimum.map_or(next, |minimum| next.max(minimum));
            let next = self.maximum.map_or(next, |maximum| next.min(maximum));
            self.text = next.to_string();
            self.revision += 1;
        }
        self.requests.push(request);
    }
}

struct NumericApp;

impl UiApp for NumericApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        if !state.present {
            return runenui_core::text("retired numeric owner").into_element();
        }
        let doc = snapshot(state.revision);
        let position =
            TextPosition::new(doc, &state.text, state.text.len(), TextAffinity::Upstream)
                .unwrap_or_else(|_| unreachable!("app-owned UTF-8 caret"));
        let range = SpinButtonRange::new(state.minimum, state.maximum, state.value(), 0.5)
            .unwrap_or_else(|_| unreachable!("app-owned finite validated range"));
        let input = spin_button(
            "Amount",
            doc,
            state.text.clone(),
            TextSelection::collapsed(position),
            range,
            |intent| Action::Edit(Box::new(intent)),
        )
        .unwrap_or_else(|_| unreachable!("checked M10 text source"))
        .on_adjust(Action::Adjust)
        .read_only(state.read_only)
        .disabled(state.disabled)
        .id("form.amount")
        .into_element();
        if state.show_step_controls {
            let mut increase = runenui_core::button("Increase")
                .on_activate(|| Action::Adjust(SpinButtonRequest::AdjustBy(number(0.5))));
            let mut decrease = runenui_core::button("Decrease")
                .on_activate(|| Action::Adjust(SpinButtonRequest::AdjustBy(number(-0.5))));
            if state.disabled || state.read_only {
                increase = increase.disabled();
                decrease = decrease.disabled();
            }
            runenui_core::row(runenui_core::children![input, increase, decrease]).into_element()
        } else {
            input
        }
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Edit(intent) => {
                let span = intent.replacement();
                state
                    .text
                    .replace_range(span.start()..span.end(), intent.replacement_text());
                state.revision += 1;
                UpdateOutput::edit(EditResolution::accepted(
                    intent.request().clone(),
                    snapshot(state.revision),
                ))
            }
            Action::Adjust(request) => {
                state.apply_request(request);
                UpdateOutput::effects(Effects::none())
            }
            Action::Text(text) => {
                state.text = text;
                state.revision += 1;
                UpdateOutput::effects(Effects::none())
            }
            Action::ReadOnly(value) => {
                state.read_only = value;
                UpdateOutput::effects(Effects::none())
            }
            Action::Disabled(value) => {
                state.disabled = value;
                UpdateOutput::effects(Effects::none())
            }
            Action::ShowStepControls(value) => {
                state.show_step_controls = value;
                UpdateOutput::effects(Effects::none())
            }
            Action::Remove => {
                state.present = false;
                UpdateOutput::effects(Effects::none())
            }
        }
    }
}

const FONT: &[u8] =
    include_bytes!("../../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf");

fn runtime(text: &str) -> AppRuntime<NumericApp> {
    runtime_with_bounds(text, Some(-10.0), Some(100.0))
}

fn runtime_with_bounds(
    text: &str,
    minimum: Option<f64>,
    maximum: Option<f64>,
) -> AppRuntime<NumericApp> {
    let mut runtime = AppRuntime::<NumericApp>::mount(State {
        text: text.into(),
        revision: 1,
        read_only: false,
        disabled: false,
        present: true,
        minimum,
        maximum,
        show_step_controls: false,
        requests: Vec::new(),
    });
    assert!(
        runtime
            .register_text_font_bytes(FONT.to_vec())
            .unwrap_or_else(|_| unreachable!("fixture font registers"))
            > 0
    );
    assert!(
        runtime
            .set_text_generic_family_mapping(
                GenericFontFamily::SansSerif,
                &[FontFamilyName::new("Cantarell")
                    .unwrap_or_else(|_| unreachable!("fixture family"))],
            )
            .unwrap_or_else(|_| unreachable!("fixture font mapping"))
    );
    settle(&mut runtime);
    runtime
}

fn settle(runtime: &mut AppRuntime<NumericApp>) {
    let _ = runtime
        .pump(PumpBudget::new(256, 256, 256, 256))
        .unwrap_or_else(|_| unreachable!("bounded numeric app pump observation"));
}

fn publish(runtime: &mut AppRuntime<NumericApp>) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &StyleEnvironment::default(),
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("single numeric editor publishes"))
}

fn semantic_target(
    publication: &runenui_runtime::SurfacePublication,
) -> (runenui_core::SurfaceId, runenui_core::SemanticNodeId) {
    let snapshot = publication.semantic_publication().snapshot();
    let node = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::SpinButton)
        .unwrap_or_else(|| unreachable!("typed SpinButton is published"));
    (snapshot.surface_id().clone(), node.id().clone())
}

#[test]
fn semantic_steps_are_fifo_relative_to_live_application_state() {
    let mut runtime = runtime("4");
    let publication = publish(&mut runtime);
    let (surface, target) = semantic_target(&publication);
    let node = publication
        .semantic_publication()
        .snapshot()
        .node(&target)
        .unwrap_or_else(|| unreachable!("semantic target resolves"));
    assert_eq!(
        node.range().and_then(runenui_core::SemanticRange::current),
        Some(number(4.0))
    );
    for _ in 0..2 {
        runtime
            .submit_semantic_action(SemanticActionRequest::new(
                surface.clone(),
                target.clone(),
                SemanticAction::Increment,
            ))
            .unwrap_or_else(|_| unreachable!("published step is available"));
    }
    assert_eq!(
        runtime.state().text,
        "4",
        "enqueue does not mutate application"
    );
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "5");
    assert_eq!(runtime.state().requests.len(), 2);
    let after = publish(&mut runtime);
    let (surface, target) = semantic_target(&after);
    runtime
        .submit_semantic_action(SemanticActionRequest::set_value(
            surface,
            target,
            number(-9.5),
        ))
        .unwrap_or_else(|_| unreachable!("absolute set admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "-9.5");
}

#[test]
fn incomplete_and_invalid_text_never_fabricates_numeric_semantics() {
    let mut runtime = runtime("4");
    runtime
        .submit_action(Action::Text("-".into()))
        .unwrap_or_else(|_| unreachable!("application owns invalid text"));
    settle(&mut runtime);
    let publication = publish(&mut runtime);
    let (surface, target) = semantic_target(&publication);
    let semantic = publication.semantic_publication().snapshot();
    let node = semantic
        .node(&target)
        .unwrap_or_else(|| unreachable!("SpinButton still semantic"));
    assert!(
        node.range()
            .and_then(runenui_core::SemanticRange::current)
            .is_none()
    );
    assert!(
        !node
            .supported_actions()
            .contains(&SemanticAction::Increment)
    );
    assert!(
        runtime
            .submit_semantic_action(SemanticActionRequest::new(
                surface,
                target,
                SemanticAction::Increment,
            ))
            .is_err()
    );
    assert_eq!(runtime.state().text, "-");
    assert_eq!(runtime.state().requests.as_slice(), &[]);
}

#[test]
fn m10_editing_remains_application_owned_and_step_readiness_respects_disabled_read_only() {
    let mut runtime = runtime("1");
    let id = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            id,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request admitted"));
    settle(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("2", None)
                .unwrap_or_else(|_| unreachable!("nonempty text event")),
        )
        .unwrap_or_else(|_| unreachable!("focused M10 TextField admits edit"));
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "12");
    assert_eq!(runtime.state().revision, 2);

    runtime
        .submit_action(Action::ReadOnly(true))
        .unwrap_or_else(|_| unreachable!("app update"));
    settle(&mut runtime);
    let publication = publish(&mut runtime);
    let (surface, target) = semantic_target(&publication);
    assert!(
        runtime
            .submit_semantic_action(SemanticActionRequest::new(
                surface,
                target,
                SemanticAction::Increment,
            ))
            .is_err()
    );
    runtime
        .submit_action(Action::ReadOnly(false))
        .unwrap_or_else(|_| unreachable!("app update"));
    runtime
        .submit_action(Action::Disabled(true))
        .unwrap_or_else(|_| unreachable!("app update"));
    settle(&mut runtime);
    let publication = publish(&mut runtime);
    let (surface, target) = semantic_target(&publication);
    assert!(
        runtime
            .submit_semantic_action(SemanticActionRequest::new(
                surface,
                target,
                SemanticAction::Increment,
            ))
            .is_err()
    );
}

#[test]
fn keyboard_up_and_down_emit_signed_app_steps_without_replacing_caret_arrows() {
    let mut runtime = runtime("-1");
    let id = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            id,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request"));
    settle(&mut runtime);
    for key in [LogicalKey::ArrowUp, LogicalKey::ArrowDown] {
        runtime
            .submit_keyboard(KeyboardEvent::new(
                KeyboardPhase::Down,
                PhysicalKey::Code("Arrow".to_owned()),
                key,
                KeyModifiers::NONE,
                false,
                KeyLocation::Standard,
                KeyboardCompositionState::Inactive,
                None,
            ))
            .unwrap_or_else(|_| unreachable!("key routed"));
    }
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "-1");
    assert_eq!(
        runtime.state().requests,
        [
            SpinButtonRequest::AdjustBy(number(0.5)),
            SpinButtonRequest::AdjustBy(number(-0.5)),
        ]
    );
}

#[test]
fn numeric_bounds_and_absolute_action_reject_out_of_range_without_stale_value_filter() {
    let mut runtime = runtime("100");
    let published = publish(&mut runtime);
    let (surface, target) = semantic_target(&published);
    assert!(
        runtime
            .submit_semantic_action(SemanticActionRequest::set_value(
                surface.clone(),
                target.clone(),
                number(1000.0),
            ))
            .is_err()
    );
    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            surface.clone(),
            target.clone(),
            SemanticAction::Increment,
        ))
        .unwrap_or_else(|_| unreachable!("bound step is still a relative intent"));
    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            surface,
            target,
            SemanticAction::Decrement,
        ))
        .unwrap_or_else(|_| unreachable!("reverse step is admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "99.5");
    assert_eq!(
        runtime.state().requests,
        [
            SpinButtonRequest::AdjustBy(number(0.5)),
            SpinButtonRequest::AdjustBy(number(-0.5)),
        ]
    );
}

#[test]
fn absolute_set_value_recovers_invalid_text_without_fabricating_prior_current() {
    let mut runtime = runtime("-");
    let before = publish(&mut runtime);
    let (surface, target) = semantic_target(&before);
    let node = before
        .semantic_publication()
        .snapshot()
        .node(&target)
        .unwrap_or_else(|| unreachable!("typed SpinButton target resolves"));
    assert_eq!(
        node.range().and_then(runenui_core::SemanticRange::current),
        None
    );
    assert!(node.supported_actions().contains(&SemanticAction::SetValue));
    assert!(
        !node
            .supported_actions()
            .contains(&SemanticAction::Increment)
    );
    runtime
        .submit_semantic_action(SemanticActionRequest::set_value(
            surface,
            target,
            number(7.5),
        ))
        .unwrap_or_else(|_| unreachable!("absolute SetValue is valid without prior current"));
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "7.5");
    assert_eq!(
        runtime.state().requests,
        [SpinButtonRequest::SetValue(number(7.5))]
    );
    let after = publish(&mut runtime);
    let (_, target) = semantic_target(&after);
    assert_eq!(
        after
            .semantic_publication()
            .snapshot()
            .node(&target)
            .and_then(|node| node.range())
            .and_then(runenui_core::SemanticRange::current),
        Some(number(7.5)),
    );
}

#[test]
fn open_ended_ranges_admit_exact_recovery_and_reject_only_present_bounds() {
    for (minimum, maximum, admitted, rejected) in [
        (None, Some(5.0), -40.0, 6.0),
        (Some(-2.0), None, 55.0, -3.0),
    ] {
        let mut runtime = runtime_with_bounds("-", minimum, maximum);
        let publication = publish(&mut runtime);
        let (surface, target) = semantic_target(&publication);
        let snapshot = publication.semantic_publication().snapshot();
        let node = snapshot
            .node(&target)
            .unwrap_or_else(|| unreachable!("current SpinButton semantic target"));
        let range = node
            .range()
            .unwrap_or_else(|| unreachable!("authored range"));
        assert_eq!(range.current(), None);
        assert_eq!(
            range.minimum().map(runenui_core::SemanticNumber::get),
            minimum
        );
        assert_eq!(
            range.maximum().map(runenui_core::SemanticNumber::get),
            maximum
        );
        assert!(node.supported_actions().contains(&SemanticAction::SetValue));
        assert!(
            !node
                .supported_actions()
                .contains(&SemanticAction::Increment)
        );
        assert!(
            runtime
                .submit_semantic_action(SemanticActionRequest::set_value(
                    surface.clone(),
                    target.clone(),
                    number(rejected),
                ))
                .is_err(),
            "out-of-range SetValue must reject against the present bound"
        );
        runtime
            .submit_semantic_action(SemanticActionRequest::set_value(
                surface,
                target,
                number(admitted),
            ))
            .unwrap_or_else(|_| unreachable!("open-ended absolute SetValue admitted"));
        settle(&mut runtime);
        assert_eq!(runtime.state().value(), Some(admitted));
        assert_eq!(
            runtime.state().requests,
            [SpinButtonRequest::SetValue(number(admitted))]
        );
    }

    let mut runtime = runtime_with_bounds("-", None, None);
    let before = publish(&mut runtime);
    let (surface, target) = semantic_target(&before);
    runtime
        .submit_semantic_action(SemanticActionRequest::set_value(
            surface,
            target,
            number(-600.0),
        ))
        .unwrap_or_else(|_| unreachable!("unbounded absolute numeric recovery"));
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "-600");
}

#[test]
fn optional_ordinary_step_buttons_share_app_actions_and_fifo_without_another_editor() {
    let mut runtime = runtime("4");
    runtime
        .submit_action(Action::ShowStepControls(true))
        .unwrap_or_else(|_| unreachable!("app authors ordinary sibling buttons"));
    settle(&mut runtime);
    let publication = publish(&mut runtime);
    let snapshot = publication.semantic_publication().snapshot();
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::SpinButton)
            .count(),
        1
    );
    let targets = ["Increase", "Decrease"].map(|label| {
        snapshot
            .nodes()
            .iter()
            .find(|node| node.role() == SemanticRole::Button && node.name() == Some(label))
            .unwrap_or_else(|| unreachable!("ordinary named button projects"))
            .id()
            .clone()
    });
    for target in targets {
        runtime
            .submit_semantic_action(SemanticActionRequest::new(
                snapshot.surface_id().clone(),
                target,
                SemanticAction::Activate,
            ))
            .unwrap_or_else(|_| unreachable!("ordinary button accepts action"));
    }
    assert_eq!(
        runtime.state().text,
        "4",
        "button admission does not mutate application state"
    );
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "4");
    assert_eq!(
        runtime.state().requests,
        [
            SpinButtonRequest::AdjustBy(number(0.5)),
            SpinButtonRequest::AdjustBy(number(-0.5)),
        ]
    );
}

#[test]
fn read_only_and_disabled_sibling_buttons_reject_ordinary_activation() {
    for (read_only, disabled) in [(true, false), (false, true)] {
        let mut runtime = runtime("4");
        runtime
            .submit_action(Action::ShowStepControls(true))
            .unwrap_or_else(|_| unreachable!("app authors normal button controls"));
        runtime
            .submit_action(Action::ReadOnly(read_only))
            .unwrap_or_else(|_| unreachable!("app authors read-only"));
        runtime
            .submit_action(Action::Disabled(disabled))
            .unwrap_or_else(|_| unreachable!("app authors disabled"));
        settle(&mut runtime);
        let publication = publish(&mut runtime);
        let snapshot = publication.semantic_publication().snapshot();
        for label in ["Increase", "Decrease"] {
            let button = snapshot
                .nodes()
                .iter()
                .find(|node| node.role() == SemanticRole::Button && node.name() == Some(label))
                .unwrap_or_else(|| unreachable!("ordinary button remains semantic"));
            assert!(button.state().disabled());
            assert!(
                runtime
                    .submit_semantic_action(SemanticActionRequest::new(
                        snapshot.surface_id().clone(),
                        button.id().clone(),
                        SemanticAction::Activate,
                    ))
                    .is_err()
            );
        }
        settle(&mut runtime);
        assert_eq!(runtime.state().text, "4");
        assert_eq!(runtime.state().requests.as_slice(), &[]);
    }
}

#[test]
fn generic_controller_equivalent_commands_use_existing_application_fifo() {
    let mut runtime = runtime("4");
    let owner = runtime.index().nodes()[0].id().clone();
    for command in [
        SemanticCommand::Increment,
        SemanticCommand::Increment,
        SemanticCommand::Decrement,
    ] {
        runtime
            .submit_command(owner.clone(), command, CommandOrigin::programmatic())
            .unwrap_or_else(|_| unreachable!("generic routed numeric command"));
    }
    assert_eq!(runtime.state().text, "4");
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "4.5");
    assert_eq!(
        runtime.state().requests,
        [
            SpinButtonRequest::AdjustBy(number(0.5)),
            SpinButtonRequest::AdjustBy(number(0.5)),
            SpinButtonRequest::AdjustBy(number(-0.5)),
        ]
    );
}

#[test]
fn unbounded_valid_current_supports_increment_decrement_in_one_fifo() {
    let mut runtime = runtime_with_bounds("-2", None, None);
    let published = publish(&mut runtime);
    let (surface, target) = semantic_target(&published);
    let snapshot = published.semantic_publication().snapshot();
    let node = snapshot
        .node(&target)
        .unwrap_or_else(|| unreachable!("unbounded current spin target"));
    assert_eq!(
        node.range().and_then(runenui_core::SemanticRange::current),
        Some(number(-2.0))
    );
    for action in [SemanticAction::Increment, SemanticAction::Decrement] {
        assert!(node.supported_actions().contains(&action));
        runtime
            .submit_semantic_action(SemanticActionRequest::new(
                surface.clone(),
                target.clone(),
                action,
            ))
            .unwrap_or_else(|_| unreachable!("unbounded step action admitted"));
    }
    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            surface,
            target,
            SemanticAction::Increment,
        ))
        .unwrap_or_else(|_| unreachable!("second increment admitted"));
    assert_eq!(runtime.state().text, "-2", "not mutated during ingress");
    settle(&mut runtime);
    assert_eq!(runtime.state().text, "-1.5");
    assert_eq!(
        runtime.state().requests,
        [
            SpinButtonRequest::AdjustBy(number(0.5)),
            SpinButtonRequest::AdjustBy(number(-0.5)),
            SpinButtonRequest::AdjustBy(number(0.5)),
        ]
    );
}

#[test]
fn exact_removed_semantic_target_is_not_reused_for_replacement_text() {
    let mut runtime = runtime("4.5");
    let before = publish(&mut runtime);
    let (surface, target) = semantic_target(&before);
    runtime
        .submit_action(Action::Remove)
        .unwrap_or_else(|_| unreachable!("app removes owner"));
    settle(&mut runtime);
    let after = publish(&mut runtime);
    assert!(
        after
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .all(|node| node.role() != SemanticRole::SpinButton)
    );
    assert!(
        runtime
            .submit_semantic_action(SemanticActionRequest::new(
                surface,
                target,
                SemanticAction::Increment,
            ))
            .is_err()
    );
    assert_eq!(runtime.state().requests.as_slice(), &[]);
}

#[test]
fn composition_active_key_rejects_numeric_adjustment() {
    let mut runtime = runtime("-1");
    let id = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            id,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request"));
    settle(&mut runtime);
    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code("ArrowUp".to_owned()),
            LogicalKey::ArrowUp,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Active,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("composition-time key routed"));
    settle(&mut runtime);
    assert_eq!(runtime.state().requests.as_slice(), &[]);
    assert_eq!(runtime.state().text, "-1");
}

#[derive(Debug)]
struct DownstreamSpin {
    snapshot: TextDocumentSnapshot,
    text: String,
    selection: TextSelection,
    range: runenui_core::SemanticRange,
}
impl runenui_core::Widget<()> for DownstreamSpin {
    type State = ();
    fn create_state(&self) -> Self::State {}

    fn text_input(&self, &(): &Self::State) -> runenui_core::WidgetTextInput {
        runenui_core::WidgetTextInput::new(true, true)
    }

    fn editable(&self, &(): &Self::State) -> Option<runenui_core::EditableContribution<()>> {
        runenui_core::EditableContribution::new(
            self.snapshot,
            self.text.as_str(),
            self.selection,
            runenui_core::TextSensitivity::Public,
            false,
            false,
            runenui_core::EditingSessionPolicy::PreserveExact,
            |_| (),
        )
        .ok()
        .map(|value| value.with_newline_policy(runenui_core::TextNewlinePolicy::ReplaceWithSpace))
    }

    fn measure(
        &self,
        &(): &Self::State,
        _: runenui_core::WidgetMeasureInput,
    ) -> runenui_core::WidgetMeasure {
        runenui_core::WidgetMeasure::Text(
            runenui_core::TextLeafMeasure::new(self.text.as_str())
                .with_wrap_mode(runenui_core::TextLeafWrap::NoWrap),
        )
    }

    fn semantics(
        &self,
        &(): &Self::State,
        _: runenui_core::SemanticContributionContext,
    ) -> runenui_core::SemanticContribution {
        let editable = runenui_core::SemanticEditable::new(
            self.snapshot,
            &self.text,
            self.selection,
            runenui_core::TextSensitivity::Public,
            false,
        )
        .unwrap_or_else(|| unreachable!("same checked M10 source"));
        let node = runenui_core::SemanticNodeContribution::primary(SemanticRole::SpinButton)
            .with_name("Amount")
            .with_range(self.range.clone())
            .with_state(
                runenui_core::SemanticState::ENABLED
                    .with_disabled(false)
                    .with_read_only(false)
                    .with_required(false),
            )
            .with_editable(editable)
            .with_editable_mode(runenui_core::SemanticEditableMode::SingleLine)
            .with_action(SemanticAction::MoveBackward)
            .with_action(SemanticAction::MoveForward)
            .with_action(SemanticAction::ExtendBackward)
            .with_action(SemanticAction::ExtendForward)
            .with_action(SemanticAction::SelectAll)
            .with_action(SemanticAction::SetSelection)
            .with_action(SemanticAction::DeleteBackward)
            .with_action(SemanticAction::DeleteForward)
            .with_action(SemanticAction::Undo)
            .with_action(SemanticAction::Redo)
            .with_action(SemanticAction::ReplaceSelection)
            .with_action(SemanticAction::Increment)
            .with_action(SemanticAction::Decrement)
            .with_action(SemanticAction::SetValue);
        runenui_core::SemanticContribution::single(node)
    }
}

#[test]
fn downstream_widget_can_author_identical_editing_range_and_action_contracts() {
    let snapshot = snapshot(9);
    let value = "12";
    let caret = TextPosition::new(snapshot, value, value.len(), TextAffinity::Upstream)
        .unwrap_or_else(|_| unreachable!("fixture UTF8 caret"));
    let selection = TextSelection::collapsed(caret);
    let range = SpinButtonRange::new(Some(-10.0), Some(100.0), Some(12.0), 0.5)
        .unwrap_or_else(|_| unreachable!("valid test range"));
    let downstream: runenui_core::Element<()> = runenui_core::Element::new(DownstreamSpin {
        snapshot,
        text: value.into(),
        selection,
        range: range.semantic_range().clone(),
    })
    .focusable(true);
    let standard = spin_button("Amount", snapshot, value, selection, range, |_| ())
        .unwrap_or_else(|_| unreachable!("same checked standard field"))
        .on_adjust(|_| ())
        .into_element();

    let (_, _, _, _, _, _, _, _, downstream, _) = downstream.into_runtime_parts().into_parts();
    let (_, _, _, _, _, _, _, _, standard, _) = standard.into_runtime_parts().into_parts();
    let a = downstream.create_state();
    let b = standard.create_state();
    assert_eq!(downstream.text_input(&a), standard.text_input(&b));
    assert_eq!(
        downstream
            .editable(&a)
            .unwrap_or_else(|_| unreachable!("custom M10"))
            .unwrap_or_else(|| unreachable!("custom editor"))
            .newline_policy(),
        standard
            .editable(&b)
            .unwrap_or_else(|_| unreachable!("standard M10"))
            .unwrap_or_else(|| unreachable!("standard editor"))
            .newline_policy(),
    );
    assert_eq!(
        downstream
            .semantics(&a, runenui_core::SemanticContributionContext::default())
            .unwrap_or_else(|_| unreachable!("custom role")),
        standard
            .semantics(&b, runenui_core::SemanticContributionContext::default())
            .unwrap_or_else(|_| unreachable!("standard role")),
    );
}
