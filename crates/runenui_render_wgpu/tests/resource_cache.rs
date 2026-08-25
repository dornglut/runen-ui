#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{
    cell::Cell,
    sync::Arc,
    task::{Context, Wake, Waker},
    thread,
};

use runenui_core::{
    Element, LogicalLength, LogicalRect, LogicalSize, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, ResourceKind, ResourceRef, StyleTokens, UiApp,
    Widget, WidgetMeasure, WidgetUpdateContext,
};
use runenui_render_wgpu::{
    BackendSelection, ImagePayload, PublicationRenderError, PublicationUpdateMode, Renderer,
    RendererInitError, RendererOptions, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest, ResourceResolveError,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintPublication, RasterScale, SurfaceBuildContext,
};

const SURFACE_WIDTH: u16 = 16;
const SURFACE_HEIGHT: u16 = 12;
const IMAGE_RGBA: [u8; 4] = [0xB4, 0x52, 0x33, 0xFF];

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
        *state = self.items.clone();
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
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::tight(logical_size))
        .with_raster_scale(RasterScale::ONE);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
        .paint_publication()
        .clone()
}

struct SwitchableImageProvider {
    resource: ResourceRef,
    payload: ImagePayload,
    loads: Cell<usize>,
    available: Cell<bool>,
}

impl SwitchableImageProvider {
    fn new(resource: ResourceRef) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            resource,
            payload: ImagePayload::new(1, 1, IMAGE_RGBA.to_vec())?,
            loads: Cell::new(0),
            available: Cell::new(true),
        })
    }

    fn loads(&self) -> usize {
        self.loads.get()
    }

    fn set_available(&self, available: bool) {
        self.available.set(available);
    }
}

impl ResourceProvider for SwitchableImageProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        if resource != &self.resource || request != ResourceRequest::Image {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "unexpected resource request in cache-loss fixture",
            ));
        }
        if !self.available.get() {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Unavailable,
                "fixture provider intentionally unavailable",
            ));
        }
        Ok(ResourcePayload::Image(self.payload.clone()))
    }
}

#[test]
fn resource_cache_loss_forces_full_resync_and_reloads_before_repaint() -> Result<(), Box<dyn Error>>
{
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let image_ref = ResourceRef::new(ResourceKind::Image);
    let provider = SwitchableImageProvider::new(image_ref.clone())?;
    let image = PaintContributionItem::image(
        image_ref,
        rect(
            0.0,
            0.0,
            f32::from(SURFACE_WIDTH),
            f32::from(SURFACE_HEIGHT),
        ),
    )?;
    let publication = publication(vec![image]);

    let first = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        first.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    let generation = first.target_generation();
    assert_eq!(provider.loads(), 1);
    assert_eq!(pixel(first.readback(), 4, 4), IMAGE_RGBA);

    let current = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        current.update_plan().mode(),
        PublicationUpdateMode::AlreadyCurrent
    );
    assert_eq!(current.target_generation(), generation);
    assert_eq!(
        provider.loads(),
        1,
        "already-current rendering reuses the cache"
    );

    assert!(renderer.discard_resource_cache());
    provider.set_available(false);
    let failed = renderer.render_offscreen_publication(&publication, &provider);
    assert!(matches!(
        failed,
        Err(PublicationRenderError::Resource {
            item_index: 0,
            error: ResourceResolveError::Provider(ref error),
        }) if error.kind() == ResourceProviderErrorKind::Unavailable
    ));
    assert_eq!(
        provider.loads(),
        2,
        "cache loss must invalidate already-current lineage and force provider preflight"
    );

    provider.set_available(true);
    let rebuilt = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_eq!(
        rebuilt.target_generation(),
        generation,
        "resource-cache loss invalidates lineage without destroying the retained target"
    );
    assert_eq!(provider.loads(), 3);
    assert_eq!(pixel(rebuilt.readback(), 4, 4), IMAGE_RGBA);

    let rebuilt_current = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        rebuilt_current.update_plan().mode(),
        PublicationUpdateMode::AlreadyCurrent
    );
    assert_eq!(provider.loads(), 3);

    eprintln!(
        "REAL GPU RESOURCE CACHE PROOF: cache reuse, cache-loss full resync, provider preflight failure, retained-target preservation, and provider-backed reconstruction succeeded; adapter={:?} backend={}",
        renderer.diagnostics().adapter_info().name,
        renderer.diagnostics().adapter_info().backend,
    );
    Ok(())
}

fn pixel(readback: &runenui_render_wgpu::OffscreenReadback, x: u32, y: u32) -> [u8; 4] {
    let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
    readback.rgba8_srgb()[index..index + 4]
        .try_into()
        .unwrap_or_else(|_| unreachable!("pixel index is in the fixture target"))
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
                "native wgpu resource-cache proof unavailable under {requested:?}; structured adapter failure: {detail}"
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
