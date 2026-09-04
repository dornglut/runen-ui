#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    CommandOrigin, Element, ElementId, FontFamilyName, GenericFontFamily, NoHostProtocol,
    SemanticCommand, StyleEnvironment, UiApp,
};
use runenui_external_widget_conformance::{
    LayoutCase, LayoutConformanceApp, LayoutState, UnsupportedMeasure, counting_measurement_tree,
    responsive_measurement_tree,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, LogicalPoint, LogicalSize, PumpBudget, SurfaceBuildContext,
    SurfacePublication,
};

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));

fn size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::try_new(width, height).unwrap_or_else(|_| unreachable!())
}

struct ResponsiveApp;

impl UiApp for ResponsiveApp {
    type State = Rc<RefCell<Vec<runenui_core::WidgetMeasureInput>>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        responsive_measurement_tree(Rc::clone(state))
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn downstream_custom_measurement_receives_bounded_requests_and_baseline() {
    let inputs = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<ResponsiveApp>::mount(Rc::clone(&inputs));
    let environment = StyleEnvironment::default();
    let publication = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&environment, LayoutConstraints::loose(size(120.0, 80.0))),
    );

    let observed = inputs.borrow();
    assert!(!observed.is_empty());
    assert!(observed.iter().all(|input| {
        !matches!(
            (input.available_width(), input.available_height()),
            (runenui_core::WidgetAvailableSpace::Definite(width), _)
                if !width.get().is_finite()
        )
    }));
    assert!(
        observed
            .iter()
            .any(|input| input.known_width().is_some() || input.known_height().is_some())
    );
    let node = publication
        .layout_report()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "responsive.measure")
        })
        .unwrap_or_else(|| unreachable!("responsive measured node is published"));
    assert!(node.constrained_outer_size().width() > 0.0);
}

fn register_controlled_text<App: UiApp>(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .register_text_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|_| unreachable!("controlled Cantarell fixture is registerable"))
            > 0
    );
    let family = FontFamilyName::new("Cantarell")
        .unwrap_or_else(|_| unreachable!("controlled family name is canonical"));
    assert!(
        runtime
            .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
            .unwrap_or_else(|_| unreachable!("controlled generic mapping is valid"))
    );
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
fn measurement_callbacks_are_transaction_local_and_clean_publication_reuses_products() {
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
    register_controlled_text(&mut runtime);
    let environment = StyleEnvironment::default();
    let context =
        SurfaceBuildContext::new(&environment, LayoutConstraints::loose(size(400.0, 200.0)));

    let first = publish(&mut runtime, &context);
    assert_eq!(
        (panel.get(), text.get(), fixed.get(), layout.get()),
        (1, 1, 1, 0)
    );
    assert!(first.frame().size().width() > 0.0);
    assert!(first.frame().size().height() > 0.0);
    let first_context = first.input_context().clone();
    let first_paint = first.paint_publication().clone();
    let first_hit_regions = first.hit_test_scene().regions().to_vec();
    let first_membership = first.hit_test_scene().mounted_targets().to_vec();
    let first_products = first.clone().into_renderer_products();

    let second = publish(&mut runtime, &context);
    assert_eq!(
        second.input_context().surface_id(),
        first_context.surface_id()
    );
    assert!(second.input_context().coordinate_revision() > first_context.coordinate_revision());
    assert!(second.input_context().hit_test_generation() > first_context.hit_test_generation());
    assert_eq!(second.paint_publication(), &first_paint);
    assert_eq!(
        second.hit_test_scene().regions(),
        first_hit_regions.as_slice()
    );
    assert_eq!(
        second.hit_test_scene().mounted_targets(),
        first_membership.as_slice()
    );
    assert_ne!(second.hit_test_scene(), first.hit_test_scene());
    assert!(second.renderer_products_eq(&first));
    assert_eq!(second.into_renderer_products(), first_products);
    assert_eq!(
        (panel.get(), text.get(), fixed.get(), layout.get()),
        (1, 1, 1, 0)
    );
    assert!(runtime.last_surface_phase_report().executed().is_empty());

    assert!(
        runtime
            .register_text_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|_| unreachable!("controlled source can advance font revision"))
            > 0
    );
    let revised = publish(&mut runtime, &context);
    assert!(revised.frame().size().width() > 0.0);
    assert!(revised.frame().size().height() > 0.0);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[
            runenui_runtime::SurfacePhase::Layout,
            runenui_runtime::SurfacePhase::HitTesting,
            runenui_runtime::SurfacePhase::Paint,
            runenui_runtime::SurfacePhase::Semantics,
        ]
    );
    assert_eq!(
        (panel.get(), text.get(), fixed.get(), layout.get()),
        (2, 2, 2, 0)
    );
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
    let environment = StyleEnvironment::default();
    let context =
        SurfaceBuildContext::new(&environment, LayoutConstraints::loose(size(100.0, 100.0)));
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
    assert_eq!(publication.frame().size(), size(100.0, 0.0));
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
        register_controlled_text(&mut runtime);
        settle_initial_mounted_declarations(&mut runtime);
        let environment = StyleEnvironment::default();
        let context =
            SurfaceBuildContext::new(&environment, LayoutConstraints::loose(size(600.0, 400.0)));
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
        assert_eq!(publication.hit_test_scene().target_at(point), Some(&action));
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
        register_controlled_text(&mut runtime);
        let environment = StyleEnvironment::default();
        let context =
            SurfaceBuildContext::new(&environment, LayoutConstraints::loose(size(600.0, 400.0)));
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
    register_controlled_text(&mut runtime);
    let environment = StyleEnvironment::default();
    let context =
        SurfaceBuildContext::new(&environment, LayoutConstraints::loose(size(600.0, 400.0)));
    let publication = publish(&mut runtime, &context);
    let nodes = publication.frame().nodes();
    assert!((nodes[2].bounds().y() - nodes[1].bounds().max_y() - 13.0).abs() <= f32::EPSILON);
    assert!((nodes[4].bounds().x() - nodes[3].bounds().max_x() - 11.0).abs() <= f32::EPSILON);
}
