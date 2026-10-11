#![allow(refining_impl_trait)]

//! A select-only ComboBox must render its ordinary Option text while M5
//! projects a singular current active descendant without focus movement.
use core::{
    error::Error,
    future::Future,
    pin::pin,
    task::{Context, Poll},
};
use runenui_core::{
    Color, ElementId, FontFamilyName, GenericFontFamily, NoHostProtocol, SemanticRelationshipKind,
    SemanticRole, StyleEnvironment, UiApp, View, combo_box, option_item,
};
use runenui_render_wgpu::{
    Renderer, RendererInitError, RendererOptions, ResourcePayload, ResourceProvider,
    ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{AppRuntime, LogicalSize, SurfaceBuildContext};
use std::{fs, path::PathBuf};

const FONT: &[u8] = include_bytes!("fixtures/Cantarell-Regular.ttf");
struct NoResources;
impl ResourceProvider for NoResources {
    fn load(
        &self,
        _: &runenui_core::ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Malformed,
            "ComboBox needs no external image resources",
        ))
    }
}
struct ComboGpuApp;
impl UiApp for ComboGpuApp {
    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn root(open: &Self::State) -> impl View<Self::Action> {
        combo_box(
            "Choose",
            ElementId::new("gpu.combo.popup").unwrap_or_else(|_| unreachable!()),
            [
                option_item("Alpha", true)
                    .id("gpu.combo.alpha")
                    .foreground(Color::WHITE)
                    .on_activate(|| ()),
                option_item("Bravo", false)
                    .id("gpu.combo.bravo")
                    .foreground(Color::WHITE)
                    .on_activate(|| ()),
            ],
            |_| (),
        )
        .id("gpu.combo.owner")
        .foreground(Color::WHITE)
        .open(*open)
    }
    fn update(_: &mut Self::State, (): Self::Action) {}
}
fn block_on<F: Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut ctx = Context::from_waker(waker);
    let mut future = pin!(future);
    loop {
        match Future::poll(future.as_mut(), &mut ctx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
fn publication(open: bool) -> Result<runenui_runtime::SurfacePublication, Box<dyn Error>> {
    let mut app = AppRuntime::<ComboGpuApp>::mount(open);
    // Reconcile initial authored popup content and current option identity
    // before publishing; there is no fake semantic target or test-only hook.
    assert!(
        app.pump(runenui_runtime::PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .is_quiescent()
    );
    app.register_text_font_bytes(FONT.to_vec())?;
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")?],
    )?;
    app.publish_surface(&SurfaceBuildContext::tight(
        &StyleEnvironment::default(),
        LogicalSize::try_new(400.0, 200.0)?,
    ))
    .map_err(|e| std::io::Error::other(format!("ComboBox surface: {e:?}")))
    .map_err(Into::into)
}
#[test]
fn select_only_combo_popup_projects_real_m8_options_and_real_wgpu_pixels()
-> Result<(), Box<dyn Error>> {
    let required = std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR").is_some()
        || std::env::var_os("RUNENUI_M11COMBO_EVIDENCE_DIR").is_some();
    let mut renderer = match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => renderer,
        Err(RendererInitError::AdapterUnavailable { .. }) if !required => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let open = publication(true)?;
    let closed = publication(false)?;
    for (source, expected_options, expected_runs) in [(&open, 2, 3), (&closed, 0, 1)] {
        let snapshot = source.semantic_publication().snapshot();
        let combo = snapshot
            .nodes()
            .iter()
            .find(|node| node.role() == SemanticRole::ComboBox)
            .ok_or_else(|| std::io::Error::other("ComboBox primary semantic role absent"))?;
        let count = snapshot
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::Option)
            .count();
        assert_eq!(count, expected_options);
        let controls = combo.relationships().iter()
            .find(|link| link.kind() == SemanticRelationshipKind::Controls);
        assert_eq!(controls.is_some(), expected_options != 0);
        if let Some(controls) = controls {
            assert_eq!(snapshot.nodes().iter()
                .find(|node| node.id() == controls.target())
                .map(|node| node.role()), Some(SemanticRole::ListBox),
                "M5 Controls must identify the actual popup ListBox, not the Popover Group");
        }
        assert_eq!(
            combo
                .relationships()
                .iter()
                .filter(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant)
                .count(),
            usize::from(expected_options != 0)
        );
        let runs = source
            .paint_scene()
            .items()
            .iter()
            .filter(|item| item.primitive().as_shaped_text_run().is_some())
            .count();
        assert_eq!(
            runs, expected_runs,
            "ordinary ComboBox Option text uses retained M8"
        );
    }
    let a = renderer.render_offscreen_publication(open.paint_publication(), &NoResources)?;
    let b = renderer.render_offscreen_publication(closed.paint_publication(), &NoResources)?;
    assert_ne!(
        a.readback().rgba8_srgb(),
        b.readback().rgba8_srgb(),
        "opening ComboBox adds actual native-wgpu Option pixels"
    );
    let dir = std::env::var_os("RUNENUI_M11COMBO_EVIDENCE_DIR")
        .or_else(|| std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR"));
    if let Some(path) = dir.map(PathBuf::from) {
        fs::create_dir_all(&path)?;
        for (name, result) in [("m11-combo-open.png", &a), ("m11-combo-closed.png", &b)] {
            let extent = result.readback().extent();
            image::save_buffer(
                path.join(name),
                result.readback().rgba8_srgb(),
                extent.width(),
                extent.height(),
                image::ColorType::Rgba8,
            )?;
        }
    }
    Ok(())
}
