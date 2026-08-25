#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{
    cell::Cell,
    sync::Arc,
    task::{Context, Wake, Waker},
    thread,
};

use runenui_core::{
    Color, ContributionClip, Element, LogicalLength, LogicalPoint, LogicalRect, LogicalSize,
    LogicalTransform, NoHostProtocol, PaintContribution, PaintContributionContext,
    PaintContributionItem, Radius, ResourceKind, ResourceRef, SceneOpacity, SceneShape,
    StyleTokens, UiApp, Widget, WidgetMeasure, WidgetUpdateContext,
};
use runenui_render_wgpu::{
    BackendSelection, ImagePayload, PublicationRenderError, Renderer, RendererInitError,
    RendererOptions, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest, ResourceResolveError, ShapedRunRaster,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintPublication, RasterScale, SurfaceBuildContext,
};

const SURFACE_WIDTH: u16 = 32;
const SURFACE_HEIGHT: u16 = 24;
const RASTER_SCALE: f32 = 2.0;
const IMAGE_PIXELS: [u8; 16] = [
    0xFF, 0x00, 0x00, 0xFF, // top-left red
    0x00, 0xFF, 0x00, 0xFF, // top-right green
    0x00, 0x00, 0xFF, 0xFF, // bottom-left blue
    0xFF, 0xFF, 0xFF, 0xFF, // bottom-right white
];
const PNG_FIXTURE: &[u8] = include_bytes!("fixtures/provider_image.png");

#[derive(Clone, Debug)]
struct SceneFixture {
    items: Vec<PaintContributionItem>,
}

impl Widget<Vec<PaintContributionItem>> for SceneFixture {
    type State = Vec<PaintContributionItem>;

    fn create_state(&self) -> Self::State {
        self.items.clone()
    }

    fn update(
        &self,
        state: &mut Self::State,
        _: &mut WidgetUpdateContext<Vec<PaintContributionItem>>,
    ) {
        state.clone_from(&self.items);
    }

    fn measure(&self, _: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(SURFACE_WIDTH),
            height: LogicalLength::from(SURFACE_HEIGHT),
        }
    }

    fn paint(&self, items: &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(items.clone())
    }
}

struct FixtureApp;

impl UiApp for FixtureApp {
    type State = Vec<PaintContributionItem>;
    type Action = Vec<PaintContributionItem>;
    type HostProtocol = NoHostProtocol;

    fn root(items: &Self::State) -> Element<Self::Action> {
        Element::new(SceneFixture {
            items: items.clone(),
        })
    }

    fn update(items: &mut Self::State, replacement: Self::Action) {
        *items = replacement;
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
}

fn publication(items: Vec<PaintContributionItem>) -> PaintPublication {
    let mut runtime = AppRuntime::<FixtureApp>::mount(items);
    let tokens = StyleTokens::new();
    let logical_size = LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
        .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
    let raster_scale = RasterScale::new(RASTER_SCALE)
        .unwrap_or_else(|_| unreachable!("fixture raster scale is valid"));
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::tight(logical_size))
        .with_raster_scale(raster_scale);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
        .paint_publication()
        .clone()
}

struct CountingImageProvider {
    resource: ResourceRef,
    payload: ImagePayload,
    loads: Cell<usize>,
}

impl CountingImageProvider {
    fn new(resource: ResourceRef) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            resource,
            payload: ImagePayload::new(2, 2, IMAGE_PIXELS.to_vec())?,
            loads: Cell::new(0),
        })
    }

    const fn loads(&self) -> usize {
        self.loads.get()
    }
}

impl ResourceProvider for CountingImageProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        if resource != &self.resource || request != ResourceRequest::Image {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "unexpected resource request in image-render fixture",
            ));
        }
        Ok(ResourcePayload::Image(self.payload.clone()))
    }
}

struct PngImageProvider {
    resource: ResourceRef,
    loads: Cell<usize>,
}

impl PngImageProvider {
    const fn new(resource: ResourceRef) -> Self {
        Self {
            resource,
            loads: Cell::new(0),
        }
    }

    const fn loads(&self) -> usize {
        self.loads.get()
    }
}

