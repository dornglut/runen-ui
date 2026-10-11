#![allow(refining_impl_trait)]

//! Real public M11 control trees projected through the ordinary native AccessKit adapter.
//! These are not synthetic semantic-node fixtures or a host-specific ComboBox implementation.
use accesskit::{AutoComplete, HasPopup, Role};
use runenui_core::{
    ElementId, FontFamilyName, GenericFontFamily, NoHostProtocol, SemanticAction,
    SemanticEditableMode,
    SemanticRole, StyleEnvironment, TextAffinity, TextDocumentId, TextDocumentRevision,
    TextDocumentSnapshot, TextPosition, TextSelection, UiApp, View, combo_box,
    editable_combo_box, option_item, text_field,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};
use runenui_winit::accessibility::{AdapterDiagnostic, SemanticAdapter, UpdateMode};

const FONT: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");

struct SelectApp;
impl UiApp for SelectApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        combo_box(
            "Select choice",
            ElementId::new("access.select.popup").unwrap_or_else(|_| unreachable!()),
            [
                option_item("One", true).id("access.select.one").on_activate(|| ()),
                option_item("Two", false).id("access.select.two").on_activate(|| ()),
            ],
            |_| (),
        )
        .id("access.select.owner")
    }

    fn update(_: &mut Self::State, _: Self::Action) {}
}

struct EditableApp;
impl UiApp for EditableApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        let value = "search";
        let snapshot = TextDocumentSnapshot::new(
            TextDocumentId::new(331001),
            TextDocumentRevision::new(1),
        );
        let caret = TextPosition::new(
            snapshot, value, value.len(), TextAffinity::Upstream,
        ).unwrap_or_else(|_| unreachable!("current UTF-8 caret"));
        let editor = text_field(
            snapshot,
            value,
            TextSelection::collapsed(caret),
            SemanticEditableMode::SingleLine,
            |_| (),
        ).unwrap_or_else(|_| unreachable!("valid M10 source"))
            .id("access.editable.owner");
        editable_combo_box(
            editor,
            "Search choices",
            ElementId::new("access.editable.popup").unwrap_or_else(|_| unreachable!()),
            [
                option_item("Alpha", false).id("access.editable.alpha").on_activate(|| ()),
                option_item("Beta", false).id("access.editable.beta").on_activate(|| ()),
            ],
            |_| (),
        ).unwrap_or_else(|_| unreachable!("single public M10 editor"))
            .open(true)
    }

    fn update(_: &mut Self::State, _: Self::Action) {}
}

fn publish<App: UiApp<State = (), Action = (), HostProtocol = NoHostProtocol>>(
    runtime: &mut AppRuntime<App>,
) -> runenui_runtime::SurfacePublication {
    runtime.register_text_font_bytes(FONT.to_vec())
        .unwrap_or_else(|_| unreachable!("known fixture font is admitted"));
    runtime.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!())],
    ).unwrap_or_else(|_| unreachable!("known font family is configured"));
    assert!(runtime.pump(PumpBudget::new(512, 512, 512, 512)).is_quiescent());
    runtime.publish_surface(&SurfaceBuildContext::tight(
        &StyleEnvironment::default(),
        LogicalSize::try_new(520.0, 280.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("current M11 accessibility publication: {error:?}"))
}

#[test]
fn actual_collapsed_select_only_combo_maps_current_selected_value_to_accesskit() {
    let mut runtime = AppRuntime::<SelectApp>::mount(());
    let publication = publish(&mut runtime);
    assert!(
        publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .any(|node| node.role() == SemanticRole::ComboBox && node.value().is_some())
    );

    let mut adapter = SemanticAdapter::new();
    let update = adapter.update(publication.semantic_publication());
    assert_eq!(update.mode, UpdateMode::InitialFull);
    assert!(
        update.diagnostics.is_empty(),
        "normal ComboBox needs no native fallback"
    );
    let native = update
        .tree_update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Select choice"))
        .unwrap_or_else(|| unreachable!("one native select-only ComboBox"))
        .1
        .clone();
    assert_eq!(native.role(), Role::ComboBox);
    assert_eq!(native.value(), Some("One"));
    assert_eq!(native.is_expanded(), Some(false));
    assert_eq!(native.has_popup(), Some(HasPopup::Listbox));
    assert!(native.active_descendant().is_none());
}

#[test]
fn actual_editable_combo_maps_one_m10_owner_and_exact_active_option_to_accesskit() {
    let mut runtime = AppRuntime::<EditableApp>::mount(());
    let publication = publish(&mut runtime);
    let snapshot = publication.semantic_publication().snapshot();
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::ComboBox)
            .count(),
        1
    );
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::EditableText)
            .count(),
        0
    );

    let mut adapter = SemanticAdapter::new();
    let update = adapter.update(publication.semantic_publication());
    assert_eq!(update.mode, UpdateMode::InitialFull);
    for diagnostic in &update.diagnostics {
        assert!(
            matches!(
                diagnostic,
                AdapterDiagnostic::UnsupportedSemanticAction {
                    action: SemanticAction::MoveBackward
                        | SemanticAction::MoveForward
                        | SemanticAction::ExtendBackward
                        | SemanticAction::ExtendForward
                        | SemanticAction::SelectAll
                        | SemanticAction::DeleteBackward
                        | SemanticAction::DeleteForward
                        | SemanticAction::Undo
                        | SemanticAction::Redo,
                    ..
                }
            ),
            "native ComboBox must not introduce structural or relationship diagnostics: {diagnostic:?}"
        );
    }
    let (owner_id, combo) = update
        .tree_update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Search choices"))
        .unwrap_or_else(|| unreachable!("one native editable ComboBox"));
    assert_eq!(combo.role(), Role::EditableComboBox);
    assert_eq!(combo.value(), Some("search"));
    assert_eq!(combo.has_popup(), Some(HasPopup::Listbox));
    assert_eq!(combo.auto_complete(), Some(AutoComplete::List));
    assert_eq!(combo.is_expanded(), Some(true));
    let popup = combo
        .controls()
        .first()
        .unwrap_or_else(|| unreachable!("actual popup ListBox target"));
    assert_eq!(
        update
            .tree_update
            .nodes
            .iter()
            .find(|(id, _)| id == popup)
            .map(|(_, node)| node.role()),
        Some(Role::ListBox)
    );
    let active = combo
        .active_descendant()
        .unwrap_or_else(|| unreachable!("current actual active Option"));
    assert_ne!(
        active, *owner_id,
        "real focus owner is not the active proxy Option"
    );
    let active_node = update
        .tree_update
        .nodes
        .iter()
        .find(|(id, _)| *id == active)
        .unwrap_or_else(|| unreachable!("native active Option projected"));
    assert_eq!(active_node.1.role(), Role::ListBoxOption);
    assert_eq!(active_node.1.label(), Some("Alpha"));
}
