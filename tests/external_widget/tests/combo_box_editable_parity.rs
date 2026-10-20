#![allow(refining_impl_trait)]

//! A downstream Widget composes the M10 editor, `ComboBox` M5 proxy, M4 focus
//! and ordinary `OptionItem` activation without private built-in recognition.
use runenui_core::{
    ChildBearingWidget, CommandOrigin, CommittedTextEvent, EditIntent, EditResolution,
    EditableContribution, EditingSessionPolicy, Effects, ElementId, EventContext, EventPhase,
    IntoUpdateOutput, NoHostProtocol, SemanticAction, SemanticAutocomplete, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticEditable, SemanticEditableMode,
    SemanticNodeContribution, SemanticPopupKind, SemanticReference, SemanticRelationship,
    SemanticRelationshipKind, SemanticRole, SemanticState, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextLeafMeasure, TextNewlinePolicy, TextPosition,
    TextSelection, TextSensitivity, UiApp, UiEvent, UpdateOutput, View, Widget, WidgetEventOutput,
    WidgetInvalidation, WidgetMeasure, WidgetMeasureInput, WidgetTextInput, WidgetUpdateContext,
    container, list_box, option_item,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, PumpBudget, SurfaceBuildContext,
};

const FONT: &[u8] =
    include_bytes!("../../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf");

const fn doc(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(
        TextDocumentId::new(30331),
        TextDocumentRevision::new(revision),
    )
}
enum Action {
    Edit(Box<EditIntent>),
    Choose(u8),
}
struct State {
    source: String,
    revision: u64,
    chosen: u8,
}
#[derive(Debug)]
struct Owner {
    snapshot: TextDocumentSnapshot,
    source: String,
    selection: TextSelection,
}
impl Widget<Action> for Owner {
    type State = bool;

    fn create_state(&self) -> Self::State {
        false
    }

    fn update(&self, _: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        context.invalidate(
            WidgetInvalidation::SEMANTICS | WidgetInvalidation::LAYOUT | WidgetInvalidation::PAINT,
        );
    }

    fn event(
        &mut self,
        second_active: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && !context.default_is_prevented()
            && event
                .as_semantic_command()
                .is_some_and(|command| command.command() == SemanticCommand::FocusDown)
        {
            *second_active = true;
            context.invalidate(WidgetInvalidation::SEMANTICS);
            context.prevent_default();
            context.stop_propagation();
        }
        WidgetEventOutput::none()
    }

    fn text_input(&self, _: &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, _: &Self::State) -> Option<EditableContribution<Action>> {
        EditableContribution::new(
            self.snapshot,
            self.source.as_str(),
            self.selection,
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            |intent| Action::Edit(Box::new(intent)),
        )
        .ok()
        .map(|contribution| contribution.with_newline_policy(TextNewlinePolicy::ReplaceWithSpace))
    }

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text(TextLeafMeasure::new(self.source.as_str()))
    }

    fn semantics(
        &self,
        second_active: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let Some(editable) = SemanticEditable::new(
            self.snapshot,
            self.source.as_str(),
            self.selection,
            TextSensitivity::Public,
            false,
        ) else {
            return SemanticContribution::empty();
        };
        let option_id = if *second_active {
            "parity.second"
        } else {
            "parity.first"
        };
        let mut node = SemanticNodeContribution::primary(SemanticRole::ComboBox)
            .with_name("Custom editable")
            .with_state(SemanticState::ENABLED.with_expanded(true))
            .with_popup(SemanticPopupKind::ListBox)
            .with_autocomplete(SemanticAutocomplete::List)
            .with_editable_mode(SemanticEditableMode::SingleLine)
            .with_editable(editable)
            .with_action(SemanticAction::RequestFocus)
            .with_action(SemanticAction::MoveBackward)
            .with_action(SemanticAction::MoveForward)
            .with_action(SemanticAction::SelectAll)
            .with_action(SemanticAction::SetSelection)
            .with_action(SemanticAction::ReplaceSelection)
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                SemanticReference::Authored {
                    element_id: ElementId::new("parity.popup").unwrap_or_else(|_| unreachable!()),
                    semantic_key: None,
                },
            ))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ActiveDescendant,
                SemanticReference::Authored {
                    element_id: ElementId::new(option_id).unwrap_or_else(|_| unreachable!()),
                    semantic_key: None,
                },
            ));
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}
impl ChildBearingWidget<Action> for Owner {}

struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let snapshot = doc(state.revision);
        let caret = TextPosition::new(
            snapshot,
            state.source.as_str(),
            state.source.len(),
            TextAffinity::Upstream,
        )
        .unwrap_or_else(|_| unreachable!("valid downstream caret"));
        let popup = list_box([
            option_item("First", state.chosen == 1)
                .id("parity.first")
                .combo_box_proxy()
                .on_activate(|| Action::Choose(1)),
            option_item("Second", state.chosen == 2)
                .id("parity.second")
                .combo_box_proxy()
                .on_activate(|| Action::Choose(2)),
        ])
        .id("parity.popup");
        container(
            Owner {
                snapshot,
                source: state.source.clone(),
                selection: TextSelection::collapsed(caret),
            },
            vec![popup.into_element()],
        )
        .id("parity.owner")
        .into_element()
        .focusable(true)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Edit(intent) => {
                let range = intent.replacement();
                state
                    .source
                    .replace_range(range.start()..range.end(), intent.replacement_text());
                state.revision += 1;
                UpdateOutput::edit(EditResolution::accepted(
                    intent.request().clone(),
                    doc(state.revision),
                ))
            }
            Action::Choose(value) => {
                state.chosen = value;
                UpdateOutput::effects(Effects::none())
            }
        }
    }
}

fn settle(app: &mut AppRuntime<App>) {
    assert!(
        app.pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|error| unreachable!("ComboBox pump: {error:?}"))
            .report()
            .is_quiescent()
    );
}

#[test]
fn downstream_widget_owns_the_same_editable_proxy_contract_without_private_runtime_paths() {
    let mut app = AppRuntime::<App>::mount(State {
        source: "a".into(),
        revision: 1,
        chosen: 0,
    });
    app.register_text_font_bytes(FONT.to_vec())
        .unwrap_or_else(|_| unreachable!());
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!())],
    )
    .unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    let owner = app
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "parity.owner")
        })
        .unwrap_or_else(|| unreachable!("downstream mounted owner"))
        .id()
        .clone();
    assert!(app.index().nodes().iter().all(|node| {
        node.authored_id()
            .is_none_or(|id| id.as_str() != "parity.first")
            || !node.is_focusable()
    }));
    app.submit_command(
        owner.clone(),
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("current focus command"));
    settle(&mut app);
    let semantic = app
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(480.0, 260.0).unwrap_or_else(|_| unreachable!()),
        ))
        .unwrap_or_else(|error| unreachable!("downstream publication: {error:?}"));
    let snapshot = semantic.semantic_publication().snapshot();
    let combo = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("downstream ComboBox semantics"));
    assert!(combo.editable().is_some());
    assert_eq!(
        combo.editable_mode(),
        Some(SemanticEditableMode::SingleLine)
    );
    assert!(
        !snapshot
            .nodes()
            .iter()
            .any(|node| node.role() == SemanticRole::EditableText)
    );
    assert_eq!(
        combo
            .relationships()
            .iter()
            .filter(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant)
            .count(),
        1
    );
    assert_eq!(app.focus().focused_node(), Some(&owner));
    app.submit_command(
        owner.clone(),
        SemanticCommand::FocusDown,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("routed proxy navigation"));
    settle(&mut app);
    assert_eq!(
        app.state().chosen,
        0,
        "active Option cannot commit durable selection"
    );
    app.submit_text(
        CommittedTextEvent::new("x", None).unwrap_or_else(|_| unreachable!("source text")),
    )
    .unwrap_or_else(|_| unreachable!("M10 downstream text ingress"));
    settle(&mut app);
    assert_eq!(app.state().source, "ax");
    assert_eq!(app.state().chosen, 0);
    assert_eq!(app.focus().focused_node(), Some(&owner));
}
