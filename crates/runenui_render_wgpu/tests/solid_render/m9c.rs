#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{
    cell::Cell,
    fmt::Write as _,
    fs,
    path::Path,
    sync::Arc,
    task::{Context, Wake, Waker},
    thread,
    time::Duration,
};

use runenui_core::{
    Element, ImageCrop, ImageDescriptor, ImageDestinationInsets, ImageIntrinsicSize, ImageMapping,
    ImagePaintDescriptor, ImageSourceInsets, LogicalLength, LogicalRect, MotionEasing,
    MotionTarget, NoHostProtocol, PaintContribution, PaintContributionContext,
    PaintContributionItem, ReducedMotionStrategy, ResourceKind, ResourceRef, SceneOpacity, UiApp,
    Widget, WidgetMeasure, WidgetMeasureInput,
};
use runenui_render_wgpu::{
    BackendSelection, ImagePayload, OffscreenPublicationReadback, PublicationUpdateMode, Renderer,
    RendererInitError, RendererOptions, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, SurfacePublication,
};

const LOGICAL_SIZE: u16 = 32;

#[derive(Clone)]
struct State {
    resource: ResourceRef,
    dimmed: bool,
}

#[derive(Clone, Copy)]
enum Action {
    SetDimmed(bool),
}

struct MotionImageApp;

impl UiApp for MotionImageApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let opacity = if state.dimmed {
            SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("fixture opacity is normalized"))
        } else {
            SceneOpacity::OPAQUE
        };
        Element::new(ImageProbe {
            resource: state.resource.clone(),
        })
        .opacity(opacity)
        .transition(MotionTarget::Opacity, transition_spec())
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::SetDimmed(dimmed) => state.dimmed = dimmed,
        }
    }
}

#[derive(Clone, Debug)]
struct ImageProbe {
    resource: ResourceRef,
}

impl Widget<Action> for ImageProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::from(LOGICAL_SIZE),
            LogicalLength::from(LOGICAL_SIZE),
        )
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let image = ImageDescriptor::new(
            self.resource.clone(),
            ImageIntrinsicSize::new(12, 12)
                .unwrap_or_else(|| unreachable!("fixture source extent is non-zero")),
        )
        .unwrap_or_else(|_| unreachable!("fixture resource is image-kind"));
        let mapping = ImageMapping::NineSlice {
            source: ImageCrop::FULL,
            source_insets: ImageSourceInsets::new(4.0, 4.0, 4.0, 4.0)
                .unwrap_or_else(|_| unreachable!("fixture source insets are valid")),
            destination_insets: ImageDestinationInsets::new(
                LogicalLength::from(8_u16),
                LogicalLength::from(8_u16),
                LogicalLength::from(8_u16),
                LogicalLength::from(8_u16),
            ),
        };
        let descriptor = ImagePaintDescriptor::new(
            image,
            LogicalRect::try_new(0.0, 0.0, f32::from(LOGICAL_SIZE), f32::from(LOGICAL_SIZE))
                .unwrap_or_else(|_| unreachable!("fixture destination is valid")),
            mapping,
        )
        .unwrap_or_else(|_| unreachable!("fixture nine-slice mapping is valid"));
        PaintContribution::single(PaintContributionItem::image(descriptor))
    }
}

fn transition_spec() -> runenui_core::TransitionSpec {
    runenui_core::TransitionSpec::new(
        Duration::from_millis(100),
        Duration::ZERO,
        MotionEasing::Linear,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("M9C visual transition spec is valid"))
}

struct CountingProvider {
    resource: ResourceRef,
    payload: ImagePayload,
    loads: Cell<usize>,
}

impl CountingProvider {
    fn new(resource: ResourceRef) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            resource,
            payload: ImagePayload::new(12, 12, nine_slice_pixels())?,
            loads: Cell::new(0),
        })
    }

    const fn loads(&self) -> usize {
        self.loads.get()
    }
}

impl ResourceProvider for CountingProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        if resource != &self.resource || request != ResourceRequest::Image {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "unexpected M9C visual resource request",
            ));
        }
        Ok(ResourcePayload::Image(self.payload.clone()))
    }
}

