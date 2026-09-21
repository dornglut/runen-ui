#![allow(refining_impl_trait)]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use core::{future::Future, pin::pin, task::Poll};
use std::{cell::Cell, task::Context};

use runenui_core::{
    Brush, Color, CommandOrigin, CompositionRange, EdgeInsets, EditIntent, EditResolution,
    EditableContribution, EditingSessionPolicy, Element, FontFamilyName, GenericFontFamily,
    HitContribution, HitContributionContext, ImageDescriptor, ImageIntrinsicSize, ImageMapping,
    ImagePaintDescriptor, LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength,
    LogicalPoint, LogicalRect, LogicalSize, NoHostProtocol, OverflowPolicy, OverflowStyle,
    PaintContribution, PaintContributionContext, PaintContributionItem, PaintPrimitive,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, PresentationOrigin,
    PresentationRotation, PresentationScale, PresentationTransform, PresentationTranslation,
    ResourceKind, ResourceRef, SceneShape, SemanticAction, SemanticContribution,
    SemanticContributionContext, SemanticEditable, SemanticNodeContribution, SemanticRole,
    SemanticState, StyleEnvironment, TextAffinity, TextDocumentId, TextDocumentRevision,
    TextDocumentSnapshot, TextPosition, TextSelection, TextSensitivity, UiApp, UpdateOutput, View,
    Widget, WidgetActivation, WidgetMeasure, WidgetMeasureInput, WidgetTextInput, children, row,
};
use runenui_render_wgpu::{
    BackendSelection, ImagePayload, PublicationRenderError, Renderer, RendererInitError,
    RendererOptions, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, LogicalDelta, PumpBudget, RasterScale, SurfaceBuildContext,
};

const FONT_BYTES: &[u8] = include_bytes!("fixtures/Cantarell-Regular.ttf");
const TEXT: &str = "M10 editable renderer\nselection pixels\npreedit pixels\nscrolled line four\nscrolled line five\ncandidate geometry";

struct EditorState {
    text: String,
    revision: u64,
    image: ResourceRef,
}

enum EditorAction {
    Edit(EditIntent),
}

#[derive(Debug)]
struct EditorWidget {
    snapshot: TextDocumentSnapshot,
    text: String,
    selection: TextSelection,
    image: ResourceRef,
}

impl Widget<EditorAction> for EditorWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::NONE
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<EditorAction>> {
        EditableContribution::new(
            self.snapshot,
            self.text.clone(),
            self.selection,
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            EditorAction::Edit,
        )
        .ok()
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let Some(editable) = SemanticEditable::new(
            self.snapshot,
            &self.text,
            self.selection,
            TextSensitivity::Public,
            false,
        ) else {
            return SemanticContribution::empty();
        };
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::EditableText)
                .with_name("M10 wgpu editable proof")
                .with_state(SemanticState::ENABLED)
                .with_editable(editable)
                .with_action(SemanticAction::SetSelection)
                .with_action(SemanticAction::ReplaceSelection),
        )
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.text.clone(),
        }
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("fixture bounds are finite")),
        )
    }

    fn paint(&self, (): &Self::State, context: PaintContributionContext) -> PaintContribution {
        let size = context.local_size();
        let background = PaintContributionItem::fill(
            SceneShape::rect(
                LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                    .unwrap_or_else(|_| unreachable!("fixture bounds are finite")),
            ),
            Brush::solid(Color::rgb(0x16, 0x20, 0x30)),
        );
        let descriptor = ImageDescriptor::new(
            self.image.clone(),
            ImageIntrinsicSize::new(1, 1)
                .unwrap_or_else(|| unreachable!("fixture image extent is non-zero")),
        )
        .unwrap_or_else(|_| unreachable!("fixture image has an image resource identity"));
        let image = PaintContributionItem::image(
            ImagePaintDescriptor::new(
                descriptor,
                LogicalRect::try_new(2.0, 2.0, 1.0, 1.0)
                    .unwrap_or_else(|_| unreachable!("fixture image rectangle is finite")),
                ImageMapping::default(),
            )
            .unwrap_or_else(|_| unreachable!("fixture image mapping is valid")),
        );
        PaintContribution::new(vec![background, image])
    }
}

