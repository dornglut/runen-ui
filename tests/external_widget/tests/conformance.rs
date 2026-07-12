use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Color, Element, ElementId, StyleTokens, View, WidgetLifecycle, WidgetLifecycleContext,
    WidgetLifecycleRequest, WidgetTypeId, text,
};
use runenui_external_widget_conformance::{
    ChildAction, ConformanceApp, CustomColumn, GenericWidget, LayoutCase, LayoutConformanceApp,
    LayoutState, ParentAction, PulseButton, UnsupportedMeasure, child_component,
    counting_measurement_tree, diagnostic_panel, layout_case_view, parent_view,
};
use runenui_runtime::{
    ActivationResult, AppRuntime, DuplicateIdentityKind, LayoutConstraints, LogicalPoint,
    LogicalSize, MeasurementProvider, RuntimeNodeRef, SurfaceBuildContext, TextMeasurement,
    TextMeasurementKind, TextMeasurementRequest, UiApp, publish_surface,
    render_debug_surface_frame,
};

fn size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::try_new(width, height).unwrap_or_else(|_| unreachable!())
}

#[test]
fn downstream_widget_identity_is_concrete_generic_and_mapping_stable() {
    let first = child_component();
    let second = child_component();
    assert_eq!(first.widget_type_id(), second.widget_type_id());
    assert_eq!(first.widget_type_id(), WidgetTypeId::of::<PulseButton>());

    let different: Element<()> = Element::new(GenericWidget(1_u8));
    let other_generic: Element<()> = Element::new(GenericWidget(1_u16));
    assert_eq!(different.semantics(), other_generic.semantics());
    assert_ne!(different.widget_type_id(), other_generic.widget_type_id());
    assert_ne!(first.widget_type_id(), different.widget_type_id());

    let before = child_component().widget_type_id();
    let mapped = child_component().map_action(ParentAction::Child);
    assert_eq!(mapped.widget_type_id(), before);
    assert_eq!(
        mapped.element_id().map(ElementId::as_str),
        Some("external.pulse")
    );
    assert_eq!(
        mapped.element_key().map(runenui_core::ElementKey::as_str),
        Some("pulse-key")
    );
    assert!(mapped.style().padding().is_some());
    assert_eq!(
        mapped.widget_diagnostics()[0].code(),
        "external.pulse.ready"
    );
}

#[test]
fn stateless_widget_state_contract_is_safe_and_empty() -> Result<(), Box<dyn std::error::Error>> {
    let widget: Element<()> = Element::new(GenericWidget("stateless"));
    let mut state = widget.create_widget_state();
    assert_eq!(
        state.state_type_id(),
        runenui_core::WidgetStateTypeId::of::<()>()
    );
    let mut context = WidgetLifecycleContext::new();
    widget.run_lifecycle(&mut state, WidgetLifecycle::Mount, &mut context)?;
    assert!(context.requests().is_empty());
    Ok(())
}

#[test]
fn downstream_state_and_lifecycle_seam_is_checked_and_ordered()
-> Result<(), Box<dyn std::error::Error>> {
    let widget = child_component();
    let mut state = widget.create_widget_state();
    let mut context = WidgetLifecycleContext::new();
    for event in [
        WidgetLifecycle::Mount,
        WidgetLifecycle::Update,
        WidgetLifecycle::Unmount,
    ] {
        widget.run_lifecycle(&mut state, event, &mut context)?;
    }
    let lifecycle_messages: Vec<_> = context
        .requests()
        .iter()
        .filter_map(|request| match request {
            WidgetLifecycleRequest::Diagnostic(diagnostic) => Some(diagnostic.message()),
            _ => None,
        })
        .collect();
    assert_eq!(lifecycle_messages, ["1:Mount", "2:Update", "3:Unmount"]);

    let built_in: Element<()> = text("different state").into_element();
    let mismatch = built_in.run_lifecycle(
        &mut state,
        WidgetLifecycle::Mount,
        &mut WidgetLifecycleContext::new(),
    );
    assert!(mismatch.is_err());
    Ok(())
}

