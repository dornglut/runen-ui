#![allow(refining_impl_trait)]
//! One real M10 editor owns `ComboBox` focus while its bounded popup reveals
//! an exact current `Option` using the standard #344 runtime scroll authority.

use runenui_core::{
    CommandOrigin, ElementId, LogicalLength, NoHostProtocol, SemanticCommand, SemanticEditableMode,
    SemanticRelationshipKind, SemanticRole, StyleEnvironment, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, UiApp, View,
    editable_combo_box, option_item, text_field,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, MountedNodeId, PumpBudget,
    SurfaceBuildContext,
};

const FONT: &[u8] =
    include_bytes!("../../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf");

struct App;
impl UiApp for App {
    fn initial_producer_budget(state: &Self::State) -> runenui_core::ProducerBudget {
        // The open popup publishes one exact-option reveal only when the
        // selected option lies below the four visible rows.
        let outputs = usize::from(*state >= 4) * 4;
        runenui_core::ProducerBudget::new(23, outputs)
            .with_application_outputs(0)
            .with_reconciliation(23, 0, 23)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_application_outputs(0)
            .with_reconciliation(0, 23, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // TextField(2), ComboBox(1), ListBox(1), scrollbar(1), and thumb(1)
        // own Event outputs. Focus transfer reserves four notifications for
        // each owner plus the TextField/ComboBox ordinary route (28 outputs).
        // The editable default additionally owns one FrameworkService start.
        runenui_core::ProducerBudget::new(32, 30)
            .with_application_outputs(0)
            .with_reconciliation(0, 23, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 1))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 0, 1))
    }

    type State = u8;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(selected: &u8) -> impl View<()> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(331), TextDocumentRevision::new(1));
        let cursor = TextPosition::new(snapshot, "", 0, TextAffinity::Upstream)
            .unwrap_or_else(|_| unreachable!("empty M10 caret"));
        let editor = text_field(
            snapshot,
            "",
            TextSelection::collapsed(cursor),
            SemanticEditableMode::SingleLine,
            |_| (),
        )
        .unwrap_or_else(|_| unreachable!("valid single-line M10 editor"))
        .id("editable.reveal.owner");
        editable_combo_box(
            editor,
            "Editable choices",
            ElementId::new("editable.reveal.list")
                .unwrap_or_else(|_| unreachable!("valid popup ID")),
            (0..18_u8).map(|index| {
                option_item(format!("Option {index:02}"), index == *selected)
                    .id(format!("editable.reveal.option.{index}"))
                    .with_layout(
                        runenui_core::LayoutStyle::default()
                            .with_width(runenui_core::LayoutDimension::Length(LogicalLength::from(
                                160_u16,
                            )))
                            .with_height(runenui_core::LayoutDimension::Length(
                                LogicalLength::from(24_u8),
                            )),
                    )
                    .on_activate(|| ())
            }),
            |_| (),
        )
        .unwrap_or_else(|_| unreachable!("standard editable ComboBox authoring"))
        .open(true)
        .popup_max_height(LogicalLength::from(96_u8))
    }

    fn update(_: &mut u8, (): ()) {}
}

fn settle(app: &mut AppRuntime<App>) {
    assert!(
        app.pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|_| unreachable!("editable reveal pump"))
            .report()
            .is_quiescent()
    );
}

fn id(app: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
    let requested = ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    app.index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&requested))
        .unwrap_or_else(|| unreachable!("live authored Option"))
        .id()
        .clone()
}

fn publish(app: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    app.publish_surface(&SurfaceBuildContext::tight(
        &StyleEnvironment::default(),
        LogicalSize::try_new(350.0, 320.0).unwrap_or_else(|_| unreachable!()),
    ))
    .unwrap_or_else(|error| unreachable!("editable ComboBox publication: {error:?}"))
}

fn command(app: &mut AppRuntime<App>, target: MountedNodeId, command: SemanticCommand) {
    app.submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("exact live ComboBox command"));
    settle(app);
}

fn configured(selected: u8) -> AppRuntime<App> {
    let mut app = AppRuntime::<App>::mount(selected)
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    app.register_text_font_bytes(FONT.to_vec())
        .unwrap_or_else(|_| unreachable!("bundled text font"));
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!())],
    )
    .unwrap_or_else(|_| unreachable!("known generic font"));
    settle(&mut app);
    app
}

#[test]
fn editable_initial_open_and_keyboard_navigation_reveal_without_focus_or_text_mutation() {
    for selected in [0_u8, 17] {
        let mut app = configured(selected);
        let owner = id(&mut app, "editable.reveal.owner");
        command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
        let list = id(&mut app, "editable.reveal.list");
        let viewport = app
            .index()
            .node(&list)
            .and_then(runenui_runtime::MountedNodeRef::parent)
            .unwrap_or_else(|| unreachable!("standard nonfocusable viewport"))
            .clone();
        let _ = publish(&mut app);
        assert_eq!(
            app.index()
                .node(&viewport)
                .unwrap_or_else(|| unreachable!())
                .interaction()
                .scroll_offset()
                .1,
            0.0
        );
        settle(&mut app);
        if selected == 0 {
            command(&mut app, owner.clone(), SemanticCommand::FocusGroupLast);
        }
        assert!(
            app.index()
                .node(&viewport)
                .unwrap_or_else(|| unreachable!())
                .interaction()
                .scroll_offset()
                .1
                > 0.0
        );
        let publication = publish(&mut app);
        let semantics = publication.semantic_publication().snapshot();
        let combo = semantics
            .nodes()
            .iter()
            .find(|node| node.role() == SemanticRole::ComboBox)
            .unwrap_or_else(|| unreachable!("editor contributes ComboBox semantics"));
        assert!(combo.editable().is_some());
        let active = combo
            .relationships()
            .iter()
            .find(|rel| rel.kind() == SemanticRelationshipKind::ActiveDescendant)
            .unwrap_or_else(|| unreachable!("current active Option"));
        assert_eq!(
            semantics.node(active.target()).and_then(|node| node.name()),
            Some("Option 17")
        );
        assert_eq!(app.focus().focused_node(), Some(&owner));
        assert_eq!(
            *app.state(),
            selected,
            "revealing cannot mutate durable selection"
        );
    }
}
