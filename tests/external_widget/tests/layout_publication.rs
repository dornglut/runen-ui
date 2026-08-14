#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    CommandOrigin, Element, ElementId, NoHostProtocol, SemanticCommand, StyleTokens, UiApp,
};
use runenui_external_widget_conformance::{
    LayoutCase, LayoutConformanceApp, LayoutState, UnsupportedMeasure, counting_measurement_tree,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, LogicalPoint, LogicalSize, MeasurementProvider, PumpBudget,
    SurfaceBuildContext, SurfacePublication, TextMeasurement, TextMeasurementKind,
    TextMeasurementRequest,
};

fn size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::try_new(width, height).unwrap_or_else(|_| unreachable!())
}

fn settle_initial_mounted_declarations<App: UiApp>(runtime: &mut AppRuntime<App>) {
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn publish<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    context: &SurfaceBuildContext<'_>,
) -> SurfacePublication {
    runtime
        .publish_surface(context)
        .unwrap_or_else(|_| unreachable!("external-widget publication is admitted"))
}

fn submit_layout_activate(
    runtime: &mut AppRuntime<LayoutConformanceApp>,
    target: runenui_core::MountedNodeId,
) {
    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
}

struct ControlLabelProvider {
    kind: Cell<Option<TextMeasurementKind>>,
    calls: Cell<usize>,
    revision: Cell<u64>,
    width: Cell<f32>,
}

impl MeasurementProvider for ControlLabelProvider {
    fn cache_identity(&self) -> u64 {
        0x434f_4e54_524f_4c4c
    }

    fn cache_revision(&self) -> u64 {
        self.revision.get()
    }

    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
        self.kind.set(Some(request.kind()));
        self.calls.set(self.calls.get() + 1);
        TextMeasurement::new(size(self.width.get(), 20.0))
    }
}

#[derive(Debug)]
struct CountingState {
    panel: Rc<Cell<usize>>,
    layout: Rc<Cell<usize>>,
    text: Rc<Cell<usize>>,
    fixed: Rc<Cell<usize>>,
}

struct CountingApp;

impl UiApp for CountingApp {
    type State = CountingState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        counting_measurement_tree(
            Rc::clone(&state.panel),
            Rc::clone(&state.layout),
            Rc::clone(&state.text),
            Rc::clone(&state.fixed),
        )
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn measurement_and_child_layout_capabilities_are_cached_across_clean_publication() {
    let panel = Rc::new(Cell::new(0));
    let layout = Rc::new(Cell::new(0));
    let text = Rc::new(Cell::new(0));
    let fixed = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<CountingApp>::mount(CountingState {
        panel: Rc::clone(&panel),
        layout: Rc::clone(&layout),
        text: Rc::clone(&text),
        fixed: Rc::clone(&fixed),
    });
    let provider = ControlLabelProvider {
        kind: Cell::new(None),
        calls: Cell::new(0),
        revision: Cell::new(1),
        width: Cell::new(144.0),
    };
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(400.0, 200.0)))
        .with_measurement_provider(&provider);

    let first = publish(&mut runtime, &context);
    assert_eq!(
        (panel.get(), text.get(), fixed.get(), layout.get()),
        (1, 1, 1, 1)
    );
    assert_eq!(provider.kind.get(), Some(TextMeasurementKind::ControlLabel));
    assert_eq!(provider.calls.get(), 1);
    assert!((first.frame().size().width() - 144.0).abs() <= f32::EPSILON);
    assert!((first.frame().size().height() - 27.0).abs() <= f32::EPSILON);
    let first_context = first.input_context().clone();
    let first_products = first.clone().into_renderer_products();

    let second = publish(&mut runtime, &context);
    assert_eq!(
        second.input_context().surface_id(),
        first_context.surface_id()
    );
    assert!(second.input_context().coordinate_revision() > first_context.coordinate_revision());
    assert!(second.input_context().hit_test_generation() > first_context.hit_test_generation());
    assert!(second.renderer_products_eq(&first));
    assert_eq!(second, first);
    assert_eq!(second.into_renderer_products(), first_products);
    assert_eq!(
        (panel.get(), text.get(), fixed.get(), layout.get()),
        (1, 1, 1, 1)
    );
    assert_eq!(provider.calls.get(), 1);
    assert!(runtime.last_surface_phase_report().executed().is_empty());

    provider.revision.set(2);
    provider.width.set(200.0);
    let revised = publish(&mut runtime, &context);
    assert!((revised.frame().size().width() - 200.0).abs() <= f32::EPSILON);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[
            runenui_runtime::SurfacePhase::Layout,
            runenui_runtime::SurfacePhase::HitTesting,
            runenui_runtime::SurfacePhase::Semantics,
        ]
    );
    assert_eq!(provider.calls.get(), 2);
}

struct UnsupportedApp;

