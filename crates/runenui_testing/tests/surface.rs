use runenui_core::{ElementId, IntoEffects, NoHostProtocol, UiApp, View, children, column, text};
use runenui_runtime::{
    LogicalSize, MeasurementProvider, TextMeasurement, TextMeasurementRequest,
};
use runenui_testing::{DEFAULT_TEST_SURFACE_SIZE, TestHarness};

struct SurfaceApp;

impl UiApp for SurfaceApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        column(children![text("Measure me").id("surface.measure")])
    }

    fn update(
        _: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[derive(Clone, Copy, Debug)]
struct WideMeasurement;

impl MeasurementProvider for WideMeasurement {
    fn cache_identity(&self) -> u64 {
        0x4d3544
    }

    fn cache_revision(&self) -> u64 {
        1
    }

    fn measure_text(&self, _: &TextMeasurementRequest<'_>) -> TextMeasurement {
        let size = LogicalSize::try_new(240.0, 48.0).unwrap_or(LogicalSize::ZERO);
        TextMeasurement::new(size)
    }
}

#[test]
fn fixed_surface_layout_hit_paint_and_custom_measurement_are_public_and_deterministic() {
    let mut harness = TestHarness::<SurfaceApp>::mount(());
    assert_eq!(harness.surface_config().size(), DEFAULT_TEST_SURFACE_SIZE);

    let Some((default_bounds, measured_id)) = (|| {
        let publication = harness.publish().ok()?;
        assert_eq!(publication.frame().size(), DEFAULT_TEST_SURFACE_SIZE);
        let authored = ElementId::new("surface.measure").ok()?;
        let node = publication
            .frame()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&authored))?;
        assert_ne!(node.paint().category(), "none");
        assert!(publication.layout_report().node(node.id()).is_some());
        let point = runenui_runtime::LogicalPoint::new(
            node.bounds().x() + 1.0,
            node.bounds().y() + 1.0,
        )
        .ok()?;
        assert_eq!(publication.frame().hit_test_id(point), Some(node.id().clone()));
        Some((node.bounds(), node.id().clone()))
    })() else {
        return;
    };

    let Some(custom_bounds) = (|| {
        let publication = harness.publish_with_measurement(&WideMeasurement).ok()?;
        let node = publication.frame().node(&measured_id)?;
        Some(node.bounds())
    })() else {
        return;
    };

    assert!(custom_bounds.width() > default_bounds.width());
    assert!(custom_bounds.height() > default_bounds.height());
}