#[test]
fn custom_widget_participates_in_runtime_layout_paint_semantics_diagnostics_and_actions()
-> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = AppRuntime::<ConformanceApp>::mount(0);
    let id = ElementId::new("external.pulse")?;
    let node = runtime
        .index()
        .node_by_authored_id(&id)
        .map(RuntimeNodeRef::id)
        .ok_or("custom node")?;
    assert!(
        runtime
            .index()
            .node(node)
            .is_some_and(RuntimeNodeRef::is_focusable)
    );

    assert_eq!(runtime.activate_node(node), ActivationResult::Dispatched);
    assert_eq!(*runtime.state(), 1);

    let tokens = StyleTokens::new();
    let publication = runtime.publish_surface(&SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::loose(size(300.0, 200.0)),
    ));
    let custom = publication
        .frame()
        .nodes()
        .iter()
        .find(|surface| {
            surface
                .authored_id()
                .is_some_and(|id| id.as_str() == "external.pulse")
        })
        .ok_or("custom surface")?;
    assert!((custom.bounds().width() - 84.0).abs() <= f32::EPSILON);
    assert!((custom.bounds().height() - 28.0).abs() <= f32::EPSILON);
    assert_eq!(custom.paint().category(), "pulse");
    assert_eq!(custom.semantics().role(), "pulse-button");
    assert_eq!(custom.semantics().action_intent(), Some("pulse"));
    assert_eq!(custom.diagnostics()[0].code(), "external.pulse.ready");
    assert!(render_debug_surface_frame(publication.frame()).contains("paint=pulse"));
    Ok(())
}

#[test]
fn disabled_external_widget_does_not_activate() {
    let mut element: Element<_> = Element::new(PulseButton::new("Disabled").disabled());
    assert!(!element.activation().enabled());
    assert_eq!(element.activate(), None);
}

#[test]
fn mapped_component_preserves_tree_style_and_nested_mapping() {
    #[derive(Debug, Eq, PartialEq)]
    enum OuterAction {
        Parent(ParentAction),
    }

    let root = parent_view()
        .background(Color::BLACK)
        .map_action(OuterAction::Parent);
    assert_eq!(root.children().len(), 3);
    assert_eq!(
        root.children()[1].element_id().map(ElementId::as_str),
        Some("external.pulse")
    );
    assert_eq!(
        root.style()
            .background()
            .and_then(runenui_core::ColorValue::as_literal),
        Some(&Color::BLACK)
    );
    let mut mapped_child = child_component()
        .map_action(ParentAction::Child)
        .map_action(OuterAction::Parent);
    assert_eq!(
        mapped_child.activate(),
        Some(OuterAction::Parent(ParentAction::Child(ChildAction::Pulse)))
    );
    assert_eq!(mapped_child.activate(), None);
}

#[test]
fn external_child_layout_widget_owns_and_publishes_a_recursive_tree()
-> Result<(), Box<dyn std::error::Error>> {
    struct DiagnosticApp;
    impl UiApp for DiagnosticApp {
        type State = usize;
        type Action = ParentAction;
        fn root(_: &Self::State) -> Element<Self::Action> {
            diagnostic_panel()
        }
        fn update(state: &mut Self::State, action: Self::Action) {
            if action == ParentAction::Child(ChildAction::Pulse) {
                *state += 1;
            }
        }
    }

    let root = diagnostic_panel();
    assert_eq!(root.widget_type_id(), WidgetTypeId::of::<CustomColumn>());
    assert_eq!(root.children().len(), 4);
    assert_eq!(root.children()[1].children().len(), 2);
    let mut runtime = AppRuntime::<DiagnosticApp>::mount(0);
    let panel_id = ElementId::new("external.diagnostic-panel")?;
    let panel_node = runtime
        .index()
        .node_by_authored_id(&panel_id)
        .map(RuntimeNodeRef::id)
        .ok_or("external panel node")?;
    let nested_id = ElementId::new("external.nested")?;
    let index = runtime.index();
    let nested_node = index
        .node_by_authored_id(&nested_id)
        .ok_or("nested action node")?;
    assert_eq!(
        nested_node
            .parent()
            .map(runenui_runtime::RuntimeNodeId::as_usize),
        Some(2)
    );
    assert_eq!(runtime.activate(nested_id), ActivationResult::Dispatched);
    assert_eq!(*runtime.state(), 1);

    let index = runtime.index();
    let diagnostics = index.diagnostics();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.kind() == DuplicateIdentityKind::ElementId
            && diagnostic.value() == "external.duplicate"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.kind() == DuplicateIdentityKind::SiblingKey
            && diagnostic.value() == "duplicate-key"
    }));

    let tokens = StyleTokens::new();
    let publication = runtime.publish_surface(&SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::loose(size(300.0, 200.0)),
    ));
    let panel = &publication.frame().nodes()[panel_node.as_usize()];
    assert_eq!(panel.paint().category(), "external-panel");
    assert_eq!(panel.semantics().role(), "group");
    assert_eq!(panel.diagnostics()[0].code(), "external.panel.ready");
    assert!(publication.frame().nodes().iter().any(|node| {
        node.authored_id()
            .is_some_and(|id| id.as_str() == "external.nested")
    }));
    Ok(())
}

