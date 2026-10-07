#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{
    sync::Arc,
    task::{Context, Wake, Waker},
    thread,
};

use runenui_core::{
    Color, Element, LogicalLength, LogicalPoint, LogicalRect, LogicalSize, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionItem, ResourceRef, SceneLayer,
    SceneShape, StyleEnvironment, SurfacePresentation, SurfacePresentationAnchor,
    SurfacePresentationPlacement, SurfacePresentationSide, UiApp, View, Widget, WidgetMeasure,
    WidgetMeasureInput, column,
};
use runenui_render_wgpu::{
    BackendSelection, Renderer, RendererInitError, RendererOptions, ResourcePayload,
    ResourceProvider, ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, RasterScale, SurfaceBuildContext};

#[derive(Clone, Debug)]
struct PaintProbe {
    color: Color,
    layer: SceneLayer,
}

impl Widget<()> for PaintProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(40_u8), LogicalLength::from(40_u8))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let rect = LogicalRect::try_new(0.0, 0.0, 40.0, 40.0)
            .unwrap_or_else(|_| unreachable!("fixture rect is valid"));
        PaintContribution::single(
            PaintContributionItem::fill(SceneShape::rect(rect), self.color.into())
                .with_layer(self.layer),
        )
    }
}

struct PresentationRenderApp;

impl UiApp for PresentationRenderApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        let ordinary = Element::new(PaintProbe {
            color: Color::WHITE,
            layer: SceneLayer::new(10_000),
        });
        let presentation = Element::new(PaintProbe {
            color: Color::BLACK,
            layer: SceneLayer::new(-10_000),
        })
        .surface_presentation(
            SurfacePresentation::new(SurfacePresentationPlacement::new(
                SurfacePresentationSide::Center,
            ))
            .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                LogicalPoint::new(20.0, 20.0)
                    .unwrap_or_else(|_| unreachable!("fixture point is finite")),
            )),
        );
        column(vec![ordinary, presentation]).into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

struct NoResources;

impl ResourceProvider for NoResources {
    fn load(
        &self,
        _: &ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Malformed,
            "presentation proof unexpectedly requested a resource",
        ))
    }
}

fn pixel(readback: &runenui_render_wgpu::OffscreenReadback, x: u32, y: u32) -> [u8; 4] {
    let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
    readback.rgba8_srgb()[index..index + 4]
        .try_into()
        .unwrap_or_else(|_| unreachable!("pixel is inside fixture target"))
}

fn renderer_or_skip() -> Result<Option<Renderer>, Box<dyn Error>> {
    match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => Ok(Some(renderer)),
        Err(RendererInitError::AdapterUnavailable {
            requested,
            compatible_surface_required,
            detail,
        }) => {
            assert_eq!(requested, BackendSelection::AllNative);
            assert!(!compatible_surface_required);
            assert!(!detail.is_empty());
            eprintln!("M11 presentation renderer evidence skipped: {detail}");
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

#[test]
fn real_wgpu_consumes_runtime_presentation_band_without_renderer_popup_authority()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    let mut runtime = AppRuntime::<PresentationRenderApp>::mount(());
    let environment = StyleEnvironment::default();
    let size = LogicalSize::new(LogicalLength::from(64_u8), LogicalLength::from(64_u8));
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::tight(size))
        .with_raster_scale(RasterScale::ONE);
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("presentation publication is admitted"));
    let output =
        renderer.render_offscreen_publication(publication.paint_publication(), &NoResources)?;

    assert_eq!(
        pixel(output.readback(), 10, 10),
        [0, 0, 0, 0xFF],
        "renderer must consume runtime ordering: presentation black stays above ordinary white despite lower SceneLayer"
    );
    Ok(())
}

struct ThreadWake(thread::Thread);

impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}
