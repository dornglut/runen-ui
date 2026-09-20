#![allow(refining_impl_trait)]
#![allow(clippy::panic)]

use runenui_core::{
    EditIntent, EditableContribution, EditingSessionPolicy, Element, EventContext, HitContribution,
    HitContributionContext, LogicalPoint, LogicalRect, NoHostProtocol, PointerButton,
    PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticContribution,
    SemanticContributionContext, SemanticEditable, SemanticNodeContribution, SemanticRole,
    SemanticState, StyleEnvironment, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot,
    TextPosition, TextSelection, TextSensitivity, UiApp, UiEvent, Widget, WidgetActivation,
    WidgetEventOutput, WidgetMeasure, WidgetMeasureInput, WidgetTextInput,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, PumpBudget, RuntimeConfig,
    SurfaceBuildContext, TraceConfig, TraceRecordKind,
};

const CANTARELL: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");

#[derive(Debug)]
struct EditableProbe {
    release_capture: bool,
}

impl Widget<()> for EditableProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::NONE
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<()>> {
        let snapshot = snapshot();
        let position =
            TextPosition::new(snapshot, "ab", 2, runenui_core::TextAffinity::Downstream).ok()?;
        EditableContribution::new(
            snapshot,
            "ab",
            TextSelection::collapsed(position),
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            |_: EditIntent| (),
        )
        .ok()
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if self.release_capture
            && event
                .as_pointer()
                .is_some_and(|pointer| pointer.phase() == PointerPhase::Down)
        {
            context.capture_pointer();
            context.release_pointer_capture();
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: String::from("ab"),
        }
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("editable probe bounds are valid")),
        )
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let snapshot = snapshot();
        let position = TextPosition::new(snapshot, "ab", 2, runenui_core::TextAffinity::Downstream)
            .unwrap_or_else(|_| unreachable!("editable probe selection is valid"));
        let editable = SemanticEditable::new(
            snapshot,
            "ab",
            TextSelection::collapsed(position),
            TextSensitivity::Public,
            false,
        )
        .unwrap_or_else(|| unreachable!("editable semantic value is checked"));
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::EditableText)
                .with_state(SemanticState::ENABLED)
                .with_editable(editable),
        )
    }
}

struct PointerApp;

impl UiApp for PointerApp {
    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(release_capture: &Self::State) -> Element<Self::Action> {
        Element::new(EditableProbe {
            release_capture: *release_capture,
        })
        .focusable(true)
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

const fn snapshot() -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(TextDocumentId::new(1), TextDocumentRevision::new(1))
}

#[test]
fn explicit_capture_then_release_prevents_selection_default_from_recapturing() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<PointerApp>::mount_with_config(true, config);
    assert!(
        runtime
            .register_text_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|error| panic!("fixture font registers: {error:?}"))
            > 0
    );
    runtime
        .set_text_generic_family_mapping(
            GenericFontFamily::SansSerif,
            &[FontFamilyName::new("Cantarell")
                .unwrap_or_else(|_| unreachable!("fixture font name is valid"))],
        )
        .unwrap_or_else(|error| panic!("fixture font mapping installs: {error:?}"));
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(200.0, 40.0)
            .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("editable surface publishes: {error:?}"));
    let pointer = PointerEvent::new(
        PointerId::new(7).unwrap_or_else(|| unreachable!("fixture pointer is non-zero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        LogicalPoint::new(1.0, 18.0)
            .unwrap_or_else(|_| unreachable!("fixture pointer position is finite")),
        publication.input_context().clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    runtime
        .submit_pointer(pointer)
        .unwrap_or_else(|error| panic!("pointer down is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerTextSelectionStarted { pointer_id } if pointer_id.get() == 7
    )));
    assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
}
