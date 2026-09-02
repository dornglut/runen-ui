use runenui_core::{
    ElementId, FontFamilyName, GenericFontFamily, IntoEffects, NoHostProtocol, UiApp, View,
    children, column, text,
};
use runenui_runtime::LogicalSize;
use runenui_testing::{
    DEFAULT_TEST_SURFACE_SIZE, TestHarness, TestSurfaceConfig, TestSurfaceConfigError,
};

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));

struct SurfaceApp;

impl UiApp for SurfaceApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        column(children![text("Measure me").id("surface.measure")])
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

fn register_controlled_text(harness: &mut TestHarness<SurfaceApp>) {
    assert!(
        harness
            .register_text_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|_| unreachable!("controlled Cantarell fixture is registerable"))
            > 0
    );
    let family = FontFamilyName::new("Cantarell")
        .unwrap_or_else(|_| unreachable!("controlled family name is canonical"));
    assert!(
        harness
            .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
            .unwrap_or_else(|_| unreachable!("controlled generic mapping is valid"))
    );
}

#[test]
fn fixed_surface_layout_and_font_backed_text_are_public_and_deterministic() {
    let mut harness = TestHarness::<SurfaceApp>::mount(());
    assert_eq!(harness.surface_config().size(), DEFAULT_TEST_SURFACE_SIZE);
    register_controlled_text(&mut harness);

    let publication = harness
        .publish()
        .unwrap_or_else(|_| unreachable!("controlled text publication succeeds"));
    assert_eq!(publication.frame().size(), DEFAULT_TEST_SURFACE_SIZE);
    let authored = ElementId::new("surface.measure")
        .unwrap_or_else(|_| unreachable!("test authored identifier is canonical"));
    let node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("authored text node is published"));
    assert!(node.bounds().width() > 0.0);
    assert!(node.bounds().height() > 0.0);
    assert!(publication.paint_scene().is_empty());
    assert!(publication.layout_report().node(node.id()).is_some());
    let point =
        runenui_runtime::LogicalPoint::new(node.bounds().x() + 1.0, node.bounds().y() + 1.0)
            .unwrap_or_else(|_| unreachable!("test point is finite"));
    assert_eq!(publication.hit_test_scene().target_at(point), None);
    assert!(
        publication
            .hit_test_scene()
            .contains_mounted_target(node.id())
    );
}

#[test]
fn surface_size_is_explicitly_configurable_and_zero_extent_is_rejected() {
    let zero_width = LogicalSize::try_new(0.0, 240.0).unwrap_or(LogicalSize::ZERO);
    assert_eq!(
        TestSurfaceConfig::new(zero_width),
        Err(TestSurfaceConfigError::ZeroExtent)
    );

    let custom_size = LogicalSize::try_new(320.0, 240.0).unwrap_or(DEFAULT_TEST_SURFACE_SIZE);
    let config = TestSurfaceConfig::new(custom_size)
        .unwrap_or_else(|_| unreachable!("non-zero test surface is valid"));
    let mut harness = TestHarness::<SurfaceApp>::mount(());
    register_controlled_text(&mut harness);
    harness.set_surface_config(config);
    let publication = harness
        .publish()
        .unwrap_or_else(|_| unreachable!("controlled text publication succeeds"));
    assert_eq!(publication.frame().size(), custom_size);
}
