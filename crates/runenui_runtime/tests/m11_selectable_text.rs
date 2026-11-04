macro_rules! admitted {
    ($result:expr) => {{
        let result = $result;
        assert!(
            result.is_ok(),
            "expected accepted runtime admission, got {:?}",
            result.as_ref().err()
        );
        let Ok(value) = result else { return };
        value
    }};
}

use runenui_core::{
    ClipboardWritePurpose, CommandOrigin, CommittedTextEvent, FrameworkServiceRequest,
    NoHostProtocol, SemanticAction, SemanticActionRequest, SemanticCommand, StyleEnvironment,
    TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextPosition,
    TextSelection, UiApp,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, PumpBudget, SurfaceBuildContext,
};

const FONT: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");
const SOURCE: &str = "read-only documentation";

struct SelectableApp;

impl UiApp for SelectableApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The single read-only selectable-text widget has one initial reconciliation.
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Read-only text routes can publish the same bounded event outputs and
        // request a clipboard service for a supported Copy command.
        runenui_core::ProducerBudget::new(6, 4)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 2))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 0, 2))
    }

    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl runenui_core::View<Self::Action> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(23), TextDocumentRevision::new(1));
        let position = TextPosition::new(snapshot, SOURCE, SOURCE.len(), TextAffinity::Upstream)
            .unwrap_or_else(|_| unreachable!("fixture selection is valid"));
        let control =
            runenui_core::selectable_text(snapshot, SOURCE, TextSelection::collapsed(position))
                .unwrap_or_else(|_| unreachable!("fixture text is revision-scoped"))
                .id("read-only.text");
        runenui_core::View::into_element(control)
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

fn pump_selectable(runtime: &mut AppRuntime<SelectableApp>, max_envelopes: usize) {
    let _ = runtime
        .pump(PumpBudget::new(
            max_envelopes,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("canonical pump observation"));
}

#[test]
fn selectable_text_uses_m10_for_read_only_selection_and_blocks_mutation() {
    let mut runtime = admitted!(AppRuntime::<SelectableApp>::mount(()));
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
        LogicalSize::try_new(280.0, 60.0).unwrap_or_else(|_| unreachable!("surface is finite")),
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
    assert!(
        !node
            .supported_actions()
            .contains(&SemanticAction::ReplaceSelection)
    );

    assert!(
        runtime
            .submit_semantic_action(SemanticActionRequest::replace_selection(
                initial
                    .semantic_publication()
                    .snapshot()
                    .surface_id()
                    .clone(),
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
    pump_selectable(&mut runtime, 8);
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
    pump_selectable(&mut runtime, 8);
    let selected = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("selected surface publishes"));
    let semantic = &selected.semantic_publication().snapshot().nodes()[0];
    let editable = semantic
        .editable()
        .unwrap_or_else(|| unreachable!("retained selection is projected"));
    assert_eq!(editable.selection().anchor().byte_offset(), 0);
    assert_eq!(editable.selection().active().byte_offset(), SOURCE.len());
    // The canonical M10 clipboard path is a routed command, not a published
    // semantic clipboard action until the generic service authority supports it.
    assert!(!semantic.supported_actions().contains(&SemanticAction::Copy));

    // Copy is handled by the existing framework-service FIFO, not a widget
    // callback or a parallel selection/clipboard model.
    let owner = runtime.index().nodes()[0].id().clone();
    admitted!(runtime.submit_command(owner, SemanticCommand::Copy, CommandOrigin::programmatic()));
    pump_selectable(&mut runtime, 16);
    assert!(runtime.pending_framework_services().iter().any(|service| {
        matches!(
            service.request(),
            FrameworkServiceRequest::ClipboardWriteText {
                text,
                purpose: ClipboardWritePurpose::Copy
            } if text.as_ref() == SOURCE
        )
    }));
}
