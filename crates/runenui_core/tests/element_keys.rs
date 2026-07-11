use runenui_core::{ElementId, ElementKey, IdentifierError, IntoElement, button};

#[test]
fn identifier_constructors_and_builder_diagnostics_are_validated() {
    assert_eq!(ElementId::new(""), Err(IdentifierError::Empty));
    assert_eq!(ElementKey::new("   "), Err(IdentifierError::WhitespaceOnly));

    let valid = button::<()>("Open")
        .id("toolbar.open")
        .key("item-1")
        .into_element();
    assert_eq!(
        valid.element_id().map(ElementId::as_str),
        Some("toolbar.open")
    );
    assert!(valid.authoring_diagnostics().is_empty());

    let invalid = button::<()>("Open").id(" ").key("").into_element();
    assert_eq!(invalid.element_id(), None);
    assert_eq!(invalid.authoring_diagnostics().len(), 2);
}
