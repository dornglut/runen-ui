#![allow(refining_impl_trait)]
#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

use core::{future::Future, pin::pin, task::Poll};
use std::{cell::Cell, fmt::Write as _, fs, path::PathBuf, task::Context, time::Instant};

use runenui_core::{
    Brush, Color, CommandOrigin, CompositionRange, EdgeInsets, EditIntent, EditResolution,
    EditableContribution, EditingSessionPolicy, Element, FontFamilyName, GenericFontFamily,
    HitContribution, HitContributionContext, ImageDescriptor, ImageIntrinsicSize, ImageMapping,
    ImagePaintDescriptor, LayoutDimension, LayoutStyle, LogicalLength, LogicalPoint, LogicalRect,
    LogicalSize, NoHostProtocol, OverflowPolicy, OverflowStyle, PaintContribution,
    PaintContributionContext, PaintContributionItem, PaintPrimitive, PointerDeviceKind,
    PointerEvent, PointerId, PointerPhase, ResourceKind, ResourceRef, SceneShape, SemanticAction,
    SemanticContribution, SemanticContributionContext, SemanticEditable, SemanticNodeContribution,
    SemanticRole, SemanticState, StyleEnvironment, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, TextSensitivity,
    UiApp, UpdateOutput, View, Widget, WidgetActivation, WidgetMeasure, WidgetMeasureInput,
    WidgetTextInput,
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
const EVIDENCE_GAP: u32 = 8;
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

    fn paint(&self, (): &Self::State, _context: PaintContributionContext) -> PaintContribution {
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
        PaintContribution::single(image)
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
        editor
            .id("m10.editor")
            .background(Color::rgb(0x16, 0x20, 0x30))
            .with_layout(
                LayoutStyle::default()
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

fn evidence_dir() -> Option<PathBuf> {
    std::env::var_os("RUNENUI_M10F_EVIDENCE_DIR").map(PathBuf::from)
}

fn write_evidence(
    renderer: &Renderer,
    extent: (u32, u32),
    panels: [(&str, &[u8]); 4],
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(directory) = evidence_dir() else {
        return Ok(());
    };
    fs::create_dir_all(&directory)?;

    let (panel_width, panel_height) = extent;
    let width = panel_width
        .checked_mul(2)
        .and_then(|width| width.checked_add(EVIDENCE_GAP))
        .ok_or("M10F evidence width overflow")?;
    let height = panel_height
        .checked_mul(2)
        .and_then(|height| height.checked_add(EVIDENCE_GAP))
        .ok_or("M10F evidence height overflow")?;
    let byte_count = usize::try_from(width)?
        .checked_mul(usize::try_from(height)?)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("M10F evidence buffer size overflow")?;
    let mut sheet = vec![0_u8; byte_count];
    for pixel in sheet.as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&[0x16, 0x20, 0x30, 0xFF]);
    }

    let row_bytes = usize::try_from(panel_width)? * 4;
    let panel_bytes = row_bytes * usize::try_from(panel_height)?;
    for (index, (name, source)) in panels.iter().enumerate() {
        if source.len() != panel_bytes {
            return Err(format!(
                "M10F panel {} has {} bytes, expected {panel_bytes}",
                name,
                source.len()
            )
            .into());
        }
        let x = if index % 2 == 0 {
            0
        } else {
            panel_width + EVIDENCE_GAP
        };
        let y = if index < 2 {
            0
        } else {
            panel_height + EVIDENCE_GAP
        };
        for row in 0..usize::try_from(panel_height)? {
            let source_start = row * row_bytes;
            let target_start =
                (usize::try_from(y)? + row) * usize::try_from(width)? * 4 + usize::try_from(x)? * 4;
            sheet[target_start..target_start + row_bytes]
                .copy_from_slice(&source[source_start..source_start + row_bytes]);
        }
    }

    image::save_buffer(
        directory.join("m10f-editable-contact-sheet.png"),
        &sheet,
        width,
        height,
        image::ColorType::Rgba8,
    )?;

    let mut manifest = String::from("M10F real-wgpu editable rendering evidence\n\n");
    writeln!(
        manifest,
        "adapter={:?}",
        renderer.diagnostics().adapter_info()
    )?;
    writeln!(
        manifest,
        "publication readback extent={panel_width}x{panel_height}"
    )?;
    manifest.push_str("panel order: initial selection | focused caret; preedit | scrolled text\n");
    manifest.push_str(
        "Each panel is the real renderer readback of a correlated runtime publication; the viewport retains its clip and presentation transform.\n",
    );
    manifest.push_str(
        "Proof also checks candidate geometry against the painted caret, retry after resource failure, and renderer re-realization of the exact publication.\n",
    );
    for (name, _) in panels {
        writeln!(manifest, "panel={name}")?;
    }
    fs::write(directory.join("m10f-editable-evidence.txt"), manifest)?;
    Ok(())
}

#[test]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "sample points are clipped to the finite RGBA readback extent before pixel indexing"
)]
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
    let initial_text_item = initial
        .paint_scene()
        .items()
        .iter()
        .find(|item| item.primitive().as_shaped_text_run().is_some())
        .unwrap_or_else(|| unreachable!("editable text has one retained shaped run"));
    let initial_text_transform = initial_text_item.local_to_surface();
    let initial_clip_transforms = initial_text_item
        .clips()
        .iter()
        .map(runenui_runtime::SceneClip::clip_to_surface)
        .collect::<Vec<_>>();
    let initial_readback =
        renderer.render_offscreen_publication(initial.paint_publication(), &provider)?;
    let evidence_extent = initial_readback.readback().extent();
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

    let selection_anchor_offset = TEXT.find("scrolled line five").map_or_else(
        || unreachable!("fixture contains the fifth line"),
        |offset| offset + "scrolled line five".len(),
    );
    let selection_active_offset = TEXT
        .find("scrolled line four")
        .unwrap_or_else(|| unreachable!("fixture contains the fourth line"));
    let selection_snapshot =
        TextDocumentSnapshot::new(TextDocumentId::new(10), TextDocumentRevision::new(0));
    let selection_anchor = TextPosition::new(
        selection_snapshot,
        TEXT,
        selection_anchor_offset,
        TextAffinity::Downstream,
    )?;
    let selection_active = TextPosition::new(
        selection_snapshot,
        TEXT,
        selection_active_offset,
        TextAffinity::Upstream,
    )?;
    runtime.submit_semantic_action(runenui_core::SemanticActionRequest::set_selection(
        focused
            .semantic_publication()
            .snapshot()
            .surface_id()
            .clone(),
        editable_node.id().clone(),
        TextSelection::new(selection_anchor, selection_active)?,
    ))?;
    runtime.pump(full_pump());

    let composition = runtime
        .start_composition(None)
        .unwrap_or_else(|error| panic!("focused editor accepts composition: {error:?}"));
    let generation = composition.generation().clone();
    let cancel_generation = generation.clone();
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
                LogicalDelta::new(0.0, 96.0)
                    .unwrap_or_else(|_| unreachable!("fixture wheel delta is finite")),
            ),
        )
        .unwrap_or_else(|error| panic!("wheel input is admitted: {error:?}"));
    runtime.pump(full_pump());
    let scrolled = publish(&mut runtime);
    assert_eq!(scrolled.hit_test_scene().target_at(point), Some(&owner));
    let scrolled_text_item = scrolled
        .paint_scene()
        .items()
        .iter()
        .find(|item| item.primitive().as_shaped_text_run().is_some())
        .unwrap_or_else(|| unreachable!("scrolled editable text keeps its retained run"));
    assert_ne!(
        scrolled_text_item.local_to_surface(),
        initial_text_transform,
        "same-owner scrolling moves text through the viewport"
    );
    let origin =
        LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!("local origin is finite"));
    let initial_text_origin = initial_text_transform
        .transform_point(origin)
        .unwrap_or_else(|| unreachable!("initial text origin is representable"));
    let scrolled_text_origin = scrolled_text_item
        .local_to_surface()
        .transform_point(origin)
        .unwrap_or_else(|| unreachable!("scrolled text origin is representable"));
    assert!(
        scrolled_text_origin.y() < initial_text_origin.y() - 20.0,
        "positive scroll moves same-owner content upward by a visible amount: initial={initial_text_origin:?}, scrolled={scrolled_text_origin:?}"
    );
    assert_eq!(
        scrolled_text_item
            .clips()
            .iter()
            .map(runenui_runtime::SceneClip::clip_to_surface)
            .collect::<Vec<_>>(),
        initial_clip_transforms,
        "same-owner scrolling keeps the viewport clip stationary"
    );
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
    let selection_color = Brush::solid(Color::rgba(255, 255, 255, 96));
    let selection_fill = |publication: &runenui_runtime::SurfacePublication| {
        publication.paint_scene().items().iter().find_map(|item| {
            let matching_brush = item.primitive().brush() == Some(&selection_color);
            let rect = match item.primitive() {
                PaintPrimitive::Fill {
                    shape: SceneShape::Rect(rect),
                    ..
                } if matching_brush => *rect,
                _ => return None,
            };
            let local_center = LogicalPoint::new(
                rect.x() + rect.width() / 2.0,
                rect.y() + rect.height() / 2.0,
            )
            .ok()?;
            let surface_center = item.local_to_surface().transform_point(local_center)?;
            let visible = item
                .clips()
                .iter()
                .all(|clip| clip.contains_surface_point(surface_center));
            Some((surface_center, visible))
        })
    };
    let (_unscrolled_selection_center, unscrolled_selection_visible) =
        selection_fill(&preedit_surface)
            .unwrap_or_else(|| unreachable!("pre-scroll text selection is painted"));
    assert!(
        !unscrolled_selection_visible,
        "the off-viewport selection rectangle is clipped before scrolling"
    );
    let (scrolled_selection_center, scrolled_selection_visible) = selection_fill(&scrolled)
        .unwrap_or_else(|| unreachable!("same-node scroll retains selection paint"));
    assert!(
        scrolled_selection_visible,
        "the scrolled selection rectangle is admitted by the stationary viewport clip"
    );
    let sample_pixel = |pixels: &[u8], point: LogicalPoint| -> [u8; 4] {
        let x = point.x().floor() as usize;
        let y = point.y().floor() as usize;
        let offset = (y * evidence_extent.width() as usize + x) * 4;
        pixels[offset..offset + 4]
            .try_into()
            .unwrap_or_else(|_| unreachable!("pixel contains four RGBA channels"))
    };
    assert_ne!(
        sample_pixel(&scrolled_pixels, scrolled_selection_center),
        [0x16_u8, 0x20, 0x30, 0xFF],
        "the real-wgpu readback contains the visible scrolled selection"
    );
    let caret_color = Brush::solid(Color::WHITE);
    let caret_fill = |publication: &runenui_runtime::SurfacePublication| {
        publication.paint_scene().items().iter().find_map(|item| {
            if item.primitive().brush() != Some(&caret_color) {
                return None;
            }
            let rect = match item.primitive() {
                PaintPrimitive::Fill {
                    shape: SceneShape::Rect(rect),
                    ..
                } if rect.width().to_bits() == 1.0_f32.to_bits() => *rect,
                _ => return None,
            };
            let local_center = LogicalPoint::new(
                rect.x() + rect.width() / 2.0,
                rect.y() + rect.height() / 2.0,
            )
            .ok()?;
            let surface_center = item.local_to_surface().transform_point(local_center)?;
            let visible = item
                .clips()
                .iter()
                .all(|clip| clip.contains_surface_point(surface_center));
            Some((surface_center, visible))
        })
    };
    let (_unscrolled_caret_center, unscrolled_caret_visible) =
        caret_fill(&preedit_surface).unwrap_or_else(|| unreachable!("pre-scroll caret is painted"));
    assert!(
        !unscrolled_caret_visible,
        "the off-viewport caret is clipped"
    );
    let (scrolled_caret_center, scrolled_caret_visible) = caret_fill(&scrolled)
        .unwrap_or_else(|| unreachable!("same-node scroll retains caret paint"));
    assert!(
        scrolled_caret_visible,
        "the scrolled caret is inside the viewport clip"
    );
    assert_ne!(
        sample_pixel(&scrolled_pixels, scrolled_caret_center),
        [0x16_u8, 0x20, 0x30, 0xFF],
        "the real-wgpu readback contains the correlated scrolled caret"
    );
    let fixed_corner = |pixels: &[u8]| -> [u8; 4] {
        let offset = ((4_usize * evidence_extent.width() as usize) + 4) * 4;
        pixels[offset..offset + 4]
            .try_into()
            .unwrap_or_else(|_| unreachable!("pixel contains four RGBA channels"))
    };
    assert_eq!(
        fixed_corner(&scrolled_pixels),
        fixed_corner(&preedit_pixels),
        "the style background at the stationary viewport corner is unchanged by scrolling"
    );
    assert_ne!(pixel_hash(&preedit_pixels), pixel_hash(&scrolled_pixels));

    runtime
        .cancel_composition(cancel_generation)
        .unwrap_or_else(|error| {
            panic!("the visual composition fixture cancels cleanly: {error:?}")
        });
    runtime.pump(full_pump());

    let pointer_id = PointerId::new(24).unwrap_or_else(|| unreachable!("pointer ID is nonzero"));
    let click_context = scrolled.input_context().clone();
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                point,
                click_context.clone(),
            )
            .with_buttons(runenui_core::PointerButtons::new([
                runenui_core::PointerButton::Primary,
            ]))
            .with_changed_button(runenui_core::PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("scrolled text click is admitted: {error:?}"));
    runtime.pump(full_pump());
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Up,
                point,
                click_context,
            )
            .with_changed_button(runenui_core::PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("scrolled text release is admitted: {error:?}"));
    runtime.pump(full_pump());
    let clicked = publish(&mut runtime);
    let clicked_selection = clicked
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::EditableText)
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("same-node scrolled text remains editable"))
        .selection();
    assert!(
        clicked_selection.active().byte_offset()
            >= TEXT
                .find("scrolled line four")
                .unwrap_or_else(|| unreachable!("fixture contains a scrolled line")),
        "pointer-to-text mapping shares the scrolled content transform: {clicked_selection:?}"
    );

    let exact_publication = scrolled.paint_publication().clone();
    let expected_revision = exact_publication.revision();
    let expected_text = runtime.state().text.clone();
    let expected_hash = pixel_hash(&scrolled_pixels);
    let mut render_readback_times = Vec::with_capacity(40);
    for _ in 0..40 {
        let started = Instant::now();
        let _ = renderer.render_offscreen_publication(&exact_publication, &provider)?;
        render_readback_times.push(started.elapsed());
    }
    render_readback_times.sort_unstable();
    let render_median_ns = render_readback_times[render_readback_times.len() / 2].as_nanos();
    let render_p95_index = (render_readback_times.len() * 95).div_ceil(100) - 1;
    let render_p95_ns = render_readback_times[render_p95_index].as_nanos();
    eprintln!(
        "issue261_measurement label=renderer_offscreen_submit_readback n={} median_ns={render_median_ns} p95_ns={render_p95_ns}",
        render_readback_times.len()
    );
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
    write_evidence(
        &renderer,
        (evidence_extent.width(), evidence_extent.height()),
        [
            ("initial selection", &initial_pixels),
            ("focused caret", &focused_pixels),
            ("preedit", &preedit_pixels),
            ("scrolled text", &scrolled_pixels),
        ],
    )?;
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