impl UiApp for UnsupportedApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        Element::new(UnsupportedMeasure).key("unsupported")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn unsupported_measurement_is_explicit_and_deterministic() {
    let mut runtime = AppRuntime::<UnsupportedApp>::mount(());
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(100.0, 100.0)));
    let publication = publish(&mut runtime, &context);
    let diagnostic = &publication
        .layout_report()
        .root()
        .unwrap_or_else(|| unreachable!())
        .diagnostics()[0];
    assert_eq!(diagnostic.code(), "runenui.measurement.unsupported");
    assert_eq!(
        diagnostic.message(),
        "unsupported widget measurement capability: external proof capability"
    );
    assert_eq!(publication.frame().size(), size(0.0, 0.0));
}

#[test]
fn every_child_layout_variant_aligns_mounted_products_hits_and_activation() {
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
        settle_initial_mounted_declarations(&mut runtime);
        let tokens = StyleTokens::new();
        let context =
            SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(600.0, 400.0)));
        let publication = publish(&mut runtime, &context);
        let indexed: Vec<_> = runtime
            .index()
            .nodes()
            .iter()
            .map(|node| (node.id().clone(), node.parent().cloned()))
            .collect();
        let framed: Vec<_> = publication
            .frame()
            .nodes()
            .iter()
            .map(|node| (node.id().clone(), node.parent().cloned()))
            .collect();
        let styled: Vec<_> = publication
            .style_report()
            .nodes()
            .iter()
            .map(|node| (node.id().clone(), node.parent().cloned()))
            .collect();
        let laid_out: Vec<_> = publication
            .layout_report()
            .nodes()
            .iter()
            .map(|node| (node.id().clone(), node.parent().cloned()))
            .collect();
        assert_eq!(framed, indexed, "frame alignment for {case:?}");
        assert_eq!(styled, indexed, "style alignment for {case:?}");
        assert_eq!(laid_out, indexed, "layout alignment for {case:?}");

        let root = publication
            .layout_report()
            .root()
            .unwrap_or_else(|| unreachable!());
        match case {
            LayoutCase::FixedMinimum => {
                assert!(root.desired_content_size().width() >= 180.0);
                assert!(root.desired_content_size().height() >= 60.0);
            }
            LayoutCase::TextMinimum => {
                assert!(root.desired_content_size().width() >= 200.0);
                assert!(root.desired_content_size().height() >= 20.0);
            }
            LayoutCase::UnsupportedMinimum => {
                assert!(root.desired_content_size().width() > 0.0);
                assert_eq!(
                    root.diagnostics()[0].code(),
                    "runenui.measurement.unsupported"
                );
            }
            _ => {}
        }

        let authored = ElementId::new("layout.action").unwrap_or_else(|_| unreachable!());
        let action = runtime
            .index()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&authored))
            .unwrap_or_else(|| unreachable!())
            .id()
            .clone();
        let bounds = publication
            .frame()
            .node(&action)
            .unwrap_or_else(|| unreachable!())
            .bounds();
        let point = LogicalPoint::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() / 2.0,
        )
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(publication.frame().hit_test_id(point), Some(action.clone()));
        submit_layout_activate(&mut runtime, action);
        assert_eq!(
            runtime
                .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
                .processed_envelopes(),
            2
        );
        assert_eq!(runtime.state().activations, 1);
    }
}

#[test]
fn external_and_nested_gaps_affect_arrangement_independently() {
    for (case, axis_gap) in [
        (LayoutCase::ExternalColumn, 5.0),
        (LayoutCase::ExternalRow, 7.0),
    ] {
        let mut runtime = AppRuntime::<LayoutConformanceApp>::mount(LayoutState {
            case,
            activations: 0,
        });
        let tokens = StyleTokens::new();
        let context =
            SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(600.0, 400.0)));
        let publication = publish(&mut runtime, &context);
        let first = publication.frame().nodes()[1].bounds();
        let second = publication.frame().nodes()[2].bounds();
        if case == LayoutCase::ExternalColumn {
            assert!((second.y() - first.max_y() - axis_gap).abs() <= f32::EPSILON);
        } else {
            assert!((second.x() - first.max_x() - axis_gap).abs() <= f32::EPSILON);
        }
    }

    let mut runtime = AppRuntime::<LayoutConformanceApp>::mount(LayoutState {
        case: LayoutCase::NestedExternal,
        activations: 0,
    });
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(600.0, 400.0)));
    let publication = publish(&mut runtime, &context);
    let nodes = publication.frame().nodes();
    assert!((nodes[2].bounds().y() - nodes[1].bounds().max_y() - 13.0).abs() <= f32::EPSILON);
    assert!((nodes[4].bounds().x() - nodes[3].bounds().max_x() - 11.0).abs() <= f32::EPSILON);
}
