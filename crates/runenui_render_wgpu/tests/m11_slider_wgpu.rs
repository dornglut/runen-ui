#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{fs, path::PathBuf};

use runenui_core::{
    Color, NoHostProtocol, SemanticNumber, StyleEnvironment, UiApp, View, slider,
};
use runenui_render_wgpu::{
    Renderer, RendererInitError, RendererOptions, ResourcePayload, ResourceProvider,
    ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{AppRuntime, LogicalSize, SurfaceBuildContext};

struct NoResources;

impl ResourceProvider for NoResources {
    fn load(
        &self,
        _: &runenui_core::ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Malformed,
            "standard Slider uses only generic shape primitives",
        ))
    }
}

struct SliderApp;

impl UiApp for SliderApp {
    type State = SemanticNumber;
    type Action = SemanticNumber;
    type HostProtocol = NoHostProtocol;

    fn root(value: &Self::State) -> impl View<Self::Action> {
        slider("Audio", 0.0, 100.0, value.get(), 10.0)
            .unwrap_or_else(|_| unreachable!("finite Slider range"))
            .on_change(|new_value| new_value)
            .foreground(Color::rgba(60, 210, 120, 255))
    }

    fn update(state: &mut Self::State, value: Self::Action) {
        *state = value;
    }
}

fn publish(runtime: &mut AppRuntime<SliderApp>) -> runenui_runtime::SurfacePublication {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(160.0, 24.0)
                .unwrap_or_else(|_| unreachable!("valid surface extent")),
        ))
        .unwrap_or_else(|_| unreachable!("standard Slider scene publishes"))
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

#[test]
fn standard_slider_uses_generic_wgpu_shape_pipeline_and_rebuilds_pixels()
-> Result<(), Box<dyn Error>> {
    let mut renderer = match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => renderer,
        Err(RendererInitError::AdapterUnavailable { .. }) => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let mut runtime = AppRuntime::<SliderApp>::mount(
        SemanticNumber::new(0.0).unwrap_or_else(|_| unreachable!("zero finite")),
    );
    let provider = NoResources;
    let initial = publish(&mut runtime);
    let items = initial.paint_scene().items();
    assert!(items.len() >= 2, "Slider has visible track and thumb");
    assert!(
        items.iter().all(|item| matches!(
            item.primitive(),
            runenui_core::PaintPrimitive::Fill { .. }
        )),
        "Slider uses only ordinary generic fill primitives"
    );
    let beginning = renderer.render_offscreen_publication(
        initial.paint_publication(), &provider,
    )?;
    let start_pixels = beginning.readback().rgba8_srgb().to_vec();
    runtime
        .submit_action(
            SemanticNumber::new(100.0)
                .unwrap_or_else(|_| unreachable!("full scale finite")),
        )
        .unwrap_or_else(|_| unreachable!("application update accepted"));
    runtime.pump(runenui_runtime::PumpBudget::new(32, 32, 32, 32));
    let full = publish(&mut runtime);
    let ending = renderer.render_offscreen_publication(
        full.paint_publication(), &provider,
    )?;
    let end_pixels = ending.readback().rgba8_srgb().to_vec();
    assert_ne!(
        start_pixels, end_pixels,
        "application-owned value rebuild must move generic filled track and thumb"
    );
    if let Some(path) = std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR") {
        let directory = PathBuf::from(path);
        fs::create_dir_all(&directory)?;
        let extent = ending.readback().extent();
        image::save_buffer(
            directory.join("m11-slider-start.png"), &start_pixels,
            extent.width(), extent.height(), image::ColorType::Rgba8,
        )?;
        image::save_buffer(
            directory.join("m11-slider-full.png"), &end_pixels,
            extent.width(), extent.height(), image::ColorType::Rgba8,
        )?;
    }
    Ok(())
}