#[test]
fn measurement_capability_is_snapshotted_once_per_node_and_publication() {
    struct ControlLabelProvider(Cell<Option<TextMeasurementKind>>);
    impl MeasurementProvider for ControlLabelProvider {
        fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
            self.0.set(Some(request.kind()));
            TextMeasurement::new(size(144.0, 20.0))
        }
    }

    let panel_calls = Rc::new(Cell::new(0));
    let child_layout_calls = Rc::new(Cell::new(0));
    let text_calls = Rc::new(Cell::new(0));
    let fixed_calls = Rc::new(Cell::new(0));
    let tree = counting_measurement_tree(
        Rc::clone(&panel_calls),
        Rc::clone(&child_layout_calls),
        Rc::clone(&text_calls),
        Rc::clone(&fixed_calls),
    );
    let tokens = StyleTokens::new();
    let provider = ControlLabelProvider(Cell::new(None));
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(400.0, 200.0)))
        .with_measurement_provider(&provider);

    let first = publish_surface(&tree, &context);
    assert_eq!(
        (panel_calls.get(), text_calls.get(), fixed_calls.get()),
        (1, 1, 1)
    );
    assert_eq!(child_layout_calls.get(), 1);
    assert_eq!(provider.0.get(), Some(TextMeasurementKind::ControlLabel));
    assert!((first.frame().size().width() - 144.0).abs() <= f32::EPSILON);
    assert!((first.frame().size().height() - 27.0).abs() <= f32::EPSILON);
    assert!((first.frame().nodes()[2].bounds().x() - 0.0).abs() <= f32::EPSILON);
    assert!((first.frame().nodes()[2].bounds().y() - 20.0).abs() <= f32::EPSILON);

    let second = publish_surface(&tree, &context);
    assert_eq!(
        (panel_calls.get(), text_calls.get(), fixed_calls.get()),
        (2, 2, 2)
    );
    assert_eq!(child_layout_calls.get(), 2);
    assert!((second.frame().size().width() - 164.0).abs() <= f32::EPSILON);
    assert!((second.frame().size().height() - 20.0).abs() <= f32::EPSILON);
    assert!((second.frame().nodes()[2].bounds().x() - 144.0).abs() <= f32::EPSILON);
    assert!((second.frame().nodes()[2].bounds().y() - 0.0).abs() <= f32::EPSILON);
}

#[test]
fn unsupported_measurement_has_an_explicit_deterministic_diagnostic()
-> Result<(), Box<dyn std::error::Error>> {
    let tree: Element<()> = Element::new(UnsupportedMeasure);
    let tokens = StyleTokens::new();
    let publication = publish_surface(
        &tree,
        &SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(100.0, 100.0))),
    );
    let diagnostic = publication
        .layout_report()
        .root()
        .and_then(|node| node.diagnostics().first())
        .ok_or("unsupported measurement diagnostic")?;
    assert_eq!(diagnostic.code(), "runenui.measurement.unsupported");
    assert_eq!(
        diagnostic.message(),
        "unsupported widget measurement capability: external proof capability"
    );
    assert_eq!(publication.frame().size(), size(0.0, 0.0));
    Ok(())
}