impl ResourceProvider for PngImageProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        if resource != &self.resource || request != ResourceRequest::Image {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "unexpected resource request in PNG fixture provider",
            ));
        }
        let decoded = image::load_from_memory(PNG_FIXTURE).map_err(|error| {
            ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                format!("PNG fixture decode failed: {error}"),
            )
        })?;
        // The fixture is authored as sRGB. `to_rgba8` is the image crate's
        // straight/unpremultiplied RGBA8 normalization path; the renderer
        // payload keeps those bytes unchanged and performs no decoding.
        let rgba8 = decoded.to_rgba8();
        ImagePayload::new(rgba8.width(), rgba8.height(), rgba8.into_raw())
            .map(ResourcePayload::Image)
            .map_err(|error| {
                ResourceProviderError::new(
                    ResourceProviderErrorKind::Malformed,
                    format!("PNG fixture normalization failed: {error}"),
                )
            })
    }
}

struct WrongPayloadProvider {
    resource: ResourceRef,
}

impl ResourceProvider for WrongPayloadProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        assert_eq!(resource, &self.resource);
        assert_eq!(request, ResourceRequest::Image);
        let raster = ShapedRunRaster::new(
            LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!()),
            0,
            0,
            RasterScale::ONE,
            Vec::new(),
        )
        .unwrap_or_else(|_| unreachable!("empty shaped payload is valid"));
        Ok(ResourcePayload::ShapedTextRun(raster))
    }
}

#[test]
fn real_gpu_image_semantics_match_scene_contract() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };

    let image_ref = ResourceRef::new(ResourceKind::Image);
    let provider = CountingImageProvider::new(image_ref.clone())?;
    let image_transform = LogicalTransform::translation(2.0, 1.0)?;
    let clip_transform = LogicalTransform::translation(4.0, 3.0)?;
    let clip = ContributionClip::new(
        SceneShape::rounded_rect(
            rect(0.0, 0.0, 8.0, 8.0),
            Radius::all(LogicalLength::new(2.0)?),
        ),
        clip_transform,
    );
    let transformed_image =
        PaintContributionItem::image(image_ref.clone(), rect(2.0, 2.0, 8.0, 8.0))?
            .with_transform(image_transform)
            .with_clip(clip);
    let translucent_image = PaintContributionItem::image(image_ref, rect(16.0, 4.0, 8.0, 8.0))?
        .with_opacity(SceneOpacity::new(0.5)?);
    let overlay = Color::rgb(0xE0, 0xA0, 0x20);
    let publication = publication(vec![
        PaintContributionItem::fill_rect(
            rect(
                0.0,
                0.0,
                f32::from(SURFACE_WIDTH),
                f32::from(SURFACE_HEIGHT),
            ),
            Color::BLACK,
        ),
        transformed_image,
        translucent_image,
        PaintContributionItem::fill_rect(rect(5.5, 4.5, 2.0, 2.0), overlay),
    ]);

    let output = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        output.readback().extent(),
        runenui_render_wgpu::OffscreenExtent::new(64, 48)?
    );
    assert_eq!(provider.loads(), 1, "one complete ref is realized once");

    let readback = output.readback();
    assert_eq!(pixel(readback, 10, 8), [0xFF, 0x00, 0x00, 0xFF]);
    assert_eq!(pixel(readback, 20, 8), [0x00, 0xFF, 0x00, 0xFF]);
    assert_eq!(pixel(readback, 10, 18), [0x00, 0x00, 0xFF, 0xFF]);
    assert_eq!(pixel(readback, 20, 18), [0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(
        pixel(readback, 8, 6),
        [0x00, 0x00, 0x00, 0xFF],
        "transformed rounded clip excludes the image corner"
    );
    assert_eq!(
        pixel(readback, 12, 10),
        [0xE0, 0xA0, 0x20, 0xFF],
        "a later literal item wins over the earlier image in scene order"
    );
    assert_pixel_near(
        pixel(readback, 35, 11),
        [188, 0, 0, 255],
        1,
        "half-opacity red image over opaque black uses linear source-over",
    );

    let current = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        current.update_plan().mode(),
        runenui_render_wgpu::PublicationUpdateMode::AlreadyCurrent
    );
    assert_eq!(
        provider.loads(),
        1,
        "already-current image render reuses realization"
    );
    assert_eq!(current.readback().rgba8_srgb(), readback.rgba8_srgb());

    eprintln!(
        "REAL GPU IMAGE PROOF: non-uniform normalized domain, affine placement, transformed rounded clipping, item opacity, mixed literal/image ordering, scale=2, same-ref realization dedupe, and already-current cache reuse succeeded; adapter={:?} backend={}",
        renderer.diagnostics().adapter_info().name,
        renderer.diagnostics().adapter_info().backend,
    );
    Ok(())
}

