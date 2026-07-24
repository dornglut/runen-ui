use runenui_core::{View, WidgetActivationContext, button};

#[derive(Debug, Eq, PartialEq)]
enum Action {
    Submit,
}

#[test]
fn disabled_button_preserves_action_and_can_be_reenabled() {
    let element = button("Submit")
        .on_activate(|| Action::Submit)
        .disabled()
        .enabled(true)
        .into_element();
    let (_, _, _, _, _, _, _, mut widget, _) = element.into_runtime_parts().into_parts();
    let mut state = widget.create_state();
    assert!(
        widget
            .activation(&state)
            .unwrap_or_else(|_| unreachable!())
            .enabled()
    );
    let mut context = WidgetActivationContext::__runtime_new();
    let first = widget
        .activate(&mut state, &mut context)
        .unwrap_or_else(|_| unreachable!());
    assert!(first.state_changed());
    assert_eq!(first.into_action(), Some(Action::Submit));
    let second = widget
        .activate(&mut state, &mut WidgetActivationContext::__runtime_new())
        .unwrap_or_else(|_| unreachable!());
    assert!(second.state_changed());
    assert_eq!(second.into_action(), Some(Action::Submit));
}

#[test]
fn nested_mapping_preserves_non_clone_action_and_state_identity() {
    #[derive(Debug, Eq, PartialEq)]
    enum Child {
        Submit,
    }
    #[derive(Debug, Eq, PartialEq)]
    enum Parent {
        Child(Child),
    }
    let original = button("Submit")
        .on_activate(|| Child::Submit)
        .into_element();
    let original_type = original.into_runtime_parts().into_parts().7.state_type_id();
    let mapped = button("Submit")
        .on_activate(|| Child::Submit)
        .into_element()
        .map_action(Parent::Child);
    let (_, _, _, _, _, _, _, mut widget, _) = mapped.into_runtime_parts().into_parts();
    assert_eq!(widget.state_type_id(), original_type);
    let mut state = widget.create_state();
    let first = widget
        .activate(&mut state, &mut WidgetActivationContext::__runtime_new())
        .unwrap_or_else(|_| unreachable!());
    assert!(first.state_changed());
    assert_eq!(first.into_action(), Some(Parent::Child(Child::Submit)));
    let second = widget
        .activate(&mut state, &mut WidgetActivationContext::__runtime_new())
        .unwrap_or_else(|_| unreachable!());
    assert!(second.state_changed());
    assert_eq!(second.into_action(), Some(Parent::Child(Child::Submit)));
}
