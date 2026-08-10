use runenui_core::{
    Axis, ChildLayout, SemanticContributionContext, SemanticItem, SemanticRole, View, WidgetMeasure,
    button, children, column, text,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Save,
}

#[test]
fn typed_builders_use_the_open_widget_protocol() {
    let text_element: runenui_core::Element<Action> = text("Title").id("title").into_element();
    let button_element = button("Save")
        .id("save")
        .disabled()
        .on_activate(|| Action::Save)
        .into_element();
    let (_, _, _, _, _, _, _, text_widget, _) = text_element.into_runtime_parts().into_parts();
    let text_state = text_widget.create_state();
    assert!(matches!(
        text_widget.measure(&text_state),
        Ok(WidgetMeasure::Text { .. })
    ));
    let (_, _, _, _, _, _, _, button_widget, _) = button_element.into_runtime_parts().into_parts();
    let button_state = button_widget.create_state();
    let semantics = button_widget
        .semantics(&button_state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!());
    let [SemanticItem::Node(button_node)] = semantics.roots() else {
        panic!("button contributes exactly one semantic node");
    };
    assert_eq!(button_node.role(), SemanticRole::Button);
    assert_eq!(button_node.name(), Some("Save"));
    assert!(button_node.state().disabled());

    let text = text("Title").id("title").into_element();
    let button = button("Save")
        .id("save")
        .disabled()
        .on_activate(|| Action::Save)
        .into_element();
    let container = column(children![text, button]).gap(8_u16).into_element();
    assert_eq!(container.children().len(), 2);
    assert!((container.layout().gap().get() - 8.0).abs() <= f32::EPSILON);
    let (_, _, _, _, _, _, _, widget, _) = container.into_runtime_parts().into_parts();
    let state = widget.create_state();
    assert_eq!(
        widget
            .child_layout(&state)
            .unwrap_or_else(|_| unreachable!()),
        Some(ChildLayout::Linear {
            axis: Axis::Vertical
        })
    );
    assert_eq!(
        widget.measure(&state).unwrap_or_else(|_| unreachable!()),
        WidgetMeasure::default()
    );
}