struct EditableApp;

impl UiApp for EditableApp {
    type State = EditorState;
    type Action = EditorAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let snapshot = TextDocumentSnapshot::new(
            TextDocumentId::new(10),
            TextDocumentRevision::new(state.revision),
        );
        let anchor = TextPosition::new(snapshot, &state.text, 14, TextAffinity::Downstream)
            .unwrap_or_else(|_| unreachable!("fixture selection anchor is a UTF-8 boundary"));
        let active = TextPosition::new(snapshot, &state.text, 6, TextAffinity::Upstream)
            .unwrap_or_else(|_| unreachable!("fixture active endpoint is a UTF-8 boundary"));
        let selection = TextSelection::new(anchor, active)
            .unwrap_or_else(|_| unreachable!("fixture endpoints share one snapshot"));
        let editor = Element::new(EditorWidget {
            snapshot,
            text: state.text.clone(),
            selection,
            image: state.image.clone(),
        })
        .id("m10.editor")
        .key("m10.editor")
        .focusable(true)
        .foreground(Color::WHITE)
        .padding(EdgeInsets::all(LogicalLength::from(8_u8)))
        .with_layout(
            LayoutStyle::default()
                .with_width(LayoutDimension::length(length(152.0)))
                .with_height(LayoutDimension::length(length(168.0))),
        );
        row(children![editor])
            .id("m10.viewport")
            .presentation(presentation())
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(LayoutDimension::length(length(168.0)))
                    .with_height(LayoutDimension::length(length(72.0)))
                    .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
            )
            .into_element()
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> UpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            EditorAction::Edit(intent) => {
                let request = intent.request().clone();
                state.text.replace_range(
                    intent.replacement().start()..intent.replacement().end(),
                    intent.replacement_text(),
                );
                state.revision += 1;
                UpdateOutput::edit(EditResolution::accepted(
                    request,
                    TextDocumentSnapshot::new(
                        TextDocumentId::new(10),
                        TextDocumentRevision::new(state.revision),
                    ),
                ))
            }
        }
    }
}

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!("fixture length is finite"))
}

fn presentation() -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(3.0, 2.0)
            .unwrap_or_else(|_| unreachable!("fixture translation is finite")),
        PresentationScale::IDENTITY,
        PresentationRotation::ZERO,
        PresentationOrigin::new(
            runenui_core::UnitInterval::ZERO,
            runenui_core::UnitInterval::ZERO,
        ),
    )
}

const fn full_pump() -> PumpBudget {
    PumpBudget::new(64, usize::MAX, usize::MAX, usize::MAX)
}

fn publish(runtime: &mut AppRuntime<EditableApp>) -> runenui_runtime::SurfacePublication {
    let styles = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(
        &styles,
        LayoutConstraints::tight(
            LogicalSize::try_new(168.0, 72.0)
                .unwrap_or_else(|_| unreachable!("fixture viewport is finite")),
        ),
    )
    .with_raster_scale(RasterScale::ONE);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("editable publication succeeds: {error:?}"))
}