fn nine_slice_pixels() -> Vec<u8> {
    let colors = [
        [0xE0, 0x30, 0x30, 0xFF],
        [0xE0, 0x90, 0x30, 0xFF],
        [0xE0, 0xE0, 0x30, 0xFF],
        [0x30, 0xE0, 0x30, 0xFF],
        [0x30, 0xE0, 0xE0, 0xFF],
        [0x30, 0x60, 0xE0, 0xFF],
        [0x80, 0x30, 0xE0, 0xFF],
        [0xD0, 0x30, 0xE0, 0xFF],
        [0xE0, 0x30, 0x80, 0xFF],
    ];
    let mut pixels = Vec::with_capacity(12 * 12 * 4);
    for y in 0..12 {
        for x in 0..12 {
            let zone_x = x / 4;
            let zone_y = y / 4;
            pixels.extend_from_slice(&colors[zone_y * 3 + zone_x]);
        }
    }
    pixels
}

fn publish(runtime: &mut AppRuntime<MotionImageApp>) -> SurfacePublication {
    let environment = runenui_core::StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("M9C visual publication is admitted"))
}

fn sampled_opacity(publication: &SurfacePublication) -> f32 {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("M9C visual fixture has a root"))
        .computed_style()
        .opacity()
        .get()
}

fn resolved_patch_count(publication: &SurfacePublication) -> usize {
    publication
        .paint_scene()
        .items()
        .iter()
        .find_map(|item| item.primitive().as_image())
        .and_then(|image| image.resolved_patch_count())
        .unwrap_or_else(|| unreachable!("M9C publication contains one resolved image"))
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
            eprintln!("M9C visual evidence skipped: {detail}");
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

fn render(
    renderer: &mut Renderer,
    publication: &SurfacePublication,
    provider: &CountingProvider,
) -> Result<OffscreenPublicationReadback, Box<dyn Error>> {
    Ok(renderer.render_offscreen_publication(publication.paint_publication(), provider)?)
}

fn evidence_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("RUNENUI_M8D_EVIDENCE_DIR").map(std::path::PathBuf::from)
}

fn write_evidence(
    directory: &Path,
    renderer: &Renderer,
    initial: &OffscreenPublicationReadback,
    middle: &OffscreenPublicationReadback,
    retry: &OffscreenPublicationReadback,
    final_readback: &OffscreenPublicationReadback,
    provider_loads: usize,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(directory)?;
    let panels = [initial, middle, retry, final_readback];
    let width = panels
        .iter()
        .map(|panel| panel.readback().extent().width())
        .sum::<u32>();
    let height = panels
        .iter()
        .map(|panel| panel.readback().extent().height())
        .max()
        .unwrap_or(0);
    let mut sheet = vec![0_u8; usize::try_from(width)? * usize::try_from(height)? * 4];
    let mut x = 0_usize;
    let sheet_width = usize::try_from(width)?;
    for panel in panels {
        let extent = panel.readback().extent();
        let panel_width = usize::try_from(extent.width())?;
        let panel_height = usize::try_from(extent.height())?;
        let source = panel.readback().rgba8_srgb();
        for row in 0..panel_height {
            let source_start = row * panel_width * 4;
            let source_end = source_start + panel_width * 4;
            let target_start = (row * sheet_width + x) * 4;
            let target_end = target_start + panel_width * 4;
            sheet[target_start..target_end].copy_from_slice(&source[source_start..source_end]);
        }
        x += panel_width;
    }
    image::save_buffer(
        directory.join("m9c-motion-contact-sheet.png"),
        &sheet,
        width,
        height,
        image::ColorType::Rgba8,
    )?;

    let mut manifest = String::new();
    manifest.push_str("M9C real-wgpu sampled-motion evidence\n\n");
    manifest.push_str(
        "Panel order: initial | midpoint | midpoint retry after cache discard | final.\n",
    );
    manifest.push_str(
        "Runtime owns transition sampling and nine-slice resolution; renderer consumes immutable publications only.\n",
    );
    writeln!(
        manifest,
        "adapter={:?}",
        renderer.diagnostics().adapter_info()
    )?;
    writeln!(manifest, "provider_loads={provider_loads}")?;
    writeln!(
        manifest,
        "initial_mode={:?} middle_mode={:?} retry_mode={:?} final_mode={:?}",
        initial.update_plan().mode(),
        middle.update_plan().mode(),
        retry.update_plan().mode(),
        final_readback.update_plan().mode()
    )?;
    fs::write(directory.join("m9c-motion-evidence.txt"), manifest)?;
    Ok(())
}

