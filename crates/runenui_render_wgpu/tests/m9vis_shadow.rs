#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{
    sync::Arc,
    task::{Context, Wake, Waker},
    thread,
};

use runenui_core::{
    Brush, Color, DropShadow, Element, LogicalLength, LogicalRect, LogicalSize, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionEntry, PaintContributionGroup,
    PaintContributionItem, ResourceRef, SceneShape, StyleEnvironment, UiApp, Widget, WidgetMeasure,
};
use runenui_render_wgpu::{
    BackendSelection, Renderer, RendererInitError, RendererOptions, ResourcePayload,
    ResourceProvider, ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintPublication, RasterScale, SurfaceBuildContext,
};

const SURFACE_WIDTH: u16 = 64;
const SURFACE_HEIGHT: u16 = 32;
const L_SOURCE: &[(u16, u16)] = &[
    (0, 0),
    (0, 1),
    (0, 2),
    (0, 3),
    (1, 3),
    (2, 3),
    (3, 3),
    (3, 2),
];

#[derive(Clone, Debug)]
struct GroupSceneFixture {
    entries: Vec<PaintContributionEntry>,
}

impl Widget<Vec<PaintContributionEntry>> for GroupSceneFixture {
    type State = Vec<PaintContributionEntry>;

    fn create_state(&self) -> Self::State {
        self.entries.clone()
    }

    fn measure(&self, _: &Self::State, _: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::from(SURFACE_WIDTH),
            LogicalLength::from(SURFACE_HEIGHT),
        )
    }

    fn paint(&self, entries: &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::from_entries(entries.clone())
    }
}

struct GroupFixtureApp;

impl UiApp for GroupFixtureApp {
    type State = Vec<PaintContributionEntry>;
    type Action = Vec<PaintContributionEntry>;
    type HostProtocol = NoHostProtocol;

    fn root(entries: &Self::State) -> Element<Self::Action> {
        Element::new(GroupSceneFixture {
            entries: entries.clone(),
        })
    }

    fn update(entries: &mut Self::State, replacement: Self::Action) {
        *entries = replacement;
    }
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
            "M9VIS-08 shadow proof unexpectedly requested a resource",
        ))
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
}

fn grouped_publication(entries: Vec<PaintContributionEntry>) -> PaintPublication {
    let mut runtime = AppRuntime::<GroupFixtureApp>::mount(entries);
    let environment = StyleEnvironment::default();
    let logical_size = LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
        .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::tight(logical_size))
        .with_raster_scale(RasterScale::ONE);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("M9VIS-08 fixture publication is admitted"))
        .paint_publication()
        .clone()
}

fn spread_shadow_group(
    origin_x: u16,
    origin_y: u16,
    cells: impl IntoIterator<Item = (u16, u16)>,
) -> Result<PaintContributionGroup, Box<dyn Error>> {
    let children = cells
        .into_iter()
        .map(|(x, y)| {
            PaintContributionItem::fill(
                SceneShape::rect(rect(
                    f32::from(origin_x + x),
                    f32::from(origin_y + y),
                    1.0,
                    1.0,
                )),
                Brush::solid(Color::rgba(0xFF, 0xFF, 0xFF, 0x00)),
            )
            .into()
        })
        .collect::<Vec<_>>();
    let shadow = DropShadow::new(
        0.0,
        0.0,
        LogicalLength::new(0.0)?,
        2.0,
        Color::WHITE,
    )?;
    Ok(PaintContributionGroup::new(children).with_shadows(vec![shadow]))
}

fn alpha(readback: &runenui_render_wgpu::OffscreenReadback, x: u32, y: u32) -> u8 {
    let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
    readback.rgba8_srgb()[index + 3]
}

#[test]
fn real_gpu_euclidean_spread_rejects_square_corners_and_is_quarter_turn_invariant()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };

    let point = spread_shadow_group(4, 4, [(0, 0)])?;
    let source = spread_shadow_group(16, 8, L_SOURCE.iter().copied())?;
    let rotated = spread_shadow_group(
        40,
        8,
        L_SOURCE.iter().copied().map(|(x, y)| (3 - y, x)),
    )?;
    let publication = grouped_publication(vec![point.into(), source.into(), rotated.into()]);
    let output = renderer.render_offscreen_publication(&publication, &NoResources)?;
    let readback = output.readback();

    assert_eq!(alpha(readback, 4, 4), u8::MAX);
    assert_eq!(alpha(readback, 2, 4), u8::MAX);
    assert_eq!(alpha(readback, 4, 2), u8::MAX);
    assert_eq!(alpha(readback, 3, 3), u8::MAX);
    assert_eq!(
        alpha(readback, 2, 2),
        0,
        "radius-2 Euclidean spread must reject the distance-sqrt(8) corner that a square kernel would include"
    );

    for y in 0..8 {
        for x in 0..8 {
            let original = alpha(readback, 14 + x, 6 + y);
            let quarter_turned = alpha(readback, 38 + (7 - y), 6 + x);
            assert_eq!(
                quarter_turned, original,
                "quarter-turn spread mismatch at local ({x}, {y})"
            );
        }
    }
    Ok(())
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
                "native wgpu M9VIS-08 proof unavailable under {requested:?}; structured adapter failure: {detail}"
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