fn renderer_or_skip() -> Result<Option<Renderer>, Box<dyn std::error::Error>> {
    match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => Ok(Some(renderer)),
        Err(RendererInitError::AdapterUnavailable {
            requested,
            compatible_surface_required,
            detail,
        }) => {
            assert_eq!(requested, BackendSelection::AllNative);
            assert!(!compatible_surface_required);
            eprintln!("M10F editable renderer proof skipped: {detail}");
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

fn pixel_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn correlated_editable_publication_renders_selection_preedit_scroll_and_retries_exactly()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(mut renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    let mut runtime = AppRuntime::<EditableApp>::mount(EditorState {
        text: TEXT.to_owned(),
        revision: 0,
        image: ResourceRef::new(ResourceKind::Image),
    });
    runtime.register_text_font_bytes(FONT_BYTES.to_vec())?;
    let family = FontFamilyName::new("Cantarell")?;
    runtime.set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[family])?;
    let provider = M10ImageProvider::new(runtime.state().image.clone(), false)?;

    let initial = publish(&mut runtime);
    assert!(
        initial
            .paint_scene()
            .items()
            .iter()
            .any(|item| item.primitive().as_shaped_text_run().is_some())
    );
    let initial_readback =
        renderer.render_offscreen_publication(initial.paint_publication(), &provider)?;
    let initial_pixels = initial_readback.readback().rgba8_srgb().to_vec();
    assert!(initial_pixels.iter().any(|channel| *channel != 0));
    let initial_rect_fill_count = initial
        .paint_scene()
        .items()
        .iter()
        .filter(|item| {
            matches!(
                item.primitive(),
                PaintPrimitive::Fill {
                    shape: SceneShape::Rect(_),
                    ..
                }
            )
        })
        .count();
    assert!(
        initial_rect_fill_count >= 3,
        "the initial non-collapsed selection spans two text lines and paints separately from the widget background: fill_count={initial_rect_fill_count}"
    );

    let owner = initial
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "m10.editor")
        })
        .unwrap_or_else(|| unreachable!("the editable owner is published"))
        .id()
        .clone();
    runtime
        .submit_command(
            owner.clone(),
            runenui_core::SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("public runtime focus request is admitted: {error:?}"));
    runtime.pump(full_pump());
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
    let focused = publish(&mut runtime);
    let editable_node = focused
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::EditableText)
        .unwrap_or_else(|| unreachable!("editable semantics are published"));
    assert!(editable_node.bounds().height() <= 72.0);
    assert!(
        editable_node
            .supported_actions()
            .contains(&SemanticAction::SetSelection)
    );

    let focused_scene = focused.paint_scene();
    let filled_rects = focused_scene
        .items()
        .iter()
        .filter_map(|item| match item.primitive() {
            PaintPrimitive::Fill {
                shape: SceneShape::Rect(rect),
                ..
            } => Some(*rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    let caret = filled_rects
        .iter()
        .find(|rect| rect.width().to_bits() == 1.0_f32.to_bits())
        .copied()
        .unwrap_or_else(|| panic!("runtime paints the logical caret: {filled_rects:?}"));
    assert!(
        filled_rects.len() > initial_rect_fill_count,
        "focused publication adds a caret to the retained non-collapsed selection: initial={initial_rect_fill_count}, focused={filled_rects:?}"
    );
    runtime.pump(full_pump());
    let services = runtime.pending_framework_services();
    let candidate = services
        .iter()
        .find_map(|service| match service.request() {
            runenui_core::FrameworkServiceRequest::InputMethod {
                enabled: true,
                candidate_area: Some(area),
                ..
            } => Some(*area),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "focused editable requests native IME geometry: {:?}",
                services
                    .iter()
                    .map(runenui_runtime::FrameworkServiceRef::request)
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(candidate.x().to_bits(), caret.x().to_bits());
    assert_eq!(candidate.y().to_bits(), caret.y().to_bits());
    assert_eq!(candidate.height().to_bits(), caret.height().to_bits());
    assert_eq!(candidate.width().to_bits(), 0.0_f32.to_bits());

    let focused_readback =
        renderer.render_offscreen_publication(focused.paint_publication(), &provider)?;
    let focused_pixels = focused_readback.readback().rgba8_srgb().to_vec();
    assert_ne!(pixel_hash(&initial_pixels), pixel_hash(&focused_pixels));

    let composition = runtime
        .start_composition(None)
        .unwrap_or_else(|error| panic!("focused editor accepts composition: {error:?}"));
    let generation = composition.generation().clone();
    let preedit = "compose";
    runtime
        .submit_composition_update(
            generation,
            preedit.to_owned(),
            Some(CompositionRange::new(preedit, 2, 5)?),
        )
        .unwrap_or_else(|error| panic!("composition update is admitted: {error:?}"));
    runtime.pump(full_pump());
    let preedit_surface = publish(&mut runtime);
    let preedit_fills = preedit_surface
        .paint_scene()
        .items()
        .iter()
        .filter(|item| matches!(item.primitive(), PaintPrimitive::Fill { .. }))
        .count();
    assert!(
        preedit_fills >= 3,
        "preedit selection, underline, and caret are painted"
    );
    let preedit_readback =
        renderer.render_offscreen_publication(preedit_surface.paint_publication(), &provider)?;
    let preedit_pixels = preedit_readback.readback().rgba8_srgb().to_vec();
    assert_ne!(pixel_hash(&focused_pixels), pixel_hash(&preedit_pixels));

    // Runtime-owned wheel scrolling changes the same published text/caret map;
    // the retained outer clip keeps off-viewport lines out of both pixels and hit.
    let point =
        LogicalPoint::new(12.0, 34.0).unwrap_or_else(|_| unreachable!("fixture pointer is finite"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                PointerId::new(23).unwrap_or_else(|| unreachable!("pointer ID is nonzero")),
                PointerDeviceKind::Mouse,
                PointerPhase::Wheel,
                point,
                preedit_surface.input_context().clone(),
            )
            .with_scroll_delta(
                LogicalDelta::new(0.0, 48.0)
                    .unwrap_or_else(|_| unreachable!("fixture wheel delta is finite")),
            ),
        )
        .unwrap_or_else(|error| panic!("wheel input is admitted: {error:?}"));
    runtime.pump(full_pump());
    let scrolled = publish(&mut runtime);
    assert_eq!(scrolled.hit_test_scene().target_at(point), Some(&owner));
    let scrolled_node = scrolled
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::EditableText)
        .unwrap_or_else(|| unreachable!("scroll preserves the editable semantic owner"));
    assert!(scrolled_node.bounds().height() <= 72.0);
    let scrolled_readback =
        renderer.render_offscreen_publication(scrolled.paint_publication(), &provider)?;
    let scrolled_pixels = scrolled_readback.readback().rgba8_srgb().to_vec();
    assert_ne!(pixel_hash(&preedit_pixels), pixel_hash(&scrolled_pixels));

    let exact_publication = scrolled.paint_publication().clone();
    let expected_revision = exact_publication.revision();
    let expected_text = runtime.state().text.clone();
    let expected_hash = pixel_hash(&scrolled_pixels);
    assert!(renderer.discard_resource_cache());
    provider.set_fail_next(true);
    assert!(matches!(
        renderer.render_offscreen_publication(&exact_publication, &provider),
        Err(PublicationRenderError::Resource { .. })
    ));
    assert_eq!(runtime.state().text, expected_text);
    provider.set_fail_next(false);
    let retry_after_failure =
        renderer.render_offscreen_publication(&exact_publication, &provider)?;
    assert_eq!(
        pixel_hash(retry_after_failure.readback().rgba8_srgb()),
        expected_hash,
        "renderer failure retries the identical retained editing publication"
    );
    assert!(renderer.discard_resource_cache());
    let retry = renderer.render_offscreen_publication(&exact_publication, &provider)?;
    assert_eq!(pixel_hash(retry.readback().rgba8_srgb()), expected_hash);
    assert_eq!(exact_publication.revision(), expected_revision);
    assert_eq!(runtime.state().text, expected_text);
    let Some(mut fresh_renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    let recreated = fresh_renderer.render_offscreen_publication(&exact_publication, &provider)?;
    assert_eq!(pixel_hash(recreated.readback().rgba8_srgb()), expected_hash);
    assert_eq!(
        runtime.state().revision,
        0,
        "renderer work cannot mutate app state"
    );
    Ok(())
}

struct M10ImageProvider {
    expected: ResourceRef,
    image: ImagePayload,
    fail_next: Cell<bool>,
    loads: Cell<usize>,
}

impl M10ImageProvider {
    fn new(
        expected: ResourceRef,
        fail_next: bool,
    ) -> Result<Self, runenui_render_wgpu::PayloadValidationError> {
        Ok(Self {
            expected,
            image: ImagePayload::new(1, 1, vec![0x38, 0x72, 0xA8, 0xFF])?,
            fail_next: Cell::new(fail_next),
            loads: Cell::new(0),
        })
    }

    fn set_fail_next(&self, fail: bool) {
        self.fail_next.set(fail);
    }
}

impl ResourceProvider for M10ImageProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        if resource != &self.expected || request != ResourceRequest::Image {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "M10F renderer requested a foreign resource identity",
            ));
        }
        if self.fail_next.replace(false) {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Unavailable,
                "intentional retained-publication retry proof",
            ));
        }
        Ok(ResourcePayload::Image(self.image.clone()))
    }
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