#[test]
fn png_provider_normalizes_complete_domain_and_reuses_image_cache() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let image_ref = ResourceRef::new(ResourceKind::Image);
    let provider = PngImageProvider::new(image_ref.clone());
    let publication = publication(vec![PaintContributionItem::image(
        image_ref,
        rect(2.0, 2.0, 8.0, 8.0),
    )?]);

    let first = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(provider.loads(), 1);
    assert_eq!(pixel(first.readback(), 5, 5), [0xFF, 0x00, 0x00, 0xFF]);
    assert_eq!(pixel(first.readback(), 18, 5), [0x00, 0xFF, 0x00, 0xFF]);
    assert_eq!(pixel(first.readback(), 5, 18), [0x00, 0x00, 0xFF, 0xFF]);
    assert_eq!(pixel(first.readback(), 18, 18), [0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(
        first.observation().resource_observations()[0].cache_outcome(),
        runenui_render_wgpu::ResourceCacheOutcome::Realized
    );

    let current = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(provider.loads(), 1);
    assert_eq!(
        current.observation().resource_observations()[0].cache_outcome(),
        runenui_render_wgpu::ResourceCacheOutcome::Reused
    );
    assert_eq!(
        current.readback().rgba8_srgb(),
        first.readback().rgba8_srgb()
    );
    eprintln!(
        "REAL GPU PNG RESOURCE PROOF: image crate PNG decode, explicit straight RGBA8 normalization, complete orientation, destination mapping, renderer-owned upload, and cache reuse succeeded; adapter={:?} backend={}",
        renderer.diagnostics().adapter_info().name,
        renderer.diagnostics().adapter_info().backend,
    );
    Ok(())
}

#[test]
fn image_provider_wrong_payload_is_deterministic_and_not_an_image_decode_fallback()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let image_ref = ResourceRef::new(ResourceKind::Image);
    let provider = WrongPayloadProvider {
        resource: image_ref.clone(),
    };
    let publication = publication(vec![PaintContributionItem::image(
        image_ref,
        rect(0.0, 0.0, 4.0, 4.0),
    )?]);
    let Err(error) = renderer.render_offscreen_publication(&publication, &provider) else {
        return Err(std::io::Error::other(
            "wrong provider payload rendered instead of failing before upload",
        )
        .into());
    };
    assert!(matches!(
        error,
        PublicationRenderError::Resource {
            item_index: 0,
            error: ResourceResolveError::PayloadKindMismatch { .. },
        }
    ));
    let observation = renderer
        .last_observation()
        .ok_or_else(|| std::io::Error::other("failed publication did not remain observable"))?;
    assert_eq!(
        observation.resource_observations()[0].cache_outcome(),
        runenui_render_wgpu::ResourceCacheOutcome::Failed
    );
    Ok(())
}

fn pixel(readback: &runenui_render_wgpu::OffscreenReadback, x: u32, y: u32) -> [u8; 4] {
    let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
    readback.rgba8_srgb()[index..index + 4]
        .try_into()
        .unwrap_or_else(|_| unreachable!("pixel index is in the fixture target"))
}

fn assert_pixel_near(actual: [u8; 4], expected: [u8; 4], tolerance: u8, context: &str) {
    for (channel, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
        assert!(
            actual.abs_diff(expected) <= tolerance,
            "{context}: channel {channel} expected {expected}±{tolerance}, got {actual}"
        );
    }
}

fn renderer_or_adapterless() -> Result<Option<Renderer>, Box<dyn Error>> {
    match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => Ok(Some(renderer)),
        Err(RendererInitError::AdapterUnavailable {
            requested,
            compatible_surface_required,
            detail,
        }) => {
            eprintln!(
                "native wgpu image proof unavailable under {requested:?}; structured adapter failure: {detail}"
            );
            assert_eq!(requested, BackendSelection::AllNative);
            assert!(!compatible_surface_required);
            assert!(!detail.is_empty());
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
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
