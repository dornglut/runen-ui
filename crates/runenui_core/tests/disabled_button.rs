use runenui_core::{View, button, children, column, text};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Submit,
}

#[test]
fn disabled_button_preserves_action_and_can_be_reenabled() {
    let mut reenabled = button("Submit")
        .on_press(Action::Submit)
        .disabled()
        .enabled(true)
        .into_element();
    assert!(reenabled.activation().enabled());
    assert_eq!(reenabled.activate(), Some(Action::Submit));

    let mut element = button("Submit").on_press(Action::Submit).into_element();
    assert!(element.activation().enabled());
    assert!(element.activation().is_actionable());
    assert_eq!(element.semantics().action_intent(), Some("activate"));
    assert_eq!(element.activate(), Some(Action::Submit));
    assert_eq!(element.activate(), None);
    assert!(element.activation().is_actionable());
    assert!(element.activation().enabled());
    assert_eq!(element.semantics().action_intent(), Some("activate"));

    let mut label: runenui_core::Element<Action> = text("Label").into_element();
    assert!(!label.activation().is_actionable());
    assert_eq!(label.activate(), None);
}

#[test]
fn nested_mapping_extracts_one_non_clone_action_without_hidden_mutation() {
    #[derive(Debug, Eq, PartialEq)]
    enum Child {
        Submit,
    }
    #[derive(Debug, Eq, PartialEq)]
    enum Parent {
        Child(Child),
    }
    #[derive(Debug, Eq, PartialEq)]
    enum Outer {
        Parent(Parent),
    }

    let mut element = button("Submit")
        .on_press(Child::Submit)
        .into_element()
        .map_action(Parent::Child)
        .map_action(Outer::Parent);
    assert_eq!(
        element.activate(),
        Some(Outer::Parent(Parent::Child(Child::Submit)))
    );
    assert_eq!(element.activate(), None);
}

#[test]
fn runtime_preorder_bridge_extracts_only_the_target_action() {
    let mut root = column(children![
        text("before"),
        button("Submit").on_press(Action::Submit),
        text("after"),
    ])
    .into_element();
    assert_eq!(
        root.extract_action_at_preorder_for_runtime(2),
        Some(Action::Submit)
    );
    assert_eq!(root.extract_action_at_preorder_for_runtime(2), None);
}
