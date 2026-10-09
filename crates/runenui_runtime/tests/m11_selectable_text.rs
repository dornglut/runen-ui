use runenui_core::{
    CommandOrigin, CommittedTextEvent, Element, NoHostProtocol, SemanticAction,
    SemanticActionRequest, SemanticCommand, StyleEnvironment, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, UiApp,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, PumpBudget, SurfaceBuildContext,
};

const FONT: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");
const SOURCE: &str = "read-only documentation";

struct SelectableApp;

impl UiApp for SelectableApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(23), TextDocumentRevision::new(1));
        let position = TextPosition::new(snapshot, SOURCE, SOURCE.len(), TextAffinity::Downstream)
            .unwrap_or_else(|_| unreachable!("fixture selection is valid"));
        let control = runenui_core::selectable_text(
            snapshot,
            SOURCE,
            TextSelection::collapsed(position),
        )
        .unwrap_or_else(|_| unreachable!("fixture text is revision-scoped"))
        .id("read-only.text");
        runenui_core::View::into_element(control)
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn selectable_text_uses_m10_for_read_only_selection_and_blocks_mutation() {
    let mut runtime = AppRuntime::<SelectableApp>::mount(());
    assert!(
        runtime
            .register_text_font_bytes(FONT.to_vec())
            .unwrap_or_else(|_| unreachable!("font registers"))
            > 0
    );
    assert!(
        runtime
            .set_text_generic_family_mapping(
                GenericFontFamily::SansSerif,
                &[FontFamilyName::new("Cantarell")
                    .unwrap_or_else(|_| unreachable!("font family validates"))],
            )
            .unwrap_or_else(|_| unreachable!("font mapping registers"))
    );
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(280.0, 60.0)
            .unwrap_or_else(|_| unreachable!("surface is finite")),
    );
    let initial = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("selectable surface publishes"));
    let node = &initial.semantic_publication().snapshot().nodes()[0];
    assert!(node.state().read_only());
    assert_eq!(
        node.editable().and_then(|editable| editable.value()),
        Some(SOURCE)
    );
    assert!(!node.supported_actions().contains(&SemanticAction::Paste));
    assert!(!node.supported_actions().contains(&SemanticAction::Cut));
    assert!(!node.supported_actions().contains(&SemanticAction::ReplaceSelection));

    assert!(
        runtime
            .submit_semantic_action(SemanticActionRequest::replace_selection(
                initial.semantic_publication().snapshot().surface_id().clone(),
                node.id().clone(),
                "mutation",
            ))
            .is_err()
    );
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("read-only text can focus"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert!(
        runtime
            .submit_text(
                CommittedTextEvent::new("x", None)
                    .unwrap_or_else(|_| unreachable!("committed text is nonempty")),
            )
            .is_err()
    );
    runtime
        .submit_command(
            owner,
            SemanticCommand::SelectAll,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("read-only selection is routed"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    let selected = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("selected surface publishes"));
    let semantic = &selected.semantic_publication().snapshot().nodes()[0];
    let editable = semantic
        .editable()
        .unwrap_or_else(|| unreachable!("retained selection is projected"));
    assert_eq!(editable.selection().anchor().byte_offset(), 0);
    assert_eq!(editable.selection().active().byte_offset(), SOURCE.len());
    assert!(semantic.supported_actions().contains(&SemanticAction::Copy));
    assert!(runtime.pending_framework_services().is_empty());
}
