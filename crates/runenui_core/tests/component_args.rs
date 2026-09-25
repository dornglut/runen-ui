use runenui_core::{
    HitContributionContext, LogicalSize, SemanticCheckedState, SemanticContributionContext,
    SemanticRole, View, WidgetAvailableSpace, WidgetMeasure, WidgetMeasureInput, button, checkbox,
    children, column, switch, text,
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
    let (_, _, _, _, _, _, _, _, text_widget, _) = text_element.into_runtime_parts().into_parts();
    let text_state = text_widget.create_state();
    assert!(matches!(
        text_widget.measure(
            &text_state,
            WidgetMeasureInput::new(
                None,
                None,
                WidgetAvailableSpace::MaxContent,
                WidgetAvailableSpace::MaxContent,
            ),
        ),
        Ok(WidgetMeasure::Text { .. })
    ));
    let (_, _, _, _, _, _, _, _, button_widget, _) =
        button_element.into_runtime_parts().into_parts();
    let button_state = button_widget.create_state();
    let semantics = button_widget
        .semantics(&button_state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(semantics.roots().len(), 1);
    let button_node = semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("button contributes one semantic node"));
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
    assert!((container.layout().gap().horizontal().get() - 8.0).abs() <= f32::EPSILON);
    assert!((container.layout().gap().vertical().get() - 8.0).abs() <= f32::EPSILON);
    assert!(matches!(
        container.layout().container(),
        runenui_core::LayoutContainer::Flex(_)
    ));
    let (_, _, _, _, _, _, _, _, widget, _) = container.into_runtime_parts().into_parts();
    let state = widget.create_state();
    assert_eq!(
        widget
            .measure(
                &state,
                WidgetMeasureInput::new(
                    None,
                    None,
                    WidgetAvailableSpace::MaxContent,
                    WidgetAvailableSpace::MaxContent,
                ),
            )
            .unwrap_or_else(|_| unreachable!()),
        WidgetMeasure::default()
    );
}

#[test]
fn binary_control_builders_use_the_open_widget_protocol() {
    assert_eq!(
        SemanticCheckedState::from(false),
        SemanticCheckedState::Unchecked
    );
    assert_eq!(
        SemanticCheckedState::from(true),
        SemanticCheckedState::Checked
    );

    let checkbox_element: runenui_core::Element<Action> =
        checkbox("Tri-state", SemanticCheckedState::Mixed)
            .id("tri-state")
            .disabled()
            .on_activate(|| Action::Save)
            .into_element();
    let (_, _, _, _, _, _, _, _, checkbox_widget, _) =
        checkbox_element.into_runtime_parts().into_parts();
    let checkbox_state = checkbox_widget.create_state();
    let checkbox_semantics = checkbox_widget
        .semantics(&checkbox_state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("checkbox semantics are valid"));
    let checkbox_node = checkbox_semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("checkbox contributes one semantic node"));
    assert_eq!(checkbox_node.role(), SemanticRole::Checkbox);
    assert_eq!(checkbox_node.name(), Some("Tri-state"));
    assert_eq!(
        checkbox_node.state().checked(),
        Some(SemanticCheckedState::Mixed)
    );
    assert!(checkbox_node.state().disabled());

    let switch_element: runenui_core::Element<Action> = switch("Power", true)
        .id("power")
        .on_activate(|| Action::Save)
        .into_element();
    let (_, _, _, _, _, _, _, _, switch_widget, _) =
        switch_element.into_runtime_parts().into_parts();
    let switch_state = switch_widget.create_state();
    let switch_semantics = switch_widget
        .semantics(&switch_state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("switch semantics are valid"));
    let switch_node = switch_semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("switch contributes one semantic node"));
    assert_eq!(switch_node.role(), SemanticRole::Switch);
    assert_eq!(switch_node.name(), Some("Power"));
    assert_eq!(
        switch_node.state().checked(),
        Some(SemanticCheckedState::Checked)
    );

    let passive_element: runenui_core::Element<Action> = checkbox("Passive", false).into_element();
    let (_, _, _, _, _, _, _, _, passive_widget, _) =
        passive_element.into_runtime_parts().into_parts();
    let passive_state = passive_widget.create_state();
    let passive_activation = passive_widget
        .activation(&passive_state)
        .unwrap_or_else(|_| unreachable!("passive checkbox activation is inspectable"));
    assert!(!passive_activation.is_actionable());
    assert!(
        passive_widget
            .hit_test(
                &passive_state,
                HitContributionContext::__runtime_new(LogicalSize::ZERO),
            )
            .unwrap_or_else(|_| unreachable!("passive checkbox hit contribution is inspectable"))
            .is_empty()
    );

    let actionable_element: runenui_core::Element<Action> = switch("Actionable", false)
        .on_activate(|| Action::Save)
        .into_element();
    let (_, _, _, _, _, _, _, _, actionable_widget, _) =
        actionable_element.into_runtime_parts().into_parts();
    let actionable_state = actionable_widget.create_state();
    let actionable_activation = actionable_widget
        .activation(&actionable_state)
        .unwrap_or_else(|_| unreachable!("actionable switch activation is inspectable"));
    assert!(actionable_activation.is_actionable());
    assert!(actionable_activation.enabled());
    assert!(
        !actionable_widget
            .hit_test(
                &actionable_state,
                HitContributionContext::__runtime_new(LogicalSize::ZERO),
            )
            .unwrap_or_else(|_| unreachable!("actionable switch hit contribution is inspectable"))
            .is_empty()
    );
}