#[test]
fn every_valid_child_layout_tree_has_aligned_publication_products_and_hits()
-> Result<(), Box<dyn std::error::Error>> {
    for case in [
        LayoutCase::BuiltInColumn,
        LayoutCase::ExternalColumn,
        LayoutCase::ExternalRow,
        LayoutCase::FixedMinimum,
        LayoutCase::TextMinimum,
        LayoutCase::UnsupportedMinimum,
        LayoutCase::NestedExternal,
    ] {
        let mut runtime = AppRuntime::<LayoutConformanceApp>::mount(LayoutState {
            case,
            activations: 0,
        });
        let tokens = StyleTokens::new();
        let publication = runtime.publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::loose(size(600.0, 400.0)),
        ));
        let index = runtime.index();
        let indexed: Vec<_> = index
            .nodes()
            .iter()
            .map(|node| (node.id(), node.parent()))
            .collect();
        let framed: Vec<_> = publication
            .frame()
            .nodes()
            .iter()
            .map(|node| (node.id(), node.parent()))
            .collect();
        let styled: Vec<_> = publication
            .style_report()
            .nodes()
            .iter()
            .map(|node| (node.id(), node.parent()))
            .collect();
        let laid_out: Vec<_> = publication
            .layout_report()
            .nodes()
            .iter()
            .map(|node| (node.id(), node.parent()))
            .collect();
        assert_eq!(framed, indexed, "frame alignment for {case:?}");
        assert_eq!(styled, indexed, "style alignment for {case:?}");
        assert_eq!(laid_out, indexed, "layout alignment for {case:?}");

        let root_layout = publication
            .layout_report()
            .root()
            .ok_or("root layout result")?;
        match case {
            LayoutCase::FixedMinimum => {
                assert!(root_layout.desired_content_size().width() >= 180.0);
                assert!(root_layout.desired_content_size().height() >= 60.0);
            }
            LayoutCase::TextMinimum => {
                assert!(root_layout.desired_content_size().width() >= 200.0);
                assert!(root_layout.desired_content_size().height() >= 20.0);
            }
            LayoutCase::UnsupportedMinimum => {
                assert!(root_layout.desired_content_size().width() > 0.0);
                assert!(root_layout.desired_content_size().height() > 0.0);
            }
            _ => {}
        }

        let action_id = ElementId::new("layout.action")?;
        let action_node = index
            .node_by_authored_id(&action_id)
            .map(RuntimeNodeRef::id)
            .ok_or("actionable descendant")?;
        let action_surface = publication
            .frame()
            .node(action_node)
            .ok_or("framed actionable descendant")?;
        let bounds = action_surface.bounds();
        let point = LogicalPoint::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() / 2.0,
        )?;
        assert_eq!(publication.frame().hit_test_id(point), Some(action_node));
        assert_eq!(
            runtime.activate_node(action_node),
            ActivationResult::Dispatched
        );
        assert_eq!(runtime.state().activations, 1);

        if case == LayoutCase::UnsupportedMinimum {
            assert_eq!(
                Some({
                    let node = root_layout;
                    node.diagnostics()
                        .iter()
                        .map(runenui_core::WidgetDiagnostic::code)
                        .collect::<Vec<_>>()
                }),
                Some(vec!["runenui.measurement.unsupported"])
            );
        }
    }
    Ok(())
}

#[test]
fn external_and_nested_container_gaps_affect_arrangement() {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(600.0, 400.0)));

    let vertical = layout_case_view(LayoutCase::ExternalColumn);
    let publication = publish_surface(&vertical, &context);
    let label = publication.frame().nodes()[1].bounds();
    let action = publication.frame().nodes()[2].bounds();
    assert!((action.y() - label.max_y() - 5.0).abs() <= f32::EPSILON);

    let horizontal = layout_case_view(LayoutCase::ExternalRow);
    let publication = publish_surface(&horizontal, &context);
    let label = publication.frame().nodes()[1].bounds();
    let action = publication.frame().nodes()[2].bounds();
    assert!((action.x() - label.max_x() - 7.0).abs() <= f32::EPSILON);

    let nested = layout_case_view(LayoutCase::NestedExternal);
    assert!((nested.layout().gap().get() - 13.0).abs() <= f32::EPSILON);
    assert!((nested.children()[1].layout().gap().get() - 11.0).abs() <= f32::EPSILON);
    let publication = publish_surface(&nested, &context);
    let head = publication.frame().nodes()[1].bounds();
    let nested_row = publication.frame().nodes()[2].bounds();
    let action = publication.frame().nodes()[3].bounds();
    let tail = publication.frame().nodes()[4].bounds();
    assert!((nested_row.y() - head.max_y() - 13.0).abs() <= f32::EPSILON);
    assert!((tail.x() - action.max_x() - 11.0).abs() <= f32::EPSILON);
}
