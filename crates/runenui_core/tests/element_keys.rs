use std::collections::{BTreeMap, HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use runenui_core::{ElementId, ElementKey, IdentifierError, View, button, element_id, element_key};

fn hash(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

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

#[test]
fn identifier_identity_is_textual_across_static_and_owned_storage() -> Result<(), IdentifierError> {
    let owned_id = ElementId::new("counter")?;
    let static_id = ElementId::from_static("counter")?;
    assert_eq!(owned_id, static_id);
    assert_eq!(owned_id.cmp(&static_id), std::cmp::Ordering::Equal);
    assert_eq!(hash(&owned_id), hash(&static_id));

    let owned_key = ElementKey::new("item")?;
    let static_key = ElementKey::from_static("item")?;
    assert_eq!(owned_key, static_key);
    assert_eq!(owned_key.cmp(&static_key), std::cmp::Ordering::Equal);
    assert_eq!(hash(&owned_key), hash(&static_key));

    let mut ordered = BTreeMap::new();
    assert_eq!(element_id!("counter"), owned_id);
    assert_eq!(element_key!("item"), owned_key);

    ordered.insert(static_id, "static");
    assert_eq!(ordered.get(&owned_id), Some(&"static"));
    let mut hashed = HashMap::new();
    hashed.insert(static_key, "static");
    assert_eq!(hashed.get(&owned_key), Some(&"static"));

    Ok(())
}

#[test]
fn identifiers_use_one_unicode_aware_grammar_in_all_public_paths() {
    let invalid = [
        ("\u{00A0}", IdentifierError::WhitespaceOnly),
        ("\u{2003}", IdentifierError::WhitespaceOnly),
        ("\u{00A0}name", IdentifierError::SurroundingWhitespace),
        ("name\u{2003}", IdentifierError::SurroundingWhitespace),
        ("name\u{0085}value", IdentifierError::ControlCharacter),
    ];
    for (value, expected) in invalid {
        assert_eq!(ElementId::new(value), Err(expected));
        assert_eq!(ElementKey::new(value), Err(expected));
        assert_eq!(ElementId::from_static(value), Err(expected));
        assert_eq!(ElementKey::from_static(value), Err(expected));

        let invalid_builder = button::<()>("Open").id(value).key(value).into_element();
        assert_eq!(invalid_builder.authoring_diagnostics().len(), 2);
        assert!(
            invalid_builder
                .authoring_diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.error() == expected)
        );
    }

    for value in ["fenster.öffnen", "画面.開始", "контрол.кнопка"] {
        assert_eq!(
            ElementId::new(value).as_ref().map(ElementId::as_str),
            Ok(value)
        );
        assert_eq!(
            ElementId::from_static(value)
                .as_ref()
                .map(ElementId::as_str),
            Ok(value)
        );
        assert_eq!(
            ElementKey::new(value).as_ref().map(ElementKey::as_str),
            Ok(value)
        );
        assert_eq!(
            ElementKey::from_static(value)
                .as_ref()
                .map(ElementKey::as_str),
            Ok(value)
        );
    }
    assert_eq!(element_id!("fenster.öffnen").as_str(), "fenster.öffnen");
    assert_eq!(element_key!("画面.開始").as_str(), "画面.開始");

    let valid = button::<()>("Open")
        .id(element_id!("контрол.кнопка"))
        .key(element_key!("画面.開始"))
        .into_element();
    assert!(valid.authoring_diagnostics().is_empty());
}
