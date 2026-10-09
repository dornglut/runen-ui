#![allow(refining_impl_trait)]

use core::{
    error::Error,
    future::Future,
    pin::pin,
    task::{Context, Poll},
};

use runenui_core::{
    Color, NoHostProtocol, SemanticNumber, StyleEnvironment, UiApp, View, splitter,
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
            "standard Splitter uses only generic shape primitives",
        ))
    }
}

struct SplitterApp;

impl UiApp for SplitterApp {
    type State = SemanticNumber;
    type Action = SemanticNumber;
    type HostProtocol = NoHostProtocol;

    fn root(value: &Self::State) -> impl View<Self::Action> {
        splitter("Audio", 0.0, 100.0, value.get(), 10.0)
            .unwrap_or_else(|_| unreachable!("finite Splitter range"))
            .on_resize(|_| SemanticNumber::new(0.0).unwrap_or_else(|_| unreachable!()))
            .background(Color::rgba(60, 210, 120, 255))
    }

    fn update(state: &mut Self::State, value: Self::Action) {
        *state = value;
    }
}

fn publish(runtime: &mut AppRuntime<SplitterApp>) -> runenui_runtime::SurfacePublication {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(12.0, 160.0)
                .unwrap_or_else(|_| unreachable!("valid surface extent")),
        ))
        .unwrap_or_else(|_| unreachable!("standard Splitter scene publishes"))
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
fn standard_splitter_paints_a_thin_generic_grip_inside_wide_hit_extent()
-> Result<(), Box<dyn Error>> {
    let mut renderer = match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => renderer,
        Err(RendererInitError::AdapterUnavailable { .. }) => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let mut runtime = AppRuntime::<SplitterApp>::mount(
        SemanticNumber::new(50.0).unwrap_or_else(|_| unreachable!("finite")),
    );
    let provider = NoResources;
    let publication = publish(&mut runtime);
    let items = publication.paint_scene().items();
    assert_eq!(items.len(), 1, "one ordinary generic divider shape");
    assert!(
        items
            .iter()
            .all(|item| matches!(item.primitive(), runenui_core::PaintPrimitive::Fill { .. })),
        "no renderer-special Splitter primitive"
    );
    let image =
        renderer.render_offscreen_publication(publication.paint_publication(), &provider)?;
    let extent = image.readback().extent();
    let pixels = image.readback().rgba8_srgb();
    let width = usize::try_from(extent.width())?;
    let row = usize::try_from(extent.height())? / 2;
    let edge = (row * width) * 4;
    let grip = (row * width + 6) * 4;
    assert_ne!(
        &pixels[edge..edge + 4],
        &pixels[grip..grip + 4],
        "generic paint produces a narrow line, not a full-width filled hit box"
    );
    Ok(())
}
