use runenui_core::{
    Color, EdgeInsets, IntoEffects, LogicalLength, NoHostProtocol, StyleEnvironment,
    StylePreferenceKind, StylePreferencePolicy, StylePreferences, StyleProperties,
    StyleResolutionLayer, UiApp, View, text,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext, SurfacePhase};

struct PreferenceApp;

impl UiApp for PreferenceApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        text("preferences").foreground(Color::BLACK).key("root")
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

fn publish_initial(runtime: &mut AppRuntime<PreferenceApp>, environment: &StyleEnvironment) {
    let context = SurfaceBuildContext::new(environment, LayoutConstraints::unbounded());
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("preference proof initial publication is admitted"));
}

#[test]
fn reduced_motion_change_invalidates_style_only_until_motion_properties_exist() {
    let mut runtime = AppRuntime::<PreferenceApp>::mount(());
    let initial = StyleEnvironment::default();
    publish_initial(&mut runtime, &initial);

    let reduced_motion =
        StyleEnvironment::default().with_preferences(StylePreferences::new(false, true));
    let context = SurfaceBuildContext::new(&reduced_motion, LayoutConstraints::unbounded());
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("reduced-motion publication is admitted"));

    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Style]
    );
    assert_eq!(
        publication
            .frame()
            .root()
            .unwrap_or_else(|| unreachable!("preference proof has a root"))
            .computed_style()
            .foreground(),
        Some(Color::BLACK)
    );
}

#[test]
fn high_contrast_paint_override_invalidates_only_style_and_paint() {
    let mut runtime = AppRuntime::<PreferenceApp>::mount(());
    let initial = StyleEnvironment::default();
    publish_initial(&mut runtime, &initial);

    let high_contrast = StyleEnvironment::default()
        .with_preferences(StylePreferences::new(true, false))
        .with_preference_policy(
            StylePreferencePolicy::new()
                .with_high_contrast(StyleProperties::EMPTY.with_foreground(Color::WHITE)),
        );
    let context = SurfaceBuildContext::new(&high_contrast, LayoutConstraints::unbounded());
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("high-contrast paint publication is admitted"));

    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Style, SurfacePhase::Paint]
    );
    let root = publication
        .style_report()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("preference proof has a style root"));
    assert_eq!(root.computed_style().foreground(), Some(Color::WHITE));
    assert_eq!(
        root.provenance().foreground_layer(),
        Some(&StyleResolutionLayer::Preference(
            StylePreferenceKind::HighContrast
        ))
    );
}

#[test]
fn high_contrast_layout_override_invalidates_every_required_layout_dependent() {
    let mut runtime = AppRuntime::<PreferenceApp>::mount(());
    let initial = StyleEnvironment::default();
    publish_initial(&mut runtime, &initial);

    let padding = EdgeInsets::all(LogicalLength::from(4_u16));
    let high_contrast = StyleEnvironment::default()
        .with_preferences(StylePreferences::new(true, false))
        .with_preference_policy(
            StylePreferencePolicy::new()
                .with_high_contrast(StyleProperties::EMPTY.with_padding(padding)),
        );
    let context = SurfaceBuildContext::new(&high_contrast, LayoutConstraints::unbounded());
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("high-contrast layout publication is admitted"));

    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[
            SurfacePhase::Style,
            SurfacePhase::Layout,
            SurfacePhase::HitTesting,
            SurfacePhase::Paint,
            SurfacePhase::Semantics,
        ]
    );
    let root = publication
        .style_report()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("preference proof has a style root"));
    assert_eq!(root.computed_style().padding(), Some(padding));
    assert_eq!(
        root.provenance().padding_layer(),
        Some(&StyleResolutionLayer::Preference(
            StylePreferenceKind::HighContrast
        ))
    );
}
