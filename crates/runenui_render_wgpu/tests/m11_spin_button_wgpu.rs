#![allow(refining_impl_trait)]

//! Correlated real-wgpu proof: the typed `SpinButton` uses the ordinary M8/M10
//! text leaf and has no renderer-specific primitive or second editing buffer.
use core::{
    error::Error,
    future::Future,
    pin::pin,
    task::{Context, Poll},
};
use runenui_core::{
    Color, FontFamilyName, GenericFontFamily, NoHostProtocol, SemanticRange, SemanticRole,
    SpinButtonRange, StyleEnvironment, TextAffinity, TextDocumentId, TextDocumentRevision,
    TextDocumentSnapshot, TextPosition, TextSelection, UiApp, View, spin_button,
};
use runenui_render_wgpu::{
    Renderer, RendererInitError, RendererOptions, ResourcePayload, ResourceProvider,
    ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{AppRuntime, LogicalSize, SurfaceBuildContext};
use std::{fs, path::PathBuf};

const FONT: &[u8] = include_bytes!("fixtures/Cantarell-Regular.ttf");

struct NoExternalResources;
impl ResourceProvider for NoExternalResources {
    fn load(
        &self,
        _: &runenui_core::ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Malformed,
            "SpinButton uses only canonical retained text; no external images",
        ))
    }
}

struct SpinGpuApp;
impl UiApp for SpinGpuApp {
    type State = String;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(source: &Self::State) -> impl View<Self::Action> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(3240), TextDocumentRevision::new(1));
        let position = TextPosition::new(snapshot, source, source.len(), TextAffinity::Upstream)
            .unwrap_or_else(|_| unreachable!("checked source UTF-8 position"));
        let value = source
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite() && (-10.0..=100.0).contains(number));
        spin_button(
            "Amount",
            snapshot,
            source.clone(),
            TextSelection::collapsed(position),
            SpinButtonRange::new(-10.0, 100.0, value, 0.5)
                .unwrap_or_else(|_| unreachable!("finite authored bounds and step")),
            |_| (),
        )
        .unwrap_or_else(|_| unreachable!("one checked TextField source"))
        .on_adjust(|_| ())
        .foreground(Color::WHITE)
        .id("gpu.numeric")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = pin!(future);
    loop {
        match Future::poll(future.as_mut(), &mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn publication(source: &str) -> Result<runenui_runtime::SurfacePublication, Box<dyn Error>> {
    let mut runtime = AppRuntime::<SpinGpuApp>::mount(source.to_owned());
    runtime.register_text_font_bytes(FONT.to_vec())?;
    runtime.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")?],
    )?;
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(300.0, 60.0)?,
        ))
        .map_err(|error| std::io::Error::other(format!("SpinButton surface error: {error:?}")))
        .map_err(Into::into)
}

#[test]
fn spin_button_valid_and_intermediate_text_share_real_wgpu_and_semantic_authority()
-> Result<(), Box<dyn Error>> {
    let mut renderer = match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => renderer,
        Err(RendererInitError::AdapterUnavailable { .. }) => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let provider = NoExternalResources;
    let valid = publication("4.5")?;
    let invalid = publication("-")?;
    let valid_snapshot = valid.semantic_publication().snapshot();
    let invalid_snapshot = invalid.semantic_publication().snapshot();
    let valid_node = valid_snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::SpinButton)
        .ok_or_else(|| std::io::Error::other("valid SpinButton role absent"))?;
    let invalid_node = invalid_snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::SpinButton)
        .ok_or_else(|| std::io::Error::other("invalid SpinButton role absent"))?;
    assert_eq!(
        valid_node
            .range()
            .and_then(SemanticRange::current)
            .map(SemanticNumber::get),
        Some(4.5)
    );
    assert_eq!(invalid_node.range().and_then(SemanticRange::current), None);
    assert!(valid_node.editable().is_some());
    assert!(invalid_node.editable().is_some());
    assert_eq!(
        valid
            .paint_scene()
            .items()
            .iter()
            .filter(|item| { item.primitive().as_shaped_text_run().is_some() })
            .count(),
        1,
        "one ordinary M8 text artifact, not another numeric renderer"
    );
    let a = renderer.render_offscreen_publication(valid.paint_publication(), &provider)?;
    let b = renderer.render_offscreen_publication(invalid.paint_publication(), &provider)?;
    assert_ne!(
        a.readback().rgba8_srgb(),
        b.readback().rgba8_srgb(),
        "ordinary retained text distinguishes valid and incomplete source pixels"
    );
    if let Some(directory) = std::env::var_os("RUNENUI_M11SPIN_EVIDENCE_DIR").map(PathBuf::from) {
        fs::create_dir_all(&directory)?;
        for (filename, render) in [("m11-spin-valid.png", &a), ("m11-spin-incomplete.png", &b)] {
            let extent = render.readback().extent();
            image::save_buffer(
                directory.join(filename),
                render.readback().rgba8_srgb(),
                extent.width(),
                extent.height(),
                image::ColorType::Rgba8,
            )?;
        }
    }
    Ok(())
}