#[test]
fn real_wgpu_consumes_runtime_sampled_nine_slice_transition_and_retained_retry()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    let resource = ResourceRef::new(ResourceKind::Image);
    let provider = CountingProvider::new(resource.clone())?;
    let mut runtime = AppRuntime::<MotionImageApp>::mount(State {
        resource,
        dimmed: false,
    });

    let initial_publication = publish(&mut runtime);
    assert_eq!(resolved_patch_count(&initial_publication), 9);
    assert!((sampled_opacity(&initial_publication) - 1.0).abs() <= f32::EPSILON);
    let initial = render(&mut renderer, &initial_publication, &provider)?;
    assert_eq!(
        initial.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_eq!(provider.loads(), 1);

    let already_current = render(&mut renderer, &initial_publication, &provider)?;
    assert_eq!(
        already_current.update_plan().mode(),
        PublicationUpdateMode::AlreadyCurrent
    );
    assert_eq!(provider.loads(), 1);
    assert_eq!(
        already_current.readback().rgba8_srgb(),
        initial.readback().rgba8_srgb()
    );

    runtime
        .submit_action(Action::SetDimmed(true))
        .unwrap_or_else(|_| unreachable!("M9C transition action is admitted"));
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
    let transition_start = publish(&mut runtime);
    assert!((sampled_opacity(&transition_start) - 1.0).abs() <= f32::EPSILON);
    let transition_start_readback = render(&mut renderer, &transition_start, &provider)?;
    assert_eq!(
        transition_start_readback.update_plan().mode(),
        PublicationUpdateMode::ExactBaseMatch
    );
    assert_eq!(provider.loads(), 1);
    assert_eq!(
        transition_start_readback.readback().rgba8_srgb(),
        initial.readback().rgba8_srgb(),
        "starting the transition at the current endpoint must preserve realized pixels"
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded M9C visual advance is valid"));
    let middle_publication = publish(&mut runtime);
    assert_eq!(resolved_patch_count(&middle_publication), 9);
    assert!((sampled_opacity(&middle_publication) - 0.75).abs() <= f32::EPSILON);
    let middle = render(&mut renderer, &middle_publication, &provider)?;
    assert_eq!(
        middle.update_plan().mode(),
        PublicationUpdateMode::ExactBaseMatch
    );
    assert_eq!(provider.loads(), 1);
    assert_ne!(
        middle.readback().rgba8_srgb(),
        initial.readback().rgba8_srgb(),
        "real-wgpu output must reflect the runtime-sampled transition"
    );

    assert!(renderer.discard_resource_cache());
    let retry = render(&mut renderer, &middle_publication, &provider)?;
    assert_eq!(
        retry.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_eq!(provider.loads(), 2);
    assert_eq!(
        retry.readback().rgba8_srgb(),
        middle.readback().rgba8_srgb(),
        "retained publication retry must re-realize resources without resampling motion"
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded M9C visual advance is valid"));
    let final_publication = publish(&mut runtime);
    assert!((sampled_opacity(&final_publication) - 0.5).abs() <= f32::EPSILON);
    let final_readback = render(&mut renderer, &final_publication, &provider)?;
    assert_eq!(
        final_readback.update_plan().mode(),
        PublicationUpdateMode::ExactBaseMatch
    );
    assert_eq!(provider.loads(), 2);
    assert_ne!(
        final_readback.readback().rgba8_srgb(),
        middle.readback().rgba8_srgb()
    );

    if let Some(directory) = evidence_dir() {
        write_evidence(
            &directory,
            &renderer,
            &initial,
            &middle,
            &retry,
            &final_readback,
            provider.loads(),
        )?;
    }
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
