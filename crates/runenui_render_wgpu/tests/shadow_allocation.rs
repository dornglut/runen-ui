#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{cell::Cell, task::Context};

use runenui_core::{
    Brush, Color, DropShadow, Element, LogicalLength, LogicalRect, LogicalSize, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionGroup, PaintContributionItem,
    SceneShape, StyleEnvironment, UiApp, Widget, WidgetMeasure,
};
use runenui_render_wgpu::{
    BackendSelection, PublicationRenderError, Renderer, RendererInitError, RendererOptions,
    ResourcePayload, ResourceProvider, ResourceProviderError, ResourceProviderErrorKind,
    ResourceRequest,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, RasterScale, SurfaceBuildContext};

const SURFACE_WIDTH: u16 = 64;
const SURFACE_HEIGHT: u16 = 48;

#[derive(Clone, Copy, Debug)]
struct OversizedShadowFixture {
    side: f32,
}

impl Widget<()> for OversizedShadowFixture {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::from(SURFACE_WIDTH),
            LogicalLength::from(SURFACE_HEIGHT),
        )
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let source_rect = LogicalRect::try_new(-self.side, 0.0, self.side, self.side)
            .unwrap_or_else(|_| unreachable!("controlled source rectangle is finite"));
        let source =
            PaintContributionItem::fill(SceneShape::rect(source_rect), Brush::solid(Color::WHITE));
        let shadow = DropShadow::new(self.side, 0.0, LogicalLength::ZERO, 0.0, Color::WHITE)
            .unwrap_or_else(|_| unreachable!("controlled shadow is finite"));
        PaintContribution::from_entries(vec![
            PaintContributionGroup::new(vec![source.into()])
                .with_shadows(vec![shadow])
                .into(),
        ])
    }
}

struct OversizedShadowApp;

impl UiApp for OversizedShadowApp {
    type State = f32;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(side: &Self::State) -> Element<Self::Action> {
        Element::new(OversizedShadowFixture { side: *side })
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[derive(Default)]
struct CountingProvider {
    loads: Cell<usize>,
}

impl ResourceProvider for CountingProvider {
    fn load(
        &self,
        _: &runenui_core::ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Malformed,
            "allocation-boundary proof unexpectedly requested a resource",
        ))
    }
}

fn publication(side: f32) -> runenui_runtime::PaintPublication {
    let mut runtime = AppRuntime::<OversizedShadowApp>::mount(side);
    let environment = StyleEnvironment::default();
    let logical_size = LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
        .unwrap_or_else(|_| unreachable!("controlled surface size is valid"));
    runtime
        .publish_surface(
            &SurfaceBuildContext::new(&environment, LayoutConstraints::tight(logical_size))
                .with_raster_scale(RasterScale::ONE),
        )
        .unwrap_or_else(|_| unreachable!("controlled publication is admitted"))
        .paint_publication()
        .clone()
}

fn renderer_or_adapterless() -> Result<Option<Renderer>, Box<dyn Error>> {
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
            eprintln!("shadow allocation proof skipped: {detail}");
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "the proof deliberately chooses a renderer-private workspace many times larger than the adapter byte limit; exact subpixel size is irrelevant"
)]
fn source_side_for_limit(max_workspace_bytes: u64) -> Result<f32, Box<dyn Error>> {
    let side_pixels = max_workspace_bytes.isqrt().saturating_mul(2).max(1024);
    let side_pixels = u32::try_from(side_pixels)
        .map_err(|_| "device buffer limit exceeds the addressable allocation-proof fixture")?;
    Ok(side_pixels as f32)
}

#[test]
fn oversized_off_surface_shadow_fails_before_crop_or_target_mutation() -> Result<(), Box<dyn Error>>
{
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    assert!(renderer.last_observation().is_none());

    let max_workspace_bytes = renderer.diagnostics().device_limits().max_buffer_size;
    let side = source_side_for_limit(max_workspace_bytes)?;
    let publication = publication(side);
    let provider = CountingProvider::default();

    let Err(error) = renderer.render_offscreen_publication(&publication, &provider) else {
        return Err("oversized exact shadow support rendered instead of failing".into());
    };
    match error {
        PublicationRenderError::GroupShadowRealization {
            shadow_index,
            detail,
        } => {
            assert_eq!(shadow_index, 0);
            assert!(
                detail.contains("exceeding renderer allocation limit"),
                "allocation policy must remain an explicit realization failure: {detail}"
            );
            assert!(
                detail.contains(&max_workspace_bytes.to_string()),
                "diagnostic must retain the adapter allocation limit: {detail}"
            );
        }
        other => return Err(format!("unexpected shadow allocation result: {other}").into()),
    }

    assert_eq!(provider.loads.get(), 0);
    assert!(
        renderer.last_observation().is_none(),
        "shadow allocation failure must happen before a render observation/transaction begins"
    );
    assert!(
        !renderer.discard_offscreen_target(),
        "shadow allocation failure must happen before an offscreen target is allocated or mutated"
    );
    Ok(())
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
