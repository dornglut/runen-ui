use std::{
    env,
    fmt::{self, Write as _},
    future::Future,
    path::PathBuf,
    pin::pin,
    sync::{Arc, OnceLock},
    task::{Context, Poll, Wake, Waker},
    thread,
};

mod accessibility;
mod device_identity;
mod framework_services;
mod keyboard_input;
mod mouse_input;
mod proof_trace;
mod text_input;
mod wheel_input;

use accessibility::{AccessibilityEvent, SemanticAdapter};
use device_identity::{DeviceIdentityError, DeviceIdentityMap};
use framework_services::NativeFrameworkServices;
use keyboard_input::{
    KeyboardIngressDiagnostic, KeyboardInputOutcome, KeyboardInputState, NativeKeyTransition,
};
use mouse_input::{
    MouseButtonOutcome, MouseIngressDiagnostic, MouseInputState, TranslatedPointerPoint,
};
use runenui_core::{
    Color, CommandOrigin, CommittedTextEvent, DragDropEvent, DragDropPayloadKind,
    DragDropPayloadMetadata, DragDropPhase, EdgeInsets, EditChangeMap, EditIntent, EditKind,
    EditResolution, EditSelection, EditableContribution, EditingSessionPolicy, Element,
    EventContext, HitContribution, HitContributionContext, InputDeviceId, KeyModifiers,
    KeyboardEvent, LayoutDimension, LayoutStyle, LogicalLength, LogicalPoint, LogicalRect,
    NoHostProtocol, OverflowPolicy, OverflowStyle, PointerEvent, SemanticAction, SemanticCommand,
    SemanticContribution, SemanticEditable, SemanticNodeContribution, SemanticRole, SemanticState,
    StyleEnvironment, SurfaceInputContext, TextAffinity, TextDocumentId, TextDocumentRevision,
    TextDocumentSnapshot, TextSelection, TextSensitivity, UiApp, UiEvent, UpdateOutput, View,
    Widget, WidgetActivation, WidgetMeasure, WidgetTextInput,
};
use runenui_render_wgpu::{
    PublicationRenderError, Renderer, RendererOptions, ResourcePayload, ResourceProvider,
    ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RasterScale, RedrawRequest, SubmitCompositionErrorKind,
    SubmitKeyboardErrorKind, SubmitTextErrorKind, SurfaceBuildContext, SurfacePublication,
};
use runenui_winit::touch_input::{TouchIngressDiagnostic, TouchInputState};
use text_input::{TextInputState, keyboard_committed_text_candidate, translate_preedit_range};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, Ime, MouseButton, Touch, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const INITIAL_PHYSICAL_SIZE: PhysicalSize<u32> = PhysicalSize::new(800, 480);
const HOST_PUMP_BUDGET: PumpBudget = PumpBudget::new(64, 64, 64, 64);

fn proof_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| env::var("RUNENUI_REFERENCE_PROOF").is_ok_and(|value| value == "1"))
}

fn proof_record(arguments: fmt::Arguments<'_>) {
    if proof_enabled() {
        eprintln!("RUNENUI_PROOF {arguments}");
    }
}

macro_rules! proof {
    ($($argument:tt)*) => {
        proof_record(format_args!($($argument)*))
    };
}

const fn drag_drop_source(
    request: &runenui_core::FrameworkServiceRequest,
) -> Option<runenui_core::WorkSequence> {
    match request {
        runenui_core::FrameworkServiceRequest::DragDrop { source, .. } => Some(*source),
        _ => None,
    }
}

fn successful_file_drop_admission(
    request: &runenui_core::FrameworkServiceRequest,
    response: &runenui_core::FrameworkServiceResponse,
) -> Option<runenui_core::WorkSequence> {
    match (request, response) {
        (
            runenui_core::FrameworkServiceRequest::DragDrop {
                source,
                phase: DragDropPhase::Drop,
                payload,
                accepted: true,
            },
            runenui_core::FrameworkServiceResponse::DragDrop(Ok(())),
        ) if payload.kind() == DragDropPayloadKind::Files => Some(*source),
        _ => None,
    }
}

#[derive(Debug)]
enum HostEvent {
    Wake,
    Accessibility(AccessibilityEvent),
}

impl From<accesskit_winit::Event> for HostEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::Accessibility(AccessibilityEvent::from(event))
    }
}

const INITIAL_EDITOR_TEXT: &str = "RunenUI M10 Reference Host\n\nThis is an application-owned editable document. Click anywhere and type; try mouse selection, clipboard shortcuts, IME composition, and scrolling.";
const LARGE_DOCUMENT_LINES: usize = 4_000;
const STRESS_DOCUMENT_LINES: usize = 16_000;
const MAX_EDITOR_HISTORY_ENTRIES: usize = 100;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReferenceDocumentPreset {
    Default,
    LargeDocument,
    StressDocument,
}

impl ReferenceDocumentPreset {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        let preset = match arguments.next().as_deref() {
            None => Self::Default,
            Some("--large-document") => Self::LargeDocument,
            Some("--stress-document") => Self::StressDocument,
            Some(argument) => {
                return Err(format!(
                    "unknown reference_winit argument `{argument}`; expected --large-document or --stress-document"
                ));
            }
        };
        if let Some(argument) = arguments.next() {
            return Err(format!(
                "unexpected extra reference_winit argument `{argument}`; choose at most one document preset"
            ));
        }
        Ok(preset)
    }

    fn initial_text(self) -> String {
        match self {
            Self::Default => INITIAL_EDITOR_TEXT.to_owned(),
            Self::LargeDocument => generated_reference_document(LARGE_DOCUMENT_LINES),
            Self::StressDocument => generated_reference_document(STRESS_DOCUMENT_LINES),
        }
    }

    fn initial_state(self) -> DemoState {
        let text = self.initial_text();
        match self {
            Self::Default => DemoState::with_selection(
                text,
                INITIAL_EDITOR_TEXT.len(),
                TextAffinity::Upstream,
            ),
            Self::LargeDocument | Self::StressDocument => {
                DemoState::with_selection(text, 0, TextAffinity::Downstream)
            }
        }
    }
}

fn generated_reference_document(line_count: usize) -> String {
    let mut text = String::with_capacity(line_count.saturating_mul(64));
    for line in 0..line_count {
        if line > 0 {
            text.push('\n');
        }
        let section = line / 200;
        match line % 16 {
            0 => write!(
                text,
                "[section {section:03} line {line:05}] reference marker for navigation"
            ),
            1 => write!(
                text,
                "Plain ASCII text for editing, selection, and typing."
            ),
            2 => write!(
                text,
                "    Indented line with whitespace and caret targets."
            ),
            3 => write!(text, "Combining: cafe\u{301} nai\u{308}ve A\u{30a} grapheme clusters."),
            4 => Ok(()),
            5 => write!(text, "Emoji/ZWJ: 👩‍💻 👨‍👩‍👧‍👦 🚀 with ASCII text."),
            6 => write!(text, "Mixed direction: marker ثم العربية ثم ASCII marker."),
            7 => write!(
                text,
                "Long wrapping line exercises responsive reflow across the reference window width and resize path."
            ),
            8 => write!(text, "Short line for quick navigation."),
            9 => write!(
                text,
                "Numbers 0123456789 and punctuation !?.,:; [] {{}} () +-=/."
            ),
            10 => write!(
                text,
                "Prose for mouse selection, word navigation, and editing."
            ),
            11 => write!(
                text,
                "        Deep indentation provides visible leading-space caret targets."
            ),
            12 => write!(
                text,
                "Resize to exercise wrapping, scrolling, caret geometry, and publication."
            ),
            13 => write!(text, "Unicode: Ελληνικά 日本語 हिन्दी alongside ordinary text."),
            14 => write!(
                text,
                "Near-line marker keeps middle and end navigation predictable."
            ),
            _ => write!(text, "[line {line:05}] end of deterministic pattern marker"),
        }
        .unwrap_or_else(|_| unreachable!("writing to a String is infallible"));
    }
    text
}

struct DemoState {
    text: String,
    revision: u64,
    selection_seed: EditSelection,
    undo: Vec<DemoHistoryEntry>,
    redo: Vec<DemoHistoryEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DemoHistoryEntry {
    text: String,
    selection: EditSelection,
}

impl DemoState {
    fn with_selection(text: String, byte_offset: usize, affinity: TextAffinity) -> Self {
        let selection_seed = EditSelection::collapsed(&text, byte_offset, affinity)
            .unwrap_or_else(|_| unreachable!("reference selection seed is checked"));
        Self {
            text,
            revision: 0,
            selection_seed,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::with_selection(
            INITIAL_EDITOR_TEXT.to_owned(),
            INITIAL_EDITOR_TEXT.len(),
            TextAffinity::Upstream,
        )
    }
}

enum DemoAction {
    Edit(EditIntent),
}

#[derive(Debug)]
struct DemoSurface {
    snapshot: TextDocumentSnapshot,
    text: String,
    selection_seed: EditSelection,
}

impl Widget<DemoAction> for DemoSurface {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn text_input(&self, _state: &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, _state: &Self::State) -> Option<EditableContribution<DemoAction>> {
        let selection = self.selection()?;
        EditableContribution::new(
            self.snapshot,
            self.text.clone(),
            selection,
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            DemoAction::Edit,
        )
        .ok()
    }

    fn semantics(
        &self,
        _state: &Self::State,
        _context: runenui_core::SemanticContributionContext,
    ) -> SemanticContribution {
        let Some(selection) = self.selection() else {
            return SemanticContribution::empty();
        };
        let Some(editable) = SemanticEditable::new(
            self.snapshot,
            &self.text,
            selection,
            TextSensitivity::Public,
            false,
        ) else {
            return SemanticContribution::empty();
        };
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::EditableText)
                .with_name("RunenUI reference editor")
                .with_state(SemanticState::ENABLED)
                .with_editable(editable)
                .with_action(SemanticAction::SetSelection)
                .with_action(SemanticAction::ReplaceSelection),
        )
    }

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, DemoAction>,
    ) -> runenui_core::WidgetEventOutput {
        if event
            .as_drag_drop()
            .is_some_and(|drop| drop.phase() == DragDropPhase::Drop)
        {
            // Dropped file names remain host-owned; the reference editor only
            // exercises exact-target admission and never opens or reads them.
            context.accept_drag_drop();
        }
        runenui_core::WidgetEventOutput::none()
    }

    fn measure(
        &self,
        _state: &Self::State,
        _input: runenui_core::WidgetMeasureInput,
    ) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.text.clone(),
        }
    }

    fn hit_test(&self, _state: &Self::State, context: HitContributionContext) -> HitContribution {
        let origin = LogicalPoint::new(0.0, 0.0)
            .unwrap_or_else(|_| unreachable!("the literal demo hit origin is finite"));
        HitContribution::single_rect(LogicalRect::new(origin, context.local_size()))
    }
}

impl DemoSurface {
    fn selection(&self) -> Option<TextSelection> {
        self.selection_seed.bind(self.snapshot, &self.text).ok()
    }
}

struct DemoApp;

impl UiApp for DemoApp {
    type State = DemoState;
    type Action = DemoAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(DemoSurface {
            snapshot: TextDocumentSnapshot::new(
                TextDocumentId::new(1),
                TextDocumentRevision::new(state.revision),
            ),
            text: state.text.clone(),
            selection_seed: state.selection_seed,
        })
        .id("reference.editor")
        .key("reference.editor")
        .with_layout(
            LayoutStyle::default()
                .with_width(LayoutDimension::Fill)
                .with_height(LayoutDimension::Fill)
                .with_overflow(OverflowStyle::new(
                    OverflowPolicy::Visible,
                    OverflowPolicy::Scroll,
                )),
        )
        .foreground(Color::WHITE)
        .background(Color::rgb(28, 32, 40))
        .padding(EdgeInsets::all(LogicalLength::from(24_u16)))
        .focusable(true)
    }

    fn update(
        state: &mut Self::State,
        DemoAction::Edit(intent): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        let request = intent.request().clone();
        let kind = intent.kind();
        let current_snapshot = TextDocumentSnapshot::new(
            TextDocumentId::new(1),
            TextDocumentRevision::new(state.revision),
        );
        if matches!(kind, EditKind::Undo) && state.undo.is_empty()
            || matches!(kind, EditKind::Redo) && state.redo.is_empty()
        {
            // An empty application-owned history operation is accepted without
            // mutating text, revision, selection, or either history stack.
            return UpdateOutput::edit(EditResolution::accepted(request, current_snapshot));
        }
        let Some(next_revision) = state.revision.checked_add(1) else {
            let snapshot = TextDocumentSnapshot::new(
                TextDocumentId::new(1),
                TextDocumentRevision::new(state.revision),
            );
            return UpdateOutput::edit(EditResolution::rejected(request, snapshot));
        };

        let (next_text, next_selection) = match kind {
            EditKind::Undo => {
                push_editor_history(
                    &mut state.redo,
                    DemoHistoryEntry {
                        text: state.text.clone(),
                        selection: intent.proposed_selection(),
                    },
                );
                let target = state
                    .undo
                    .pop()
                    .unwrap_or_else(|| unreachable!("non-empty undo history was preflighted"));
                (target.text, target.selection)
            }
            EditKind::Redo => {
                push_editor_history(
                    &mut state.undo,
                    DemoHistoryEntry {
                        text: state.text.clone(),
                        selection: intent.proposed_selection(),
                    },
                );
                let target = state
                    .redo
                    .pop()
                    .unwrap_or_else(|| unreachable!("non-empty redo history was preflighted"));
                (target.text, target.selection)
            }
            _ => {
                let Some(inverse) = intent.inverse() else {
                    return UpdateOutput::edit(EditResolution::rejected(request, current_snapshot));
                };
                push_editor_history(
                    &mut state.undo,
                    DemoHistoryEntry {
                        text: state.text.clone(),
                        selection: inverse.selection(),
                    },
                );
                state.redo.clear();
                let replacement = intent.replacement();
                let mut next_text = state.text.clone();
                next_text.replace_range(
                    replacement.start()..replacement.end(),
                    intent.replacement_text(),
                );
                (next_text, intent.proposed_selection())
            }
        };
        let resulting_snapshot = TextDocumentSnapshot::new(
            TextDocumentId::new(1),
            TextDocumentRevision::new(next_revision),
        );
        let resolution = if matches!(kind, EditKind::Undo | EditKind::Redo) {
            let replaced = runenui_core::TextRange::new(
                TextDocumentSnapshot::new(
                    TextDocumentId::new(1),
                    TextDocumentRevision::new(state.revision),
                ),
                &state.text,
                0,
                state.text.len(),
            )
            .unwrap_or_else(|_| unreachable!("the application-owned history range is valid"));
            EditResolution::transformed(
                request,
                resulting_snapshot,
                EditChangeMap::new(replaced, next_text.len()),
            )
        } else {
            EditResolution::accepted(request, resulting_snapshot)
        };
        state.text = next_text;
        state.revision = next_revision;
        state.selection_seed = next_selection;
        UpdateOutput::edit(resolution)
    }

    fn trace_action_label(_action: &Self::Action) -> Option<&'static str> {
        Some("edit")
    }
}

fn push_editor_history(history: &mut Vec<DemoHistoryEntry>, entry: DemoHistoryEntry) {
    history.push(entry);
    if history.len() > MAX_EDITOR_HISTORY_ENTRIES {
        let _ = history.remove(0);
    }
}

struct NoResources;

impl ResourceProvider for NoResources {
    fn load(
        &self,
        _resource: &runenui_core::ResourceRef,
        _request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Missing,
            "the reference host has no external resource payloads",
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct NativeMapping {
    physical_size: PhysicalSize<u32>,
    native_scale_factor: f64,
    logical_size: LogicalSize,
    raster_scale: RasterScale,
}

impl NativeMapping {
    #[must_use]
    fn from_window(window: &Window) -> Option<Self> {
        Self::from_parts(window.inner_size(), window.scale_factor())
    }

    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "native f64 geometry is range-checked before conversion into RunenUI's accepted f32 neutral protocol"
    )]
    fn from_parts(physical_size: PhysicalSize<u32>, native_scale_factor: f64) -> Option<Self> {
        if physical_size.width == 0
            || physical_size.height == 0
            || !native_scale_factor.is_finite()
            || native_scale_factor <= 0.0
            || native_scale_factor > f64::from(f32::MAX)
        {
            return None;
        }

        let logical_width = f64::from(physical_size.width) / native_scale_factor;
        let logical_height = f64::from(physical_size.height) / native_scale_factor;
        if !logical_width.is_finite()
            || !logical_height.is_finite()
            || logical_width > f64::from(f32::MAX)
            || logical_height > f64::from(f32::MAX)
        {
            return None;
        }

        let logical_size =
            LogicalSize::try_new(logical_width as f32, logical_height as f32).ok()?;
        let raster_scale = RasterScale::new(native_scale_factor as f32).ok()?;
        Some(Self {
            physical_size,
            native_scale_factor,
            logical_size,
            raster_scale,
        })
    }
}

#[derive(Clone, Debug)]
struct PendingFrame {
    publication: SurfacePublication,
    mapping: NativeMapping,
}

#[derive(Clone, Debug)]
struct DisplayedFrame {
    input_context: SurfaceInputContext,
    mapping: NativeMapping,
}

impl DisplayedFrame {
    #[must_use]
    fn from_pending(pending: &PendingFrame) -> Self {
        Self {
            input_context: pending.publication.input_context().clone(),
            mapping: pending.mapping,
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "native f64 pointer coordinates are finite and f32-range-checked before conversion into RunenUI logical coordinates"
    )]
    fn translate_cursor(
        &self,
        current_mapping: Option<NativeMapping>,
        physical_position: PhysicalPosition<f64>,
    ) -> Result<TranslatedPointerPoint, PointIngressDiagnostic> {
        let current_mapping =
            current_mapping.ok_or(PointIngressDiagnostic::NativeMappingUnavailable)?;
        if current_mapping != self.mapping {
            return Err(PointIngressDiagnostic::DisplayedMappingMismatch);
        }
        if !physical_position.x.is_finite() || !physical_position.y.is_finite() {
            return Err(PointIngressDiagnostic::NonFiniteNativePosition);
        }

        let logical_x = physical_position.x / self.mapping.native_scale_factor;
        let logical_y = physical_position.y / self.mapping.native_scale_factor;
        if !logical_x.is_finite()
            || !logical_y.is_finite()
            || logical_x < f64::from(f32::MIN)
            || logical_x > f64::from(f32::MAX)
            || logical_y < f64::from(f32::MIN)
            || logical_y > f64::from(f32::MAX)
        {
            return Err(PointIngressDiagnostic::LogicalPositionOutOfRange);
        }

        let position = LogicalPoint::new(logical_x as f32, logical_y as f32).unwrap_or_else(|_| {
            unreachable!("translated logical cursor coordinates were validated")
        });
        Ok(TranslatedPointerPoint {
            position,
            input_context: self.input_context.clone(),
            modifiers: KeyModifiers::NONE,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PointIngressDiagnostic {
    NoDisplayedFrame,
    NativeMappingUnavailable,
    DisplayedMappingMismatch,
    CursorPositionUnavailable,
    NonFiniteNativePosition,
    LogicalPositionOutOfRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextIngressDiagnostic {
    NoFocusedRuntimeTarget,
    FocusedTargetNotTextCapable,
    FocusedTargetNotCompositionCapable,
    InvalidNativePreeditRange,
    CompositionNoLongerActive,
}

fn translate_modifiers(state: ModifiersState) -> KeyModifiers {
    let mut modifiers = KeyModifiers::NONE;
    if state.shift_key() {
        modifiers = modifiers.with_shift();
    }
    if state.control_key() {
        modifiers = modifiers.with_control();
    }
    if state.alt_key() {
        modifiers = modifiers.with_alt();
    }
    if state.super_key() {
        modifiers = modifiers.with_meta();
    }
    modifiers
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

fn block_on<FutureType: Future>(future: FutureType) -> FutureType::Output {
    let mut future = pin!(future);
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}

struct ReferenceHost {
    runtime: AppRuntime<DemoApp>,
    trace_sink: Option<runenui_runtime::TraceSinkReceiver>,
    style_environment: StyleEnvironment,
    event_loop_proxy: EventLoopProxy<HostEvent>,
    window: Option<Arc<Window>>,
    accessibility: Option<accesskit_winit::Adapter>,
    semantic_adapter: SemanticAdapter,
    renderer: Option<Renderer>,
    mapping: Option<NativeMapping>,
    pending_redraw: Option<RedrawRequest>,
    pending_frame: Option<PendingFrame>,
    displayed_frame: Option<DisplayedFrame>,
    device_identities: DeviceIdentityMap,
    mouse: MouseInputState,
    touch: TouchInputState,
    keyboard: KeyboardInputState,
    text_input: TextInputState,
    framework_services: NativeFrameworkServices,
    deferred_framework_service_completions: Vec<(
        runenui_runtime::FrameworkServiceToken,
        runenui_core::FrameworkServiceResponse,
        Option<runenui_core::WorkSequence>,
        bool,
    )>,
    modifiers: KeyModifiers,
    last_point_ingress_diagnostic: Option<PointIngressDiagnostic>,
    last_mouse_ingress_diagnostic: Option<MouseIngressDiagnostic>,
    last_keyboard_ingress_diagnostic: Option<KeyboardIngressDiagnostic>,
    last_text_ingress_diagnostic: Option<TextIngressDiagnostic>,
    mapping_publication_needed: bool,
    presentation_suppressed: bool,
    initial_focus_requested: bool,
}

impl ReferenceHost {
    #[must_use]
    fn new(proxy: EventLoopProxy<HostEvent>, state: DemoState) -> Self {
        let (runtime, trace_sink) = proof_trace::mount::<DemoApp>(state, proof_enabled());
        let wake_proxy = proxy.clone();
        runtime.set_wake_transport(move || {
            let _ = wake_proxy.send_event(HostEvent::Wake);
        });
        let host = Self {
            runtime,
            trace_sink,
            style_environment: StyleEnvironment::default(),
            event_loop_proxy: proxy,
            window: None,
            accessibility: None,
            semantic_adapter: SemanticAdapter::new(),
            renderer: None,
            mapping: None,
            pending_redraw: None,
            pending_frame: None,
            displayed_frame: None,
            device_identities: DeviceIdentityMap::default(),
            mouse: MouseInputState::default(),
            touch: TouchInputState::default(),
            keyboard: KeyboardInputState::default(),
            text_input: TextInputState::default(),
            framework_services: NativeFrameworkServices::new(),
            deferred_framework_service_completions: Vec::new(),
            modifiers: KeyModifiers::NONE,
            last_point_ingress_diagnostic: None,
            last_mouse_ingress_diagnostic: None,
            last_keyboard_ingress_diagnostic: None,
            last_text_ingress_diagnostic: None,
            mapping_publication_needed: false,
            presentation_suppressed: false,
            initial_focus_requested: false,
        };
        host.drain_runtime_trace();
        host
    }

    fn drain_runtime_trace(&self) {
        proof_trace::drain(self.trace_sink.as_ref());
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, detail: &str) {
        eprintln!("reference_winit fatal: {detail}");
        let _ = self.runtime.shutdown();
        self.framework_services
            .reset_native_window_ime(self.window.as_deref());
        self.framework_services.shutdown();
        self.deferred_framework_service_completions.clear();
        self.drain_runtime_trace();
        event_loop.exit();
    }

    fn note_point_ingress_diagnostic(&mut self, diagnostic: PointIngressDiagnostic) {
        if self.last_point_ingress_diagnostic != Some(diagnostic) {
            eprintln!("reference_winit point ingress withheld: {diagnostic:?}");
        }
        self.last_point_ingress_diagnostic = Some(diagnostic);
    }

    fn note_mouse_ingress_diagnostic(&mut self, diagnostic: MouseIngressDiagnostic) {
        if self.last_mouse_ingress_diagnostic != Some(diagnostic) {
            eprintln!("reference_winit mouse ingress withheld: {diagnostic:?}");
        }
        self.last_mouse_ingress_diagnostic = Some(diagnostic);
    }

    fn note_keyboard_ingress_diagnostic(&mut self, diagnostic: KeyboardIngressDiagnostic) {
        if self.last_keyboard_ingress_diagnostic != Some(diagnostic) {
            eprintln!("reference_winit keyboard ingress withheld: {diagnostic:?}");
        }
        self.last_keyboard_ingress_diagnostic = Some(diagnostic);
    }

    fn note_text_ingress_diagnostic(&mut self, diagnostic: TextIngressDiagnostic) {
        if self.last_text_ingress_diagnostic != Some(diagnostic) {
            eprintln!("reference_winit text ingress withheld: {diagnostic:?}");
        }
        self.last_text_ingress_diagnostic = Some(diagnostic);
    }

    fn resolve_native_device_id(
        &mut self,
        event_loop: &ActiveEventLoop,
        native: DeviceId,
    ) -> Option<InputDeviceId> {
        match self.device_identities.resolve(native) {
            Ok(device_id) => Some(device_id),
            Err(DeviceIdentityError::Exhausted) => {
                self.fail(event_loop, "native input device identity space exhausted");
                None
            }
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.window.is_some() {
            return Ok(());
        }
        let attributes = Window::default_attributes()
            .with_title("RunenUI M10 reference editor")
            .with_inner_size(INITIAL_PHYSICAL_SIZE)
            .with_visible(false);
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| format!("native window creation failed: {error}"))?;
        let activation_handler = self.semantic_adapter.activation_handler();
        let accessibility = accesskit_winit::Adapter::with_mixed_handlers(
            event_loop,
            &window,
            activation_handler,
            self.event_loop_proxy.clone(),
        );
        proof!("stage=accessibility_adapter_installed_before_show");
        self.framework_services
            .reset_native_window_ime(Some(&window));
        self.window = Some(Arc::new(window));
        self.accessibility = Some(accessibility);
        proof!("stage=window_created");
        Ok(())
    }

    fn ensure_renderer(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.renderer.is_some() {
            return Ok(());
        }
        let window = self
            .window
            .clone()
            .ok_or_else(|| "renderer creation requires the host-owned window".to_owned())?;
        let display = event_loop.owned_display_handle();
        let renderer = block_on(Renderer::request_with_surface_target(
            RendererOptions::new(),
            Box::new(display),
            window,
        ))
        .map_err(|error| format!("native renderer creation failed: {error}"))?;
        let diagnostics = renderer.diagnostics();
        let adapter = diagnostics.adapter_info();
        proof!(
            "stage=renderer_created adapter_name={:?} backend={:?} device_type={:?} surface_format={:?}",
            adapter.name,
            adapter.backend,
            adapter.device_type,
            diagnostics.surface_format()
        );
        self.renderer = Some(renderer);
        Ok(())
    }

    fn refresh_mapping(&mut self) -> bool {
        let next = self.window.as_deref().and_then(NativeMapping::from_window);
        if next == self.mapping {
            return false;
        }
        self.mapping = next;
        self.pending_frame = None;
        self.mapping_publication_needed = next.is_some();
        match next {
            Some(mapping) => proof!(
                "stage=mapping_changed physical={}x{} native_scale={} logical={}x{} raster_scale={}",
                mapping.physical_size.width,
                mapping.physical_size.height,
                mapping.native_scale_factor,
                mapping.logical_size.width(),
                mapping.logical_size.height(),
                mapping.raster_scale.get()
            ),
            None => proof!("stage=mapping_unavailable"),
        }
        true
    }

    fn configure_renderer(&mut self, force: bool) -> Result<(), String> {
        let Some(mapping) = self.mapping else {
            return Ok(());
        };
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| "surface configuration requires the renderer".to_owned())?;
        let configured_matches = renderer.configured_surface_extent().is_some_and(|extent| {
            extent.width() == mapping.physical_size.width
                && extent.height() == mapping.physical_size.height
        });
        if force || !configured_matches {
            renderer
                .configure_surface(mapping.physical_size.width, mapping.physical_size.height)
                .map_err(|error| format!("native surface configuration failed: {error}"))?;
            proof!(
                "stage=surface_configured physical={}x{} force={force}",
                mapping.physical_size.width,
                mapping.physical_size.height
            );
        }
        Ok(())
    }

    fn renderer_addresses_mapping(&self, mapping: NativeMapping) -> bool {
        self.renderer
            .as_ref()
            .and_then(Renderer::configured_surface_extent)
            .is_some_and(|extent| {
                extent.width() == mapping.physical_size.width
                    && extent.height() == mapping.physical_size.height
            })
    }

    fn sync_runtime_text_input(&mut self) {
        let focused_owner = self.runtime.focus().focused_node().cloned();
        let capability = self.runtime.focused_text_input_capability();
        let sync = self.text_input.sync_runtime(focused_owner, capability);
        if sync.reset_native_ime() {
            proof!("stage=native_composition_generation_retired");
        }
    }

    fn pump_runtime_once(&mut self) {
        let _ = self.runtime.pump(HOST_PUMP_BUDGET);
        self.retry_deferred_framework_service_completions();
        let _ = self.runtime.pump(HOST_PUMP_BUDGET);
        self.execute_pending_framework_services();
        let _ = self.runtime.pump(HOST_PUMP_BUDGET);
        self.drain_runtime_trace();
        self.sync_runtime_text_input();
        for (source, paths) in self.framework_services.take_admitted_drop_batches() {
            proof!(
                "stage=native_drop_admitted source={} items={}",
                source.get(),
                paths.len()
            );
        }
    }

    fn retry_deferred_framework_service_completions(&mut self) {
        let deferred = core::mem::take(&mut self.deferred_framework_service_completions);
        for (token, response, drop_source, admit_drop_paths) in deferred {
            match self.runtime.complete_framework_service(&token, response) {
                Ok(_) => {
                    if admit_drop_paths && let Some(source) = drop_source {
                        self.framework_services.commit_drop_path_admission(source);
                    }
                    proof!("stage=framework_service_completion_retry_queued");
                }
                Err(runenui_runtime::FrameworkServiceResponseError::Full(response)) => {
                    self.deferred_framework_service_completions.push((
                        token,
                        response,
                        drop_source,
                        admit_drop_paths,
                    ));
                }
                Err(runenui_runtime::FrameworkServiceResponseError::Stale(_)) => {
                    if let Some(source) = drop_source {
                        self.discard_drop_path_custody(source);
                    }
                    proof!("stage=framework_service_completion_discarded reason=stale");
                }
                Err(
                    runenui_runtime::FrameworkServiceResponseError::Closed(_)
                    | runenui_runtime::FrameworkServiceResponseError::Terminal { .. },
                ) => {
                    if let Some(source) = drop_source {
                        self.discard_drop_path_custody(source);
                    }
                    proof!("stage=framework_service_completion_discarded reason=closed");
                }
                Err(
                    runenui_runtime::FrameworkServiceResponseError::ForeignRuntime(_)
                    | runenui_runtime::FrameworkServiceResponseError::MismatchedKind(_),
                ) => {
                    if let Some(source) = drop_source {
                        self.discard_drop_path_custody(source);
                    }
                    proof!("stage=framework_service_completion_discarded reason=invalid");
                }
            }
        }
    }

    fn execute_pending_framework_services(&mut self) {
        if !self.deferred_framework_service_completions.is_empty() {
            return;
        }
        let requests = self
            .runtime
            .pending_framework_services()
            .into_iter()
            .map(|service| (service.token(), service.request().clone()))
            .collect::<Vec<_>>();
        for (token, request) in requests {
            let response = self
                .framework_services
                .execute(self.window.as_deref(), &request);
            let outcome = NativeFrameworkServices::response_outcome(&response);
            let admitted_drop_source = successful_file_drop_admission(&request, &response);
            match self.runtime.complete_framework_service(&token, response) {
                Ok(_) => {
                    if let Some(source) = admitted_drop_source {
                        self.framework_services.commit_drop_path_admission(source);
                    }
                    proof!("stage=framework_service_completion_queued outcome={outcome}");
                }
                Err(runenui_runtime::FrameworkServiceResponseError::Full(response)) => {
                    proof!(
                        "stage=framework_service_completion_deferred kind={:?}",
                        response.kind()
                    );
                    self.deferred_framework_service_completions.push((
                        token,
                        response,
                        drag_drop_source(&request),
                        admitted_drop_source.is_some(),
                    ));
                }
                Err(runenui_runtime::FrameworkServiceResponseError::Stale(_)) => {
                    if let Some(source) = drag_drop_source(&request) {
                        self.discard_drop_path_custody(source);
                    }
                    proof!("stage=framework_service_completion_discarded reason=stale");
                }
                Err(
                    runenui_runtime::FrameworkServiceResponseError::Closed(_)
                    | runenui_runtime::FrameworkServiceResponseError::Terminal { .. },
                ) => {
                    if let Some(source) = drag_drop_source(&request) {
                        self.discard_drop_path_custody(source);
                    }
                    proof!("stage=framework_service_completion_discarded reason=closed");
                }
                Err(
                    runenui_runtime::FrameworkServiceResponseError::ForeignRuntime(_)
                    | runenui_runtime::FrameworkServiceResponseError::MismatchedKind(_),
                ) => {
                    if let Some(source) = drag_drop_source(&request) {
                        self.discard_drop_path_custody(source);
                    }
                    proof!("stage=framework_service_completion_discarded reason=invalid");
                }
            }
        }
    }

    fn discard_drop_path_custody(&mut self, source: runenui_core::WorkSequence) {
        self.framework_services.discard_pending_drop_paths(source);
        self.framework_services.discard_admitted_drop_paths(source);
    }

    fn collect_redraw_request(&mut self) {
        if self.pending_redraw.is_none() {
            self.pending_redraw = self.runtime.take_redraw_request();
            if self.pending_redraw.is_some() {
                proof!("stage=redraw_taken");
            }
        }
    }

    fn establish_initial_runtime_focus(&mut self, event_loop: &ActiveEventLoop) -> bool {
        if self.initial_focus_requested || self.runtime.focus().focused_node().is_some() {
            return true;
        }

        let Some(target) = self
            .runtime
            .index()
            .nodes()
            .first()
            .map(|node| node.id().clone())
        else {
            return true;
        };
        if let Err(error) = self.runtime.submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        ) {
            self.fail(
                event_loop,
                &format!("initial runtime focus request failed: {error:?}"),
            );
            return false;
        }
        self.initial_focus_requested = true;
        proof!("stage=initial_runtime_focus_requested");
        self.pump_runtime_once();
        true
    }

    fn publish_if_needed(&mut self) -> Result<bool, String> {
        if self.pending_frame.is_some() {
            return Ok(false);
        }
        let Some(mapping) = self.mapping else {
            return Ok(false);
        };
        if !self.renderer_addresses_mapping(mapping) {
            return Ok(false);
        }

        self.collect_redraw_request();
        if self.pending_redraw.is_none() && !self.mapping_publication_needed {
            return Ok(false);
        }

        let context = SurfaceBuildContext::tight(&self.style_environment, mapping.logical_size)
            .with_raster_scale(mapping.raster_scale);
        let publication = self
            .runtime
            .publish_surface(&context)
            .map_err(|error| format!("surface publication failed: {error:?}"))?;
        let accessibility_update = self
            .semantic_adapter
            .update(publication.semantic_publication());
        for diagnostic in &accessibility_update.diagnostics {
            eprintln!("reference_winit accessibility diagnostic: {diagnostic:?}");
        }
        proof!(
            "stage=accessibility_update mode={:?} tree_id={:?} nodes={} diagnostics={}",
            accessibility_update.mode,
            accessibility_update.tree_update.tree_id,
            accessibility_update.tree_update.nodes.len(),
            accessibility_update.diagnostics.len()
        );
        if let Some(accessibility) = self.accessibility.as_mut() {
            let tree_update = accessibility_update.tree_update;
            accessibility.update_if_active(|| tree_update);
        }
        proof!(
            "stage=surface_published input_context={:?} physical={}x{} native_scale={}",
            publication.input_context(),
            mapping.physical_size.width,
            mapping.physical_size.height,
            mapping.native_scale_factor
        );

        if let Some(request) = self.pending_redraw.take() {
            self.runtime
                .acknowledge_redraw(&request)
                .map_err(|error| format!("redraw acknowledgement failed: {error:?}"))?;
            proof!("stage=redraw_acknowledged");
        }
        self.drain_runtime_trace();
        self.mapping_publication_needed = false;
        self.pending_frame = Some(PendingFrame {
            publication,
            mapping,
        });
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        Ok(true)
    }

    fn drive_runtime(&mut self, _event_loop: &ActiveEventLoop) {
        self.pump_runtime_once();
        self.collect_redraw_request();
    }

    fn submit_pointer_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: PointerEvent,
        stage: &str,
    ) -> bool {
        // Native window events are serialized on this event loop. Submit first,
        // then pump once; callers must not pump the same pointer event again.
        proof!("stage=pointer_translated source={stage:?} event={event:?}");
        if let Err(error) = self.runtime.submit_pointer(event) {
            self.fail(
                event_loop,
                &format!("{stage} could not enter runtime input: {error}"),
            );
            return false;
        }
        self.pump_runtime_once();
        self.collect_redraw_request();
        true
    }

    fn handle_native_drag_drop_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::HoveredFile(_) => {
                self.handle_native_drag_drop(event_loop, DragDropPhase::Hover, None);
            }
            WindowEvent::HoveredFileCancelled => {
                self.handle_native_drag_drop(event_loop, DragDropPhase::Cancel, None);
            }
            WindowEvent::DroppedFile(path) => {
                self.handle_native_drag_drop(event_loop, DragDropPhase::Drop, Some(path));
            }
            _ => {}
        }
    }

    fn handle_native_drag_drop(
        &mut self,
        event_loop: &ActiveEventLoop,
        phase: DragDropPhase,
        path: Option<PathBuf>,
    ) {
        let Some(device_id) = self.mouse.active_device_id() else {
            proof!("stage=native_drop_withheld reason=no_active_pointer_stream phase={phase:?}");
            return;
        };
        let point = match self.translate_latest_cursor() {
            Ok(point) => point,
            Err(diagnostic) => {
                proof!("stage=native_drop_withheld reason={diagnostic:?} phase={phase:?}");
                return;
            }
        };
        let mut event = match self.mouse.cursor_moved(device_id, point) {
            Ok(event) => event,
            Err(diagnostic) => {
                proof!("stage=native_drop_withheld reason={diagnostic:?} phase={phase:?}");
                return;
            }
        };
        let payload = DragDropPayloadMetadata::new(
            DragDropPayloadKind::Files,
            std::num::NonZeroU32::MIN,
            None,
        );
        event = event.with_drag_drop(DragDropEvent::new(phase, payload));

        self.pump_runtime_once();
        proof!("stage=native_drop_translated phase={phase:?} payload=files");
        let submission = match self.runtime.submit_pointer(event) {
            Ok(submission) => submission,
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!("native drag/drop could not enter runtime input: {error}"),
                );
                return;
            }
        };
        let source = submission.sequence();
        if let Some(path) = path
            && let Err(failure) = self.framework_services.stage_drop_paths(source, vec![path])
        {
            proof!("stage=native_drop_path_rejected reason={failure:?}");
        }
        self.pump_runtime_once();
        if phase == DragDropPhase::Drop {
            let service_pending = self
                .runtime
                .pending_framework_services()
                .iter()
                .any(|service| {
                    matches!(
                        service.request(),
                        runenui_core::FrameworkServiceRequest::DragDrop {
                            source: pending_source,
                            ..
                        } if *pending_source == source
                    )
                });
            let completion_deferred = self
                .deferred_framework_service_completions
                .iter()
                .any(|(_, _, pending_source, _)| *pending_source == Some(source));
            if !service_pending && !completion_deferred {
                self.framework_services.discard_pending_drop_paths(source);
            }
        }
        self.request_pending_redraw();
    }

    fn submit_keyboard_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: KeyboardEvent,
        stage: &str,
    ) -> bool {
        proof!("stage=keyboard_translated source={stage:?} event={event:?}");
        match self.runtime.submit_keyboard(event) {
            Ok(_) => true,
            Err(error) if error.kind() == SubmitKeyboardErrorKind::NoFocusedTarget => {
                self.note_keyboard_ingress_diagnostic(
                    KeyboardIngressDiagnostic::NoFocusedRuntimeTarget,
                );
                true
            }
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!("{stage} could not enter runtime input: {error}"),
                );
                false
            }
        }
    }

    fn handle_window_focus(&mut self, event_loop: &ActiveEventLoop, focused: bool) {
        proof!("stage=window_focus focused={focused}");
        if focused {
            self.pump_runtime_once();
            self.text_input.set_window_focused(true);
            if let Some(window) = self.window.as_ref() {
                self.framework_services
                    .set_native_window_focused(window, true);
            }
            return;
        }

        if !self.cancel_native_touch_contacts(event_loop, "native window lost focus") {
            return;
        }
        if !self.cancel_focus_sensitive_input(event_loop, "native window lost focus") {
            return;
        }
        self.text_input.set_window_focused(false);
        if let Some(window) = self.window.as_ref() {
            self.framework_services
                .set_native_window_focused(window, false);
        }
        self.handle_native_point_authority_loss(event_loop, "native window lost focus");
        self.modifiers = KeyModifiers::NONE;
    }

    fn submit_committed_text(
        &mut self,
        event_loop: &ActiveEventLoop,
        text: &str,
        device_id: Option<InputDeviceId>,
        stage: &str,
    ) -> bool {
        if text.is_empty() {
            return true;
        }
        if !self.text_input.accepts_committed_text() {
            self.note_text_ingress_diagnostic(TextIngressDiagnostic::FocusedTargetNotTextCapable);
            return true;
        }
        proof!(
            "stage=committed_text_translated source={stage:?} bytes={} chars={} device_id={device_id:?}",
            text.len(),
            text.chars().count()
        );
        let event = CommittedTextEvent::new(text.to_owned(), device_id)
            .unwrap_or_else(|_| unreachable!("empty committed text was filtered"));
        match self.runtime.submit_text(event) {
            Ok(_) => {
                self.last_text_ingress_diagnostic = None;
                true
            }
            Err(error) if error.kind() == SubmitTextErrorKind::NoFocusedTarget => {
                self.note_text_ingress_diagnostic(TextIngressDiagnostic::NoFocusedRuntimeTarget);
                true
            }
            Err(error) if error.kind() == SubmitTextErrorKind::FocusedTargetNotTextCapable => {
                self.note_text_ingress_diagnostic(
                    TextIngressDiagnostic::FocusedTargetNotTextCapable,
                );
                true
            }
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!("{stage} could not enter committed-text input: {error}"),
                );
                false
            }
        }
    }

    fn start_native_composition(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Option<runenui_core::CompositionGeneration> {
        if let Some(generation) = self.text_input.composition_generation() {
            return Some(generation.clone());
        }
        if !self.text_input.accepts_composition() {
            self.note_text_ingress_diagnostic(
                TextIngressDiagnostic::FocusedTargetNotCompositionCapable,
            );
            return None;
        }
        match self.runtime.start_composition(None) {
            Ok(submission) => {
                let generation = submission.generation().clone();
                self.text_input
                    .remember_composition_generation(generation.clone());
                self.last_text_ingress_diagnostic = None;
                proof!("stage=composition_started generation={}", generation.get());
                eprintln!("reference_winit composition started");
                Some(generation)
            }
            Err(error) if error.kind() == SubmitCompositionErrorKind::NoFocusedTarget => {
                self.note_text_ingress_diagnostic(TextIngressDiagnostic::NoFocusedRuntimeTarget);
                None
            }
            Err(error)
                if error.kind()
                    == SubmitCompositionErrorKind::FocusedTargetNotCompositionCapable =>
            {
                self.note_text_ingress_diagnostic(
                    TextIngressDiagnostic::FocusedTargetNotCompositionCapable,
                );
                None
            }
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!("native composition start could not enter runtime input: {error}"),
                );
                None
            }
        }
    }

    fn retire_stale_native_composition(&mut self) {
        self.text_input.retire_composition();
        proof!("stage=composition_retired reason=stale");
        self.note_text_ingress_diagnostic(TextIngressDiagnostic::CompositionNoLongerActive);
    }

    fn cancel_native_composition(&mut self, event_loop: &ActiveEventLoop, reason: &str) -> bool {
        let Some(generation) = self.text_input.composition_generation().cloned() else {
            return true;
        };
        let generation_value = generation.get();
        match self.runtime.cancel_composition(generation) {
            Ok(_) => {
                self.text_input.retire_composition();
                proof!(
                    "stage=composition_cancelled generation={generation_value} reason={reason:?}"
                );
                eprintln!("reference_winit composition cancelled: {reason}");
                self.pump_runtime_once();
                true
            }
            Err(error)
                if matches!(
                    error.kind(),
                    SubmitCompositionErrorKind::MissingGeneration
                        | SubmitCompositionErrorKind::StaleGeneration
                        | SubmitCompositionErrorKind::NoFocusedTarget
                        | SubmitCompositionErrorKind::FocusedTargetNotCompositionCapable
                ) =>
            {
                self.retire_stale_native_composition();
                true
            }
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!(
                        "native composition cancellation could not enter runtime input: {error}"
                    ),
                );
                false
            }
        }
    }

    fn handle_ime_preedit(
        &mut self,
        event_loop: &ActiveEventLoop,
        preedit: String,
        native_range: Option<(usize, usize)>,
    ) {
        self.pump_runtime_once();
        let Ok(range) = translate_preedit_range(&preedit, native_range) else {
            self.note_text_ingress_diagnostic(TextIngressDiagnostic::InvalidNativePreeditRange);
            return;
        };
        if preedit.is_empty() && self.text_input.composition_generation().is_none() {
            return;
        }
        let Some(generation) = self.start_native_composition(event_loop) else {
            return;
        };
        proof!(
            "stage=composition_update generation={} bytes={} chars={} range={range:?}",
            generation.get(),
            preedit.len(),
            preedit.chars().count()
        );
        match self
            .runtime
            .submit_composition_update(generation, preedit, range)
        {
            Ok(_) => {
                self.last_text_ingress_diagnostic = None;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    SubmitCompositionErrorKind::MissingGeneration
                        | SubmitCompositionErrorKind::StaleGeneration
                ) =>
            {
                self.retire_stale_native_composition();
                return;
            }
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!("native composition update could not enter runtime input: {error}"),
                );
                return;
            }
        }
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn handle_ime_commit(&mut self, event_loop: &ActiveEventLoop, text: &str) {
        self.pump_runtime_once();
        if let Some(generation) = self.text_input.composition_generation().cloned() {
            let generation_value = generation.get();
            match self.runtime.submit_composition_end(generation) {
                Ok(_) => {
                    self.text_input.retire_composition();
                    proof!("stage=composition_ended generation={generation_value}");
                    eprintln!("reference_winit composition ended");
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        SubmitCompositionErrorKind::MissingGeneration
                            | SubmitCompositionErrorKind::StaleGeneration
                    ) =>
                {
                    self.retire_stale_native_composition();
                }
                Err(error) => {
                    self.fail(
                        event_loop,
                        &format!("native composition end could not enter runtime input: {error}"),
                    );
                    return;
                }
            }
        }
        if !self.submit_committed_text(event_loop, text, None, "native IME commit") {
            return;
        }
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn handle_ime_state(&mut self, event_loop: &ActiveEventLoop, ime: Ime) {
        match ime {
            Ime::Enabled => {
                proof!("stage=native_ime state=enabled");
            }
            Ime::Preedit(preedit, range) => self.handle_ime_preedit(event_loop, preedit, range),
            Ime::Commit(text) => self.handle_ime_commit(event_loop, &text),
            Ime::Disabled => {
                proof!("stage=native_ime state=disabled");
                let _ = self.cancel_native_composition(event_loop, "native IME disabled");
            }
        }
    }

    fn invalidate_mouse_point_authority(
        &mut self,
        event_loop: &ActiveEventLoop,
        reason: &str,
    ) -> bool {
        let Some(event) = self.mouse.invalidate_point_authority(self.modifiers) else {
            return true;
        };
        eprintln!("reference_winit mouse stream cancelled: {reason}");
        self.submit_pointer_event(event_loop, event, "native mouse cancellation")
    }

    fn cancel_mouse_for_device_change(
        &mut self,
        event_loop: &ActiveEventLoop,
        reason: &str,
    ) -> bool {
        let Some(event) = self.mouse.cancel_for_device_change(self.modifiers) else {
            return true;
        };
        eprintln!("reference_winit mouse stream cancelled: {reason}");
        self.submit_pointer_event(event_loop, event, "native mouse device transition")
    }

    fn cancel_keyboard_authority(&mut self, event_loop: &ActiveEventLoop, reason: &str) -> bool {
        let events = self
            .keyboard
            .cancel_all(self.modifiers, self.text_input.keyboard_composition_state());
        if events.is_empty() {
            return true;
        }
        eprintln!("reference_winit keyboard lifetimes cancelled: {reason}");
        for event in events {
            if !self.submit_keyboard_event(event_loop, event, "native keyboard cancellation") {
                return false;
            }
        }
        self.pump_runtime_once();
        true
    }

    fn cancel_focus_sensitive_input(&mut self, event_loop: &ActiveEventLoop, reason: &str) -> bool {
        // Keyboard ingress resolves the runtime's focused target at submission time. Settle
        // prior work, then batch keyboard cancels while composition is still active before
        // generation-owned composition cleanup is allowed to pump and change focus.
        proof!(
            "stage=focus_sensitive_cleanup reason={reason:?} composition={:?}",
            self.text_input.keyboard_composition_state()
        );
        self.pump_runtime_once();
        if !self.cancel_keyboard_authority(event_loop, reason) {
            return false;
        }
        self.cancel_native_composition(event_loop, reason)
    }

    fn handle_native_point_authority_loss(&mut self, event_loop: &ActiveEventLoop, reason: &str) {
        proof!("stage=point_authority_lost reason={reason:?}");
        if !self.invalidate_mouse_point_authority(event_loop, reason) {
            return;
        }
        self.request_pending_redraw();
    }

    fn translate_latest_cursor(&self) -> Result<TranslatedPointerPoint, PointIngressDiagnostic> {
        let physical_position = self
            .mouse
            .last_native_position()
            .ok_or(PointIngressDiagnostic::CursorPositionUnavailable)?;
        self.displayed_frame.as_ref().map_or(
            Err(PointIngressDiagnostic::NoDisplayedFrame),
            |displayed| {
                displayed
                    .translate_cursor(self.mapping, physical_position)
                    .map(|point| point.with_modifiers(self.modifiers))
            },
        )
    }

    fn handle_cursor_moved(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_device_id: DeviceId,
        physical_position: PhysicalPosition<f64>,
    ) {
        let Some(device_id) = self.resolve_native_device_id(event_loop, native_device_id) else {
            return;
        };
        if self
            .mouse
            .active_device_id()
            .is_some_and(|active| active != device_id)
            && !self.cancel_mouse_for_device_change(event_loop, "native mouse device changed")
        {
            return;
        }
        self.mouse.note_cursor_position(physical_position);
        let translated = match self.displayed_frame.as_ref() {
            Some(displayed) => displayed
                .translate_cursor(self.mapping, physical_position)
                .map(|point| point.with_modifiers(self.modifiers)),
            None => Err(PointIngressDiagnostic::NoDisplayedFrame),
        };
        let translated = match translated {
            Ok(translated) => translated,
            Err(diagnostic) => {
                self.note_point_ingress_diagnostic(diagnostic);
                return;
            }
        };
        self.last_point_ingress_diagnostic = None;

        let event = match self.mouse.cursor_moved(device_id, translated) {
            Ok(event) => event,
            Err(diagnostic) => {
                self.fail(
                    event_loop,
                    &format!("native mouse stream could not advance: {diagnostic:?}"),
                );
                return;
            }
        };
        self.last_mouse_ingress_diagnostic = None;
        if !self.submit_pointer_event(event_loop, event, "native cursor translation") {
            return;
        }
        self.request_pending_redraw();
    }

    fn handle_mouse_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) {
        let Some(device_id) = self.resolve_native_device_id(event_loop, native_device_id) else {
            return;
        };
        if matches!(state, ElementState::Pressed)
            && self
                .mouse
                .active_device_id()
                .is_some_and(|active| active != device_id)
            && !self.cancel_mouse_for_device_change(event_loop, "native mouse device changed")
        {
            return;
        }

        let translated = match self.translate_latest_cursor() {
            Ok(translated) => {
                self.last_point_ingress_diagnostic = None;
                Some(translated)
            }
            Err(diagnostic) => {
                self.note_point_ingress_diagnostic(diagnostic);
                if !self.invalidate_mouse_point_authority(
                    event_loop,
                    "button transition arrived without matching point authority",
                ) {
                    return;
                }
                None
            }
        };

        let outcome = match self
            .mouse
            .button_input(device_id, state, button, translated)
        {
            Ok(outcome) => outcome,
            Err(diagnostic) => {
                self.fail(
                    event_loop,
                    &format!("native mouse transition could not be represented: {diagnostic:?}"),
                );
                return;
            }
        };
        match outcome {
            MouseButtonOutcome::Submit(event) => {
                self.last_mouse_ingress_diagnostic = None;
                if !self.submit_pointer_event(event_loop, event, "native mouse button translation")
                {
                    return;
                }
            }
            MouseButtonOutcome::Suppressed(diagnostic) => {
                self.note_mouse_ingress_diagnostic(diagnostic);
            }
        }
        self.request_pending_redraw();
    }

    fn handle_native_touch(&mut self, event_loop: &ActiveEventLoop, touch: Touch) {
        let Some(device_id) = self.resolve_native_device_id(event_loop, touch.device_id) else {
            return;
        };
        let translated = self
            .displayed_frame
            .as_ref()
            .map_or(Err(PointIngressDiagnostic::NoDisplayedFrame), |displayed| {
                displayed.translate_cursor(self.mapping, touch.location)
            });
        let translated = match translated {
            Ok(point) => point,
            Err(diagnostic) => {
                self.note_point_ingress_diagnostic(diagnostic);
                proof!(
                    "stage=native_touch_withheld phase={:?} reason={diagnostic:?}",
                    touch.phase
                );
                if touch.phase != TouchPhase::Started {
                    let _ = self.cancel_native_touch_contacts(
                        event_loop,
                        "touch coordinate mapping became unavailable",
                    );
                }
                return;
            }
        };
        let event = match self.touch.transition(
            device_id,
            touch.id,
            touch.phase,
            translated.position,
            translated.input_context,
        ) {
            Ok(event) => event,
            Err(diagnostic) => {
                proof!("stage=native_touch_suppressed diagnostic={diagnostic:?}");
                if matches!(diagnostic, TouchIngressDiagnostic::MovementDeltaOutOfRange) {
                    let _ = self.cancel_native_touch_contacts(
                        event_loop,
                        "touch movement exceeded neutral geometry",
                    );
                }
                return;
            }
        };
        self.last_point_ingress_diagnostic = None;
        proof!("stage=native_touch_translated event={event:?}");
        if !self.submit_pointer_event(event_loop, event, "native touch translation") {
            return;
        }
        self.request_pending_redraw();
    }

    fn cancel_native_touch_contacts(&mut self, event_loop: &ActiveEventLoop, reason: &str) -> bool {
        let events = self.touch.cancel_all();
        proof!(
            "stage=native_touch_cancelled reason={reason:?} count={}",
            events.len()
        );
        for event in events {
            if !self.submit_pointer_event(event_loop, event, "native touch cancellation") {
                return false;
            }
        }
        true
    }

    fn handle_keyboard_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_device_id: DeviceId,
        event: &winit::event::KeyEvent,
        is_synthetic: bool,
    ) {
        let Some(device_id) = self.resolve_native_device_id(event_loop, native_device_id) else {
            return;
        };
        if matches!(event.state, ElementState::Pressed)
            && !is_synthetic
            && self.runtime.focus().focused_node().is_none()
        {
            self.note_keyboard_ingress_diagnostic(
                KeyboardIngressDiagnostic::NoFocusedRuntimeTarget,
            );
            return;
        }

        let composition = self.text_input.keyboard_composition_state();
        let transition = NativeKeyTransition::from_event(event, is_synthetic);
        let outcome = self
            .keyboard
            .key_input(device_id, &transition, self.modifiers, composition);
        match outcome {
            KeyboardInputOutcome::Submit(keyboard_event) => {
                let committed_text = keyboard_committed_text_candidate(
                    event.state,
                    is_synthetic,
                    self.text_input.accepts_committed_text(),
                    composition,
                    keyboard_event.logical_key(),
                    event.text.as_deref(),
                )
                .map(str::to_owned);
                self.last_keyboard_ingress_diagnostic = None;
                if !self.submit_keyboard_event(
                    event_loop,
                    keyboard_event,
                    "native keyboard translation",
                ) {
                    return;
                }
                if let Some(text) = committed_text
                    && !self.submit_committed_text(
                        event_loop,
                        &text,
                        Some(device_id),
                        "native keyboard text",
                    )
                {
                    return;
                }
            }
            KeyboardInputOutcome::Suppressed(diagnostic) => {
                self.note_keyboard_ingress_diagnostic(diagnostic);
                return;
            }
        }
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn prepare_native_target(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        self.create_window(event_loop)?;
        self.ensure_renderer(event_loop)?;
        let _ = self.refresh_mapping();
        self.configure_renderer(false)?;
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
            proof!("stage=window_shown");
        }
        Ok(())
    }

    fn handle_mapping_change(&mut self, event_loop: &ActiveEventLoop) {
        let changed = self.refresh_mapping();
        if changed && !self.cancel_native_touch_contacts(event_loop, "native mapping changed") {
            return;
        }
        if changed && !self.invalidate_mouse_point_authority(event_loop, "native mapping changed") {
            return;
        }
        if changed && let Err(error) = self.configure_renderer(false) {
            self.fail(event_loop, &error);
            return;
        }
        self.drive_runtime(event_loop);
    }

    fn request_pending_redraw(&self) {
        if self.presentation_suppressed {
            return;
        }
        if (self.pending_frame.is_some()
            || self.pending_redraw.is_some()
            || self.mapping_publication_needed)
            && let Some(window) = self.window.as_ref()
        {
            window.request_redraw();
        }
    }

    fn record_presented_frame(&mut self, event_loop: &ActiveEventLoop, pending: &PendingFrame) {
        self.displayed_frame = Some(DisplayedFrame::from_pending(pending));
        self.pending_frame = None;
        proof!(
            "stage=presented input_context={:?} physical={}x{} native_scale={}",
            pending.publication.input_context(),
            pending.mapping.physical_size.width,
            pending.mapping.physical_size.height,
            pending.mapping.native_scale_factor
        );
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn render_pending(&mut self, event_loop: &ActiveEventLoop) {
        self.pump_runtime_once();
        if let Err(error) = self.publish_if_needed() {
            self.fail(event_loop, &error);
            return;
        }

        let Some(pending) = self.pending_frame.clone() else {
            return;
        };
        if self.mapping != Some(pending.mapping) {
            self.mapping_publication_needed = self.mapping.is_some();
            self.pending_frame = None;
            proof!("stage=pending_frame_dropped reason=mapping_changed");
            if let Err(error) = self.publish_if_needed() {
                self.fail(event_loop, &error);
            }
            return;
        }
        if self.presentation_suppressed {
            return;
        }

        let Some(window) = self.window.as_ref() else {
            return;
        };
        proof!(
            "stage=present_attempt input_context={:?} physical={}x{} native_scale={}",
            pending.publication.input_context(),
            pending.mapping.physical_size.width,
            pending.mapping.physical_size.height,
            pending.mapping.native_scale_factor
        );
        let render_result = self
            .renderer
            .as_mut()
            .ok_or(PublicationRenderError::SurfaceUnavailable)
            .and_then(|renderer| {
                renderer.render_surface_publication(
                    pending.publication.paint_publication(),
                    &NoResources,
                    || window.pre_present_notify(),
                )
            });

        match render_result {
            Ok(observation) => {
                if !observation.presented() {
                    self.fail(
                        event_loop,
                        "renderer reported successful native rendering without successful presentation",
                    );
                    return;
                }
                self.record_presented_frame(event_loop, &pending);
            }
            Err(
                error @ (PublicationRenderError::SurfaceTimeout
                | PublicationRenderError::SurfaceOccluded),
            ) => {
                proof!("stage=present_retry reason={error:?}");
                self.request_pending_redraw();
            }
            Err(
                error @ (PublicationRenderError::SurfaceOutdated
                | PublicationRenderError::SurfaceSuboptimal
                | PublicationRenderError::SurfaceNotConfigured),
            ) => {
                proof!("stage=surface_reconfigure reason={error:?}");
                if let Err(error) = self.configure_renderer(true) {
                    self.fail(event_loop, &error);
                } else {
                    self.request_pending_redraw();
                }
            }
            Err(PublicationRenderError::SurfaceLost) => {
                proof!("stage=surface_lost");
                if !self.invalidate_mouse_point_authority(event_loop, "native surface lost") {
                    return;
                }
                self.displayed_frame = None;
                self.renderer = None;
                let recovery = self
                    .ensure_renderer(event_loop)
                    .and_then(|()| self.configure_renderer(false));
                if let Err(error) = recovery {
                    self.fail(event_loop, &error);
                } else {
                    proof!("stage=surface_reconfigured_after_loss");
                    self.request_pending_redraw();
                }
            }
            Err(error) => {
                self.fail(event_loop, &format!("native presentation failed: {error}"));
            }
        }
    }
}

impl ReferenceHost {
    fn handle_accessibility_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        event: AccessibilityEvent,
    ) {
        match event {
            AccessibilityEvent::InitialTreeRequested => {
                proof!("stage=accessibility_initial_tree_requested");
            }
            AccessibilityEvent::AccessibilityDeactivated => {
                proof!("stage=accessibility_deactivated");
            }
            AccessibilityEvent::ActionRequested(request) => {
                proof!(
                    "stage=accessibility_action_received action={:?} tree_id={:?} node={:?}",
                    request.action,
                    request.target_tree,
                    request.target_node
                );
                match self.semantic_adapter.action_request(&request) {
                    Ok(semantic_request) => {
                        proof!(
                            "stage=accessibility_action_translated action={:?}",
                            semantic_request.action()
                        );
                        match self.runtime.submit_semantic_action(semantic_request) {
                            Ok(_) => {
                                proof!("stage=accessibility_action_submitted");
                                self.pump_runtime_once();
                            }
                            Err(error) => {
                                eprintln!(
                                    "reference_winit accessibility action rejected by runtime: {error:?}"
                                );
                                proof!("stage=accessibility_action_runtime_rejected");
                            }
                        }
                    }
                    Err(diagnostic) => {
                        eprintln!("reference_winit accessibility action withheld: {diagnostic:?}");
                        proof!("stage=accessibility_action_rejected");
                    }
                }
            }
        }
        self.drain_runtime_trace();
        self.collect_redraw_request();
    }
}

impl ApplicationHandler<HostEvent> for ReferenceHost {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        proof!("stage=host_resumed");
        if let Err(error) = self.prepare_native_target(event_loop) {
            self.fail(event_loop, &error);
            return;
        }
        self.presentation_suppressed = false;
        if !self.establish_initial_runtime_focus(event_loop) {
            return;
        }
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        proof!("stage=host_suspended");
        if !self.cancel_native_touch_contacts(event_loop, "native host suspended") {
            return;
        }
        if !self.cancel_focus_sensitive_input(event_loop, "native host suspended") {
            return;
        }
        self.text_input.set_window_focused(false);
        if let Some(window) = self.window.as_ref() {
            self.framework_services
                .reset_native_window_ime(Some(window));
        }
        if !self.invalidate_mouse_point_authority(event_loop, "native host suspended") {
            return;
        }
        self.modifiers = KeyModifiers::NONE;
        self.renderer = None;
        self.mapping = None;
        self.pending_frame = None;
        self.displayed_frame = None;
        self.mapping_publication_needed = false;
        self.presentation_suppressed = true;
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: HostEvent) {
        match event {
            HostEvent::Wake => {
                proof!("stage=wake_received");
                self.drive_runtime(event_loop);
            }
            HostEvent::Accessibility(event) => self.handle_accessibility_event(event_loop, event),
        }
        self.request_pending_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
        {
            return;
        }
        if let (Some(window), Some(accessibility)) =
            (self.window.as_ref(), self.accessibility.as_mut())
        {
            accessibility.process_event(window, &event);
            proof!("stage=accessibility_event_processed");
        }
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                proof!("stage=window_exit");
                if !self.cancel_native_touch_contacts(event_loop, "native window destroyed") {
                    return;
                }
                let _ = self.runtime.shutdown();
                self.framework_services
                    .reset_native_window_ime(self.window.as_deref());
                self.framework_services.shutdown();
                self.deferred_framework_service_completions.clear();
                self.drain_runtime_trace();
                event_loop.exit();
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                proof!("stage=native_mapping_event");
                self.handle_mapping_change(event_loop);
                self.request_pending_redraw();
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                self.handle_cursor_moved(event_loop, device_id, position);
            }
            drag_event @ (WindowEvent::HoveredFile(_)
            | WindowEvent::HoveredFileCancelled
            | WindowEvent::DroppedFile(_)) => {
                self.handle_native_drag_drop_event(event_loop, drag_event);
            }
            WindowEvent::CursorLeft { .. } => {
                proof!("stage=cursor_left");
                self.handle_native_point_authority_loss(event_loop, "native cursor left window");
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                self.handle_mouse_input(event_loop, device_id, state, button);
            }
            WindowEvent::MouseWheel {
                device_id, delta, ..
            } => {
                wheel_input::handle_mouse_wheel(self, event_loop, device_id, delta);
            }
            WindowEvent::Touch(touch) => self.handle_native_touch(event_loop, touch),
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                self.handle_keyboard_input(event_loop, device_id, &event, is_synthetic);
            }
            WindowEvent::Ime(ime) => self.handle_ime_state(event_loop, ime),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = translate_modifiers(modifiers.state());
                proof!("stage=modifiers_changed modifiers={:?}", self.modifiers);
            }
            WindowEvent::Focused(focused) => self.handle_window_focus(event_loop, focused),
            WindowEvent::Occluded(occluded) => {
                proof!("stage=window_occluded occluded={occluded}");
                self.presentation_suppressed = occluded;
                if !occluded {
                    self.request_pending_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                proof!("stage=redraw_event");
                self.render_pending(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.request_pending_redraw();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        proof!("stage=host_exiting");
        let _ = self.runtime.shutdown();
        self.framework_services
            .reset_native_window_ime(self.window.as_deref());
        self.framework_services.shutdown();
        self.deferred_framework_service_completions.clear();
        self.drain_runtime_trace();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let preset = ReferenceDocumentPreset::parse(env::args().skip(1))
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    let event_loop = EventLoop::<HostEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let mut host = ReferenceHost::new(proxy, preset.initial_state());
    event_loop.run_app(&mut host)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        AppRuntime, CommandOrigin, CommittedTextEvent, DemoApp, DemoHistoryEntry, DemoState,
        DisplayedFrame, HOST_PUMP_BUDGET, INITIAL_EDITOR_TEXT, KeyboardEvent,
        LARGE_DOCUMENT_LINES, LogicalSize, MAX_EDITOR_HISTORY_ENTRIES, NativeMapping,
        PendingFrame, PointIngressDiagnostic, ReferenceDocumentPreset, STRESS_DOCUMENT_LINES,
        SemanticAdapter, SemanticCommand, StyleEnvironment, SurfaceBuildContext,
        mouse_input::{
            MouseButtonOutcome, MouseIngressDiagnostic, MouseInputState, TranslatedPointerPoint,
            translate_mouse_button,
        },
        push_editor_history, translate_modifiers,
    };
    use runenui_core::{
        InputDeviceId, KeyModifiers, KeyboardPhase, LogicalPoint, PointerButton, PointerPhase,
    };
    use runenui_runtime::{FontSourcePolicy, RuntimeConfig};
    use winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{ElementState, MouseButton},
        keyboard::ModifiersState,
    };

    #[test]
    fn reference_document_preset_parsing_is_explicit_and_bounded() {
        assert_eq!(
            ReferenceDocumentPreset::parse(Vec::<String>::new()),
            Ok(ReferenceDocumentPreset::Default)
        );
        assert_eq!(
            ReferenceDocumentPreset::parse(["--large-document".to_owned()]),
            Ok(ReferenceDocumentPreset::LargeDocument)
        );
        assert_eq!(
            ReferenceDocumentPreset::parse(["--stress-document".to_owned()]),
            Ok(ReferenceDocumentPreset::StressDocument)
        );
        assert!(ReferenceDocumentPreset::parse(["--unknown".to_owned()]).is_err());
        assert!(
            ReferenceDocumentPreset::parse([
                "--large-document".to_owned(),
                "--stress-document".to_owned(),
            ])
            .is_err()
        );
    }

    #[test]
    fn generated_document_presets_are_deterministic_and_match_declared_scale() {
        for (preset, expected_lines, minimum_bytes, maximum_bytes) in [
            (
                ReferenceDocumentPreset::LargeDocument,
                LARGE_DOCUMENT_LINES,
                225_000,
                240_000,
            ),
            (
                ReferenceDocumentPreset::StressDocument,
                STRESS_DOCUMENT_LINES,
                910_000,
                945_000,
            ),
        ] {
            let first = preset.initial_text();
            let second = preset.initial_text();

            assert_eq!(first, second);
            assert_eq!(first.lines().count(), expected_lines);
            assert!((minimum_bytes..=maximum_bytes).contains(&first.len()));
            assert!(first.contains("[section 000 line 00000]"));
            assert!(first.contains("cafe\u{301}"));
            assert!(first.contains("👩‍💻"));
            assert!(first.contains("العربية"));
            assert!(first.contains("Long wrapping line"));

            let state = preset.initial_state();
            assert_eq!(state.text, first);
            assert_eq!(state.selection_seed.byte_range(), 0..0);
        }
    }

    fn input_device(value: u64) -> InputDeviceId {
        InputDeviceId::new(value)
            .unwrap_or_else(|| unreachable!("fixture input device identity is non-zero"))
    }

    fn displayed_frame(mapping: NativeMapping) -> DisplayedFrame {
        let mut runtime = AppRuntime::<DemoApp>::mount(DemoState::default());
        let _ = runtime.pump(HOST_PUMP_BUDGET);
        let style_environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::tight(&style_environment, mapping.logical_size)
            .with_raster_scale(mapping.raster_scale);
        let publication = runtime.publish_surface(&context).unwrap_or_else(|error| {
            unreachable!("fixture surface publication is valid: {error:?}")
        });
        DisplayedFrame {
            input_context: publication.input_context().clone(),
            mapping,
        }
    }

    #[test]
    fn reference_editor_publishes_visible_editable_text_and_accepts_commits() {
        let mut runtime = AppRuntime::<DemoApp>::mount_with_config(
            DemoState::default(),
            RuntimeConfig::default()
                .with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
        );
        let _ = runtime.pump(HOST_PUMP_BUDGET);
        let mapping = NativeMapping::from_parts(PhysicalSize::new(800, 480), 1.0)
            .unwrap_or_else(|| unreachable!("the fixture mapping is valid"));
        let style_environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::tight(&style_environment, mapping.logical_size)
            .with_raster_scale(mapping.raster_scale);
        let publication = runtime.publish_surface(&context).unwrap_or_else(|error| {
            unreachable!("reference editor publication is valid: {error:?}")
        });

        assert!(
            publication
                .paint_scene()
                .items()
                .iter()
                .any(|item| item.primitive().as_shaped_text_run().is_some())
        );
        assert!(
            publication
                .semantic_publication()
                .snapshot()
                .nodes()
                .iter()
                .any(|node| {
                    node.role() == runenui_core::SemanticRole::EditableText
                        && node.editable().is_some()
                })
        );
        let accessibility_update =
            SemanticAdapter::new().update(publication.semantic_publication());
        assert!(
            accessibility_update.diagnostics.is_empty(),
            "reference editor semantic actions are translated without diagnostics: {:?}",
            accessibility_update.diagnostics
        );

        let owner = runtime.index().nodes()[0].id().clone();
        runtime
            .submit_command(
                owner,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("reference editor accepts focus"));
        let _ = runtime.pump(HOST_PUMP_BUDGET);
        runtime
            .submit_text(
                CommittedTextEvent::new("!", None)
                    .unwrap_or_else(|_| unreachable!("the fixture commit is valid")),
            )
            .unwrap_or_else(|_| unreachable!("reference editor accepts committed text"));
        let _ = runtime.pump(HOST_PUMP_BUDGET);

        assert!(runtime.state().text.ends_with('!'));
        assert_eq!(runtime.state().revision, 1);
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Exercises the full native mouse ingress through publication.
    fn native_mouse_drag_selects_preloaded_text_before_keyboard_input() {
        fn expect_ok<T, E: std::fmt::Debug>(result: Result<T, E>, message: &str) -> T {
            assert!(result.is_ok(), "{message}: {:?}", result.as_ref().err());
            result.unwrap_or_else(|_| {
                unreachable!("the assertion above reports the unexpected error")
            })
        }

        let mapping = NativeMapping::from_parts(PhysicalSize::new(800, 480), 1.0)
            .unwrap_or_else(|| unreachable!("fixture native mapping is valid"));
        let mut runtime = AppRuntime::<DemoApp>::mount_with_config(
            DemoState::default(),
            RuntimeConfig::default()
                .with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
        );
        runtime.pump(HOST_PUMP_BUDGET);
        let owner = runtime.index().nodes()[0].id().clone();
        runtime
            .submit_command(
                owner,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("the reference host focuses the editor at startup"));
        runtime.pump(HOST_PUMP_BUDGET);
        assert_eq!(runtime.state().text, INITIAL_EDITOR_TEXT);
        let style_environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::tight(&style_environment, mapping.logical_size)
            .with_raster_scale(mapping.raster_scale);
        let initial = runtime
            .publish_surface(&context)
            .unwrap_or_else(|error| unreachable!("reference editor surface is valid: {error:?}"));
        let initial_selection = initial
            .semantic_publication()
            .snapshot()
            .nodes()
            .first()
            .and_then(|node| node.editable())
            .unwrap_or_else(|| unreachable!("the initial publication includes the editable text"))
            .selection();
        assert!(
            initial_selection.is_collapsed(),
            "startup text begins with an ordinary collapsed caret"
        );
        let surface = initial.input_context().clone();
        let device_id = input_device(41);
        let translated = |position: LogicalPoint| TranslatedPointerPoint {
            position,
            input_context: surface.clone(),
            modifiers: KeyModifiers::NONE,
        };
        let start = LogicalPoint::new(259.230_47, 61.878_906)
            .unwrap_or_else(|_| unreachable!("selection anchor is finite"));
        let end = LogicalPoint::new(377.496_1, 79.593_75)
            .unwrap_or_else(|_| unreachable!("selection focus is finite"));
        let mut mouse = MouseInputState::default();
        let hover = mouse.cursor_moved(device_id, translated(start));
        let hover = expect_ok(hover, "native cursor move is translated");
        expect_ok(runtime.submit_pointer(hover), "native hover is routed");
        runtime.pump(HOST_PUMP_BUDGET);
        let down_surface = runtime
            .publish_surface(&context)
            .unwrap_or_else(|error| {
                unreachable!("the host republishes after hover before pointer-down: {error:?}")
            })
            .input_context()
            .clone();
        let down_point = |position| TranslatedPointerPoint {
            position,
            input_context: down_surface.clone(),
            modifiers: KeyModifiers::NONE,
        };
        let down = mouse.button_input(
            device_id,
            ElementState::Pressed,
            MouseButton::Left,
            Some(down_point(start)),
        );
        let down = expect_ok(down, "native primary press is translated");
        let MouseButtonOutcome::Submit(down) = down else {
            unreachable!("first native primary press is admitted")
        };
        expect_ok(
            runtime.submit_pointer(down),
            "native primary press is routed",
        );
        runtime.pump(HOST_PUMP_BUDGET);
        let drag_surface = runtime
            .publish_surface(&context)
            .unwrap_or_else(|error| {
                unreachable!("the captured drag uses the post-press surface: {error:?}")
            })
            .input_context()
            .clone();
        let drag_point = |position| TranslatedPointerPoint {
            position,
            input_context: drag_surface.clone(),
            modifiers: KeyModifiers::NONE,
        };
        let movement = mouse.cursor_moved(device_id, drag_point(end));
        let movement = expect_ok(movement, "native drag movement is translated");
        expect_ok(
            runtime.submit_pointer(movement),
            "native drag movement is routed",
        );
        runtime.pump(HOST_PUMP_BUDGET);
        let release = mouse.button_input(
            device_id,
            ElementState::Released,
            MouseButton::Left,
            Some(drag_point(end)),
        );
        let release = expect_ok(release, "native primary release is translated");
        let MouseButtonOutcome::Submit(release) = release else {
            unreachable!("matching native primary release is admitted")
        };
        expect_ok(
            runtime.submit_pointer(release),
            "native primary release is routed",
        );
        runtime.pump(HOST_PUMP_BUDGET);

        let selection_publication = runtime.publish_surface(&context);
        let selection_publication =
            expect_ok(selection_publication, "dragged selection republishes");
        let selection = selection_publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .first()
            .and_then(|node| node.editable())
            .unwrap_or_else(|| unreachable!("editable selection remains published"))
            .selection();
        assert!(
            !selection.is_collapsed(),
            "native drag should publish a non-collapsed editable range"
        );
        assert!(selection.active().byte_offset() > selection.anchor().byte_offset());
        let selected_paint_item_count = selection_publication.paint_scene().items().len();

        // A completed drag must release the runtime's selection gesture and native capture so
        // another drag can immediately establish a fresh anchor without a click or keypress.
        let next_surface = selection_publication.input_context().clone();
        let next_point = |position: LogicalPoint| TranslatedPointerPoint {
            position,
            input_context: next_surface.clone(),
            modifiers: KeyModifiers::NONE,
        };
        let second_start = LogicalPoint::new(30.0, 80.0)
            .unwrap_or_else(|_| unreachable!("second drag anchor is finite"));
        let second_end = LogicalPoint::new(160.0, 80.0)
            .unwrap_or_else(|_| unreachable!("second drag focus is finite"));
        let hover = expect_ok(
            mouse.cursor_moved(device_id, next_point(second_start)),
            "second drag hover is translated",
        );
        expect_ok(runtime.submit_pointer(hover), "second drag hover is routed");
        runtime.pump(HOST_PUMP_BUDGET);
        let down = expect_ok(
            mouse.button_input(
                device_id,
                ElementState::Pressed,
                MouseButton::Left,
                Some(next_point(second_start)),
            ),
            "second drag press is translated",
        );
        let MouseButtonOutcome::Submit(down) = down else {
            unreachable!("second primary press is admitted")
        };
        expect_ok(runtime.submit_pointer(down), "second drag press is routed");
        runtime.pump(HOST_PUMP_BUDGET);
        let movement = expect_ok(
            mouse.cursor_moved(device_id, next_point(second_end)),
            "second drag movement is translated",
        );
        expect_ok(
            runtime.submit_pointer(movement),
            "second drag movement is routed",
        );
        runtime.pump(HOST_PUMP_BUDGET);
        let release = expect_ok(
            mouse.button_input(
                device_id,
                ElementState::Released,
                MouseButton::Left,
                Some(next_point(second_end)),
            ),
            "second drag release is translated",
        );
        let MouseButtonOutcome::Submit(release) = release else {
            unreachable!("second matching release is admitted")
        };
        expect_ok(
            runtime.submit_pointer(release),
            "second drag release is routed",
        );
        runtime.pump(HOST_PUMP_BUDGET);
        let second_selection_publication = runtime
            .publish_surface(&context)
            .unwrap_or_else(|error| unreachable!("second selection republishes: {error:?}"));
        let second_selection = second_selection_publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .first()
            .and_then(|node| node.editable())
            .unwrap_or_else(|| unreachable!("editable selection remains published"))
            .selection();
        assert!(
            !second_selection.is_collapsed(),
            "a second native drag must start a fresh text-selection gesture: {second_selection:?}"
        );

        let click_point = LogicalPoint::new(790.0, 470.0)
            .unwrap_or_else(|_| unreachable!("selection-collapse click is finite"));
        let translated_click = TranslatedPointerPoint {
            position: click_point,
            input_context: second_selection_publication.input_context().clone(),
            modifiers: KeyModifiers::NONE,
        };
        let hover = mouse.cursor_moved(device_id, translated_click.clone());
        let hover = expect_ok(hover, "selection-collapse cursor move is translated");
        expect_ok(
            runtime.submit_pointer(hover),
            "selection-collapse hover is routed",
        );
        runtime.pump(HOST_PUMP_BUDGET);
        let down = mouse.button_input(
            device_id,
            ElementState::Pressed,
            MouseButton::Left,
            Some(translated_click.clone()),
        );
        let down = expect_ok(down, "selection-collapse press is translated");
        let MouseButtonOutcome::Submit(down) = down else {
            unreachable!("selection-collapse press is admitted")
        };
        expect_ok(
            runtime.submit_pointer(down),
            "selection-collapse press is routed",
        );
        runtime.pump(HOST_PUMP_BUDGET);
        let up = mouse.button_input(
            device_id,
            ElementState::Released,
            MouseButton::Left,
            Some(translated_click),
        );
        let up = expect_ok(up, "selection-collapse release is translated");
        let MouseButtonOutcome::Submit(up) = up else {
            unreachable!("matching selection-collapse release is admitted")
        };
        expect_ok(
            runtime.submit_pointer(up),
            "selection-collapse release is routed",
        );
        runtime.pump(HOST_PUMP_BUDGET);
        let collapsed_publication = runtime
            .publish_surface(&context)
            .unwrap_or_else(|error| unreachable!("collapsed selection republishes: {error:?}"));
        assert!(
            collapsed_publication.paint_scene().items().len() < selected_paint_item_count,
            "blank-area click should remove the painted selection overlay: selected={selected_paint_item_count}, collapsed={}",
            collapsed_publication.paint_scene().items().len()
        );
        let collapsed_selection = collapsed_publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .first()
            .and_then(|node| node.editable())
            .unwrap_or_else(|| unreachable!("collapsed editable selection remains published"))
            .selection();
        assert!(
            collapsed_selection.is_collapsed(),
            "a native click should collapse the previously dragged editable selection: {collapsed_selection:?}"
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one end-to-end history test verifies empty, edit, selection, undo, and redo behavior"
    )]
    #[allow(
        clippy::panic,
        reason = "panic reports the exact rejected keyboard submission in this test"
    )]
    fn reference_editor_undo_and_redo_restore_application_owned_text_history() {
        let mut runtime = AppRuntime::<DemoApp>::mount_with_config(
            DemoState::default(),
            RuntimeConfig::default()
                .with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
        );
        runtime.pump(HOST_PUMP_BUDGET);
        let owner = runtime.index().nodes()[0].id().clone();
        let initial_selection = runtime.state().selection_seed;
        runtime
            .submit_command(
                owner.clone(),
                SemanticCommand::Undo,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("empty reference undo is accepted"));
        runtime.pump(HOST_PUMP_BUDGET);
        assert_eq!(runtime.state().text, INITIAL_EDITOR_TEXT);
        assert_eq!(runtime.state().revision, 0);
        assert_eq!(runtime.state().selection_seed, initial_selection);
        assert!(runtime.state().undo.is_empty());
        assert!(runtime.state().redo.is_empty());
        runtime
            .submit_command(
                owner.clone(),
                SemanticCommand::Redo,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("empty reference redo is accepted"));
        runtime.pump(HOST_PUMP_BUDGET);
        assert_eq!(runtime.state().text, INITIAL_EDITOR_TEXT);
        assert_eq!(runtime.state().revision, 0);
        assert_eq!(runtime.state().selection_seed, initial_selection);
        assert!(runtime.state().undo.is_empty());
        assert!(runtime.state().redo.is_empty());
        runtime
            .submit_command(
                owner.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("reference editor accepts focus"));
        runtime.pump(HOST_PUMP_BUDGET);
        runtime
            .submit_text(
                CommittedTextEvent::new("Q", None)
                    .unwrap_or_else(|_| unreachable!("fixture text is valid")),
            )
            .unwrap_or_else(|_| unreachable!("reference editor accepts committed text"));
        runtime.pump(HOST_PUMP_BUDGET);
        assert!(runtime.state().text.ends_with('Q'));
        runtime
            .publish_surface(&SurfaceBuildContext::tight(
                &StyleEnvironment::default(),
                LogicalSize::try_new(800.0, 480.0)
                    .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
            ))
            .unwrap_or_else(|error| unreachable!("committed document republishes: {error:?}"));
        runtime
            .submit_keyboard(KeyboardEvent::new(
                KeyboardPhase::Down,
                runenui_core::PhysicalKey::ArrowLeft,
                runenui_core::LogicalKey::ArrowLeft,
                KeyModifiers::NONE.with_shift(),
                false,
                runenui_core::KeyLocation::Standard,
                runenui_core::KeyboardCompositionState::Inactive,
                None,
            ))
            .unwrap_or_else(|error| panic!("reference editor accepts Shift+Left: {error:?}"));
        runtime.pump(HOST_PUMP_BUDGET);
        runtime
            .submit_command(
                owner.clone(),
                SemanticCommand::Undo,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("reference editor accepts undo"));
        runtime.pump(HOST_PUMP_BUDGET);
        assert_eq!(runtime.state().text, INITIAL_EDITOR_TEXT);
        assert_eq!(runtime.state().selection_seed, initial_selection);

        let redo =
            runtime.submit_command(owner, SemanticCommand::Redo, CommandOrigin::programmatic());
        assert!(redo.is_ok(), "reference editor accepts redo: {redo:?}");
        runtime.pump(HOST_PUMP_BUDGET);
        assert_eq!(runtime.state().text, format!("{INITIAL_EDITOR_TEXT}Q"));
        assert_eq!(
            runtime.state().selection_seed.anchor(),
            INITIAL_EDITOR_TEXT.len() + 1
        );
        assert_eq!(
            runtime.state().selection_seed.active(),
            INITIAL_EDITOR_TEXT.len()
        );
        assert_eq!(
            runtime.state().selection_seed.anchor_affinity(),
            runenui_core::TextAffinity::Upstream
        );
        assert_eq!(
            runtime.state().selection_seed.active_affinity(),
            runenui_core::TextAffinity::Downstream
        );
        assert_eq!(runtime.state().revision, 3);
        assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
    }

    #[test]
    fn reference_editor_history_is_bounded_to_the_most_recent_hundred_states() {
        let mut history = Vec::new();
        for index in 0..MAX_EDITOR_HISTORY_ENTRIES + 2 {
            let text = format!("entry-{index}");
            let selection = runenui_core::EditSelection::collapsed(
                &text,
                text.len(),
                runenui_core::TextAffinity::Upstream,
            )
            .unwrap_or_else(|_| unreachable!("history selection is a valid byte boundary"));
            push_editor_history(&mut history, DemoHistoryEntry { text, selection });
        }
        assert_eq!(history.len(), MAX_EDITOR_HISTORY_ENTRIES);
        assert_eq!(history[0].text, "entry-2");
        assert_eq!(history[MAX_EDITOR_HISTORY_ENTRIES - 1].text, "entry-101");
    }

    #[test]
    #[ignore = "opt-in issue 261 latency sampling; run with --ignored --nocapture"]
    #[allow(
        clippy::too_many_lines,
        reason = "one opt-in harness measures the four required input and publication stages"
    )]
    fn issue_261_responsiveness_measurements() {
        use std::time::{Duration, Instant};

        fn summarize(samples: &mut [Duration]) -> (u128, u128) {
            samples.sort_unstable();
            let median = samples[samples.len() / 2].as_nanos();
            let p95_index = (samples.len() * 95).div_ceil(100).saturating_sub(1);
            (median, samples[p95_index].as_nanos())
        }

        fn report(label: &str, samples: &mut [Duration]) {
            let (median, p95) = summarize(samples);
            eprintln!(
                "issue261_measurement label={label} n={} median_ns={median} p95_ns={p95}",
                samples.len()
            );
        }

        let mapping = NativeMapping::from_parts(PhysicalSize::new(800, 480), 1.0)
            .unwrap_or_else(|| unreachable!("measurement mapping is valid"));
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::tight(&environment, mapping.logical_size)
            .with_raster_scale(mapping.raster_scale);
        for (name, extra_lines) in [
            ("short", 0),
            ("medium", 20),
            ("fixture_40_lines", 40),
            ("long", 80),
        ] {
            let mut runtime = AppRuntime::<DemoApp>::mount_with_config(
                DemoState::default(),
                RuntimeConfig::default()
                    .with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
            );
            runtime.pump(HOST_PUMP_BUDGET);
            let owner = runtime.index().nodes()[0].id().clone();
            runtime
                .submit_command(
                    owner.clone(),
                    SemanticCommand::RequestFocus,
                    CommandOrigin::programmatic(),
                )
                .unwrap_or_else(|_| unreachable!("measurement editor accepts focus"));
            runtime.pump(HOST_PUMP_BUDGET);
            if extra_lines > 0 {
                let text =
                    "multiline responsiveness fixture — retained text layout\n".repeat(extra_lines);
                runtime
                    .submit_text(
                        CommittedTextEvent::new(text, None)
                            .unwrap_or_else(|_| unreachable!("measurement text is valid")),
                    )
                    .unwrap_or_else(|_| unreachable!("measurement editor accepts long text"));
                runtime.pump(HOST_PUMP_BUDGET);
            }
            let displayed_publication = runtime
                .publish_surface(&context)
                .unwrap_or_else(|error| unreachable!("measurement surface is valid: {error:?}"));
            let displayed = DisplayedFrame::from_pending(&PendingFrame {
                publication: displayed_publication.clone(),
                mapping,
            });
            let point = PhysicalPosition::new(32.0, 32.0);
            let mut translation = Vec::with_capacity(40);
            for _ in 0..40 {
                let started = Instant::now();
                let _ = displayed.translate_cursor(Some(mapping), point);
                translation.push(started.elapsed());
            }
            report(&format!("{name}.native_translation"), &mut translation);

            let mut typing = Vec::with_capacity(40);
            let mut publication = Vec::with_capacity(40);
            let (mut reshaped, mut relinebroken, mut reused) = (0, 0, 0);
            for _ in 0..40 {
                let started = Instant::now();
                runtime
                    .submit_text(
                        CommittedTextEvent::new("x", None)
                            .unwrap_or_else(|_| unreachable!("measurement commit is valid")),
                    )
                    .unwrap_or_else(|_| unreachable!("measurement text is admitted"));
                runtime.pump(HOST_PUMP_BUDGET);
                typing.push(started.elapsed());
                let started = Instant::now();
                let published = runtime.publish_surface(&context).unwrap_or_else(|error| {
                    unreachable!("measurement publication is valid: {error:?}")
                });
                publication.push(started.elapsed());
                for measurement in published
                    .layout_report()
                    .nodes()
                    .iter()
                    .flat_map(runenui_runtime::SurfaceLayoutNode::text_measurements)
                {
                    match measurement.decision() {
                        runenui_runtime::TextLayoutDecision::Reshaped => reshaped += 1,
                        runenui_runtime::TextLayoutDecision::Relinebroken => relinebroken += 1,
                        runenui_runtime::TextLayoutDecision::Reused => reused += 1,
                    }
                }
            }
            report(&format!("{name}.typing_submit_pump"), &mut typing);
            report(&format!("{name}.text_publication"), &mut publication);
            eprintln!(
                "issue261_measurement label={name}.text_layout_decisions reshaped={reshaped} relinebroken={relinebroken} reused={reused}"
            );

            let mut drag_runtime = AppRuntime::<DemoApp>::mount_with_config(
                DemoState::default(),
                RuntimeConfig::default()
                    .with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
            );
            drag_runtime.pump(HOST_PUMP_BUDGET);
            let drag_owner = drag_runtime.index().nodes()[0].id().clone();
            drag_runtime
                .submit_command(
                    drag_owner,
                    SemanticCommand::RequestFocus,
                    CommandOrigin::programmatic(),
                )
                .unwrap_or_else(|_| unreachable!("drag measurement editor accepts focus"));
            drag_runtime.pump(HOST_PUMP_BUDGET);
            if extra_lines > 0 {
                let text =
                    "multiline responsiveness fixture — retained text layout\n".repeat(extra_lines);
                drag_runtime
                    .submit_text(
                        CommittedTextEvent::new(text, None)
                            .unwrap_or_else(|_| unreachable!("measurement text is valid")),
                    )
                    .unwrap_or_else(|_| unreachable!("drag measurement text is accepted"));
                drag_runtime.pump(HOST_PUMP_BUDGET);
            }
            let drag_surface = drag_runtime
                .publish_surface(&context)
                .unwrap_or_else(|error| unreachable!("drag surface is valid: {error:?}"));
            let drag_context = drag_surface.input_context().clone();
            let pointer_id = runenui_core::PointerId::new(71)
                .unwrap_or_else(|| unreachable!("measurement pointer id is nonzero"));
            let start = LogicalPoint::new(32.0, 32.0)
                .unwrap_or_else(|_| unreachable!("measurement drag starts at finite point"));
            drag_runtime
                .submit_pointer(
                    runenui_core::PointerEvent::new(
                        pointer_id,
                        runenui_core::PointerDeviceKind::Mouse,
                        PointerPhase::Down,
                        start,
                        drag_context.clone(),
                    )
                    .with_buttons(runenui_core::PointerButtons::new([PointerButton::Primary]))
                    .with_changed_button(PointerButton::Primary),
                )
                .unwrap_or_else(|_| unreachable!("measurement drag starts"));
            drag_runtime.pump(HOST_PUMP_BUDGET);
            let mut drag = Vec::with_capacity(40);
            for index in 0..40 {
                let x = if index % 2 == 0 { 160.0 } else { 12.0 };
                let position = LogicalPoint::new(x, 32.0)
                    .unwrap_or_else(|_| unreachable!("measurement drag point is finite"));
                let started = Instant::now();
                drag_runtime
                    .submit_pointer(
                        runenui_core::PointerEvent::new(
                            pointer_id,
                            runenui_core::PointerDeviceKind::Mouse,
                            PointerPhase::Move,
                            position,
                            drag_context.clone(),
                        )
                        .with_buttons(runenui_core::PointerButtons::new([PointerButton::Primary])),
                    )
                    .unwrap_or_else(|_| unreachable!("captured measurement drag is admitted"));
                drag_runtime.pump(HOST_PUMP_BUDGET);
                drag.push(started.elapsed());
            }
            report(&format!("{name}.drag_submit_pump"), &mut drag);
        }
    }

    #[test]
    #[ignore = "opt-in issue 261 bulk-paste latency sampling; run with --ignored --nocapture"]
    #[allow(
        clippy::too_many_lines,
        reason = "one opt-in harness measures a bulk text commit and its synchronous publication"
    )]
    fn issue_261_bulk_paste_measurements() {
        use std::time::{Duration, Instant};

        fn report(label: &str, samples: &mut [Duration]) {
            samples.sort_unstable();
            let median = samples[samples.len() / 2].as_nanos();
            let p95_index = (samples.len() * 95).div_ceil(100).saturating_sub(1);
            eprintln!(
                "issue261_measurement label={label} n={} median_ns={median} p95_ns={}",
                samples.len(),
                samples[p95_index].as_nanos()
            );
        }

        let mapping = NativeMapping::from_parts(PhysicalSize::new(800, 480), 1.0)
            .unwrap_or_else(|| unreachable!("measurement mapping is valid"));
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::tight(&environment, mapping.logical_size)
            .with_raster_scale(mapping.raster_scale);
        let fixture = "multiline responsiveness fixture — retained text layout\n";

        for (name, lines) in [
            ("40_lines", 40),
            ("400_lines", 400),
            ("4000_lines", 4000),
            ("16000_lines", 16000),
        ] {
            let mut commit_pump = Vec::with_capacity(3);
            let mut publication = Vec::with_capacity(3);
            let mut complete = Vec::with_capacity(3);
            let text = fixture.repeat(lines);
            let bytes = text.len();
            for _ in 0..3 {
                let mut runtime = AppRuntime::<DemoApp>::mount_with_config(
                    DemoState::default(),
                    RuntimeConfig::default()
                        .with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
                );
                runtime.pump(HOST_PUMP_BUDGET);
                let owner = runtime.index().nodes()[0].id().clone();
                runtime
                    .submit_command(
                        owner,
                        SemanticCommand::RequestFocus,
                        CommandOrigin::programmatic(),
                    )
                    .unwrap_or_else(|_| unreachable!("paste fixture accepts focus"));
                runtime.pump(HOST_PUMP_BUDGET);
                runtime
                    .publish_surface(&context)
                    .unwrap_or_else(|error| unreachable!("warm surface is valid: {error:?}"));

                let started = Instant::now();
                runtime
                    .submit_text(
                        CommittedTextEvent::new(text.clone(), None)
                            .unwrap_or_else(|_| unreachable!("paste fixture text is valid")),
                    )
                    .unwrap_or_else(|_| unreachable!("paste fixture is admitted"));
                runtime.pump(HOST_PUMP_BUDGET);
                let commit_elapsed = started.elapsed();
                commit_pump.push(commit_elapsed);

                let started = Instant::now();
                runtime
                    .publish_surface(&context)
                    .unwrap_or_else(|error| unreachable!("paste publication is valid: {error:?}"));
                let publication_elapsed = started.elapsed();
                publication.push(publication_elapsed);
                complete.push(commit_elapsed + publication_elapsed);
            }

            eprintln!("issue261_measurement label={name}.paste_bytes value={bytes}");
            report(&format!("{name}.paste_submit_pump"), &mut commit_pump);
            report(&format!("{name}.paste_publication"), &mut publication);
            report(&format!("{name}.paste_complete"), &mut complete);
        }
    }

    #[test]
    #[ignore = "opt-in issue 263 phase-separated large-document profile; run release with --ignored --nocapture"]
    #[allow(
        clippy::too_many_lines,
        reason = "one opt-in harness compares four publication states across the accepted large-document sizes"
    )]
    fn issue_263_large_document_publication_profile() {
        use std::time::Instant;

        const SAMPLE_COUNT: usize = 20;
        const CANTARELL: &[u8] =
            include_bytes!("../../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf");
        type Profile = runenui_runtime::SurfacePublicationTestProfile;
        type TimingField = (&'static str, fn(&Profile) -> u128);
        type CountField = (&'static str, fn(&Profile) -> usize);

        fn summarize(values: &mut [u128]) -> (u128, u128) {
            values.sort_unstable();
            let median = values[values.len() / 2];
            let p95_index = (values.len() * 95).div_ceil(100).saturating_sub(1);
            (median, values[p95_index])
        }

        fn report_samples(label: &str, samples: &mut [u128]) {
            let (median, p95) = summarize(samples);
            eprintln!(
                "issue263_profile label={label} n={} median_ns={median} p95_ns={p95}",
                samples.len()
            );
        }

        fn report_ns(label: &str, profiles: &[Profile], field: fn(&Profile) -> u128) {
            let mut values = profiles.iter().map(field).collect::<Vec<_>>();
            let (median, p95) = summarize(&mut values);
            eprintln!(
                "issue263_profile label={label} n={} median_ns={median} p95_ns={p95}",
                profiles.len()
            );
        }

        fn report_count(label: &str, profiles: &[Profile], field: fn(&Profile) -> usize) {
            let mut values = profiles.iter().map(field);
            let first = values
                .next()
                .unwrap_or_else(|| unreachable!("profile sample set is non-empty"));
            let (mut minimum, mut maximum) = (first, first);
            for value in values {
                minimum = minimum.min(value);
                maximum = maximum.max(value);
            }
            eprintln!(
                "issue263_profile_count label={label} n={} min={minimum} max={maximum}",
                profiles.len()
            );
        }

        fn report_profile(label: &str, total_ns: &mut [u128], profiles: &[Profile]) {
            let mut remaining_runtime = total_ns
                .iter()
                .zip(profiles)
                .map(|(total, profile)| {
                    total.saturating_sub(
                        profile
                            .surface_plan_ns
                            .saturating_add(profile.displayed_text_targets_ns)
                            .saturating_add(profile.semantic_candidate_ns)
                            .saturating_add(profile.semantic_plan_ns),
                    )
                })
                .collect::<Vec<_>>();
            let (median, p95) = summarize(total_ns);
            eprintln!(
                "issue263_profile label={label}.total_publication n={} median_ns={median} p95_ns={p95}",
                profiles.len()
            );
            let timings: [TimingField; 14] = [
                ("surface_plan", |p| p.surface_plan_ns),
                ("layout", |p| p.layout_ns),
                ("widget_measure_callback", |p| p.widget_measure_callback_ns),
                ("text_request_prepare", |p| p.text_request_prepare_ns),
                ("text_layout", |p| p.text_layout_ns),
                ("text_shape", |p| p.text.shape_ns),
                ("text_line_break_align", |p| p.text.line_break_align_ns),
                ("text_artifact_extract", |p| p.text.artifact_extract_ns),
                ("text_graphemes", |p| p.text.grapheme_ns),
                ("text_legal_offsets", |p| p.text.legal_offsets_ns),
                ("paint", |p| p.paint_ns),
                ("displayed_text_targets", |p| p.displayed_text_targets_ns),
                ("semantic_candidate", |p| p.semantic_candidate_ns),
                ("semantic_plan", |p| p.semantic_plan_ns),
            ];
            for (suffix, field) in timings {
                report_ns(&format!("{label}.{suffix}"), profiles, field);
            }
            let (remaining_median, remaining_p95) = summarize(&mut remaining_runtime);
            eprintln!(
                "issue263_profile label={label}.remaining_runtime n={} median_ns={remaining_median} p95_ns={remaining_p95}",
                profiles.len()
            );
            let counts: [CountField; 17] = [
                ("measure_calls", |p| p.measure_calls),
                ("reshaped", |p| p.reshaped),
                ("relinebroken", |p| p.relinebroken),
                ("reused", |p| p.reused),
                ("paint_text_run_items", |p| p.paint_text_run_items),
                ("text_shape_calls", |p| p.text.shape_calls),
                ("text_line_break_calls", |p| p.text.line_break_calls),
                ("text_artifact_extract_calls", |p| {
                    p.text.artifact_extract_calls
                }),
                ("text_caret_map_calls", |p| p.text.caret_map_calls),
                ("text_grapheme_compute_calls", |p| {
                    p.text.grapheme_compute_calls
                }),
                ("text_legal_offsets_calls", |p| p.text.legal_offsets_calls),
                ("text_artifact_lines", |p| p.text.artifact_lines),
                ("text_artifact_runs", |p| p.text.artifact_runs),
                ("text_artifact_glyphs", |p| p.text.artifact_glyphs),
                ("text_artifact_clusters", |p| p.text.artifact_clusters),
                ("text_grapheme_boundaries", |p| p.text.grapheme_boundaries),
                ("text_legal_offsets", |p| p.text.legal_offsets),
            ];
            for (suffix, field) in counts {
                report_count(&format!("{label}.{suffix}"), profiles, field);
            }
        }

        fn publish_profile(
            runtime: &mut AppRuntime<DemoApp>,
            context: &SurfaceBuildContext<'_>,
        ) -> (u128, Profile) {
            let started = Instant::now();
            runtime
                .publish_surface(context)
                .unwrap_or_else(|error| unreachable!("profile publication succeeds: {error:?}"));
            let total_ns = started.elapsed().as_nanos();
            (
                total_ns,
                runtime.__take_surface_publication_profile_for_test(),
            )
        }

        let mapping = NativeMapping::from_parts(PhysicalSize::new(800, 480), 1.0)
            .unwrap_or_else(|| unreachable!("profile mapping is valid"));
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::tight(&environment, mapping.logical_size)
            .with_raster_scale(mapping.raster_scale);
        let fixture = "multiline responsiveness fixture — retained text layout\n";
        let replacement_fixture = "replacement publication fixture — retained text layout state\n";

        for (name, lines) in [
            ("40_lines", 40),
            ("400_lines", 400),
            ("4000_lines", 4000),
            ("16000_lines", 16000),
        ] {
            let text = fixture.repeat(lines);
            let replacement = replacement_fixture.repeat(lines);
            let mut first_mutation = Vec::with_capacity(SAMPLE_COUNT);
            let mut first_total = Vec::with_capacity(SAMPLE_COUNT);
            let mut first_profiles = Vec::with_capacity(SAMPLE_COUNT);
            let mut unchanged_total = Vec::with_capacity(SAMPLE_COUNT);
            let mut unchanged_profiles = Vec::with_capacity(SAMPLE_COUNT);
            let mut localized_mutation = Vec::with_capacity(SAMPLE_COUNT);
            let mut localized_total = Vec::with_capacity(SAMPLE_COUNT);
            let mut localized_profiles = Vec::with_capacity(SAMPLE_COUNT);
            let mut replacement_mutation = Vec::with_capacity(SAMPLE_COUNT);
            let mut replacement_total = Vec::with_capacity(SAMPLE_COUNT);
            let mut replacement_profiles = Vec::with_capacity(SAMPLE_COUNT);

            for _ in 0..SAMPLE_COUNT {
                let mut runtime = AppRuntime::<DemoApp>::mount_with_config(
                    DemoState::default(),
                    RuntimeConfig::default()
                        .with_text_font_source_policy(FontSourcePolicy::BundledOnly),
                );
                runtime
                    .register_text_font_bytes(CANTARELL.to_vec())
                    .unwrap_or_else(|_| unreachable!("controlled profile font registers"));
                let profile_family = runenui_core::FontFamilyName::new("Cantarell")
                    .unwrap_or_else(|_| unreachable!("controlled profile family is valid"));
                runtime
                    .set_text_generic_family_mapping(
                        runenui_core::GenericFontFamily::SansSerif,
                        &[profile_family],
                    )
                    .unwrap_or_else(|_| {
                        unreachable!("controlled profile generic mapping is valid")
                    });
                runtime.pump(HOST_PUMP_BUDGET);
                let owner = runtime.index().nodes()[0].id().clone();
                runtime
                    .submit_command(
                        owner.clone(),
                        SemanticCommand::RequestFocus,
                        CommandOrigin::programmatic(),
                    )
                    .unwrap_or_else(|_| unreachable!("profile editor accepts focus"));
                runtime.pump(HOST_PUMP_BUDGET);
                let _ = publish_profile(&mut runtime, &context);

                runtime
                    .submit_command(
                        owner.clone(),
                        SemanticCommand::SelectAll,
                        CommandOrigin::programmatic(),
                    )
                    .unwrap_or_else(|_| unreachable!("profile editor accepts select-all"));
                runtime.pump(HOST_PUMP_BUDGET);
                let mutation_started = Instant::now();
                runtime
                    .submit_text(
                        CommittedTextEvent::new(text.clone(), None)
                            .unwrap_or_else(|_| unreachable!("profile replacement is valid")),
                    )
                    .unwrap_or_else(|_| unreachable!("profile replacement is admitted"));
                runtime.pump(HOST_PUMP_BUDGET);
                first_mutation.push(mutation_started.elapsed().as_nanos());
                let (total, profile) = publish_profile(&mut runtime, &context);
                first_total.push(total);
                first_profiles.push(profile);

                let (total, profile) = publish_profile(&mut runtime, &context);
                unchanged_total.push(total);
                unchanged_profiles.push(profile);

                let mutation_started = Instant::now();
                runtime
                    .submit_text(
                        CommittedTextEvent::new("x", None)
                            .unwrap_or_else(|_| unreachable!("localized profile edit is valid")),
                    )
                    .unwrap_or_else(|_| unreachable!("localized profile edit is admitted"));
                runtime.pump(HOST_PUMP_BUDGET);
                localized_mutation.push(mutation_started.elapsed().as_nanos());
                let (total, profile) = publish_profile(&mut runtime, &context);
                localized_total.push(total);
                localized_profiles.push(profile);

                runtime
                    .submit_command(
                        owner.clone(),
                        SemanticCommand::SelectAll,
                        CommandOrigin::programmatic(),
                    )
                    .unwrap_or_else(|_| unreachable!("replacement profile accepts select-all"));
                runtime.pump(HOST_PUMP_BUDGET);
                let mutation_started = Instant::now();
                runtime
                    .submit_text(
                        CommittedTextEvent::new(replacement.clone(), None)
                            .unwrap_or_else(|_| unreachable!("second replacement is valid")),
                    )
                    .unwrap_or_else(|_| unreachable!("second replacement is admitted"));
                runtime.pump(HOST_PUMP_BUDGET);
                replacement_mutation.push(mutation_started.elapsed().as_nanos());
                let (total, profile) = publish_profile(&mut runtime, &context);
                replacement_total.push(total);
                replacement_profiles.push(profile);
            }

            eprintln!(
                "issue263_profile_fixture label={name} lines={lines} first_bytes={} replacement_bytes={} samples={SAMPLE_COUNT}",
                text.len(),
                replacement.len()
            );
            report_samples(
                &format!("{name}.first_replacement.submit_pump"),
                &mut first_mutation,
            );
            report_profile(
                &format!("{name}.first_replacement"),
                &mut first_total,
                &first_profiles,
            );
            report_profile(
                &format!("{name}.unchanged_republish"),
                &mut unchanged_total,
                &unchanged_profiles,
            );
            report_samples(
                &format!("{name}.localized_edit.submit_pump"),
                &mut localized_mutation,
            );
            report_profile(
                &format!("{name}.localized_edit"),
                &mut localized_total,
                &localized_profiles,
            );
            report_samples(
                &format!("{name}.full_replacement.submit_pump"),
                &mut replacement_mutation,
            );
            report_profile(
                &format!("{name}.full_replacement"),
                &mut replacement_total,
                &replacement_profiles,
            );
        }
    }

    fn translated_point(
        displayed: &DisplayedFrame,
        mapping: NativeMapping,
    ) -> TranslatedPointerPoint {
        displayed
            .translate_cursor(Some(mapping), PhysicalPosition::new(240.0, 120.0))
            .unwrap_or_else(|_| unreachable!("fixture displayed mapping admits point ingress"))
    }

    fn submitted(outcome: MouseButtonOutcome) -> runenui_core::PointerEvent {
        match outcome {
            MouseButtonOutcome::Submit(event) => event,
            MouseButtonOutcome::Suppressed(diagnostic) => {
                unreachable!("fixture transition must be submitted: {diagnostic:?}")
            }
        }
    }

    #[test]
    fn native_mapping_rejects_zero_extent_and_invalid_scale() {
        assert!(NativeMapping::from_parts(PhysicalSize::new(0, 10), 1.0).is_none());
        assert!(NativeMapping::from_parts(PhysicalSize::new(10, 10), 0.0).is_none());
        assert!(NativeMapping::from_parts(PhysicalSize::new(10, 10), f64::NAN).is_none());
    }

    #[test]
    fn native_mapping_preserves_physical_extent_and_derives_neutral_values() {
        let mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("the fixture native mapping is valid"));
        assert_eq!(mapping.physical_size, PhysicalSize::new(1200, 800));
        assert!((mapping.logical_size.width() - 600.0).abs() < f32::EPSILON);
        assert!((mapping.logical_size.height() - 400.0).abs() < f32::EPSILON);
        assert!((mapping.raster_scale.get() - 2.0).abs() < f32::EPSILON);
        assert!((mapping.native_scale_factor - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn native_modifiers_translate_to_existing_neutral_bits() {
        assert_eq!(
            translate_modifiers(ModifiersState::empty()),
            KeyModifiers::NONE
        );
        let modifiers = translate_modifiers(
            ModifiersState::SHIFT
                | ModifiersState::CONTROL
                | ModifiersState::ALT
                | ModifiersState::SUPER,
        );
        assert!(modifiers.shift());
        assert!(modifiers.control());
        assert!(modifiers.alt());
        assert!(modifiers.meta());
    }

    #[test]
    fn displayed_frame_translates_cursor_with_exact_mapping_and_context() {
        let mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("the fixture native mapping is valid"));
        let displayed = displayed_frame(mapping);
        let expected_context = displayed.input_context.clone();

        let translated = displayed
            .translate_cursor(Some(mapping), PhysicalPosition::new(240.0, 120.0))
            .unwrap_or_else(|_| unreachable!("matching displayed mapping admits point ingress"));

        assert!((translated.position.x() - 120.0).abs() < f32::EPSILON);
        assert!((translated.position.y() - 60.0).abs() < f32::EPSILON);
        assert_eq!(translated.input_context, expected_context);
        assert_eq!(translated.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn displayed_frame_withholds_cursor_when_native_mapping_changed() {
        let displayed_mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("the displayed mapping is valid"));
        let current_mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 1.5)
            .unwrap_or_else(|| unreachable!("the current mapping is valid"));
        let displayed = displayed_frame(displayed_mapping);

        assert_eq!(
            displayed.translate_cursor(Some(current_mapping), PhysicalPosition::new(240.0, 120.0),),
            Err(PointIngressDiagnostic::DisplayedMappingMismatch)
        );
    }

    #[test]
    fn displayed_frame_withholds_cursor_without_current_mapping() {
        let mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("the displayed mapping is valid"));
        let displayed = displayed_frame(mapping);

        assert_eq!(
            displayed.translate_cursor(None, PhysicalPosition::new(240.0, 120.0)),
            Err(PointIngressDiagnostic::NativeMappingUnavailable)
        );
    }

    #[test]
    fn displayed_frame_rejects_invalid_native_cursor_coordinates() {
        let mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("the displayed mapping is valid"));
        let displayed = displayed_frame(mapping);

        assert_eq!(
            displayed.translate_cursor(Some(mapping), PhysicalPosition::new(f64::NAN, 120.0)),
            Err(PointIngressDiagnostic::NonFiniteNativePosition)
        );
        assert_eq!(
            displayed.translate_cursor(Some(mapping), PhysicalPosition::new(f64::MAX, 120.0)),
            Err(PointIngressDiagnostic::LogicalPositionOutOfRange)
        );
    }

    #[test]
    fn mouse_stream_identity_is_stable_until_release_then_advances() {
        let mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("the fixture mapping is valid"));
        let displayed = displayed_frame(mapping);
        let modifiers = KeyModifiers::SHIFT.with_control();
        let point = translated_point(&displayed, mapping).with_modifiers(modifiers);
        let device_id = input_device(11);
        let mut mouse = MouseInputState::default();

        let moved = mouse
            .cursor_moved(device_id, point.clone())
            .unwrap_or_else(|_| unreachable!("first hover allocates a mouse stream"));
        let first_id = moved.pointer_id();
        assert_eq!(first_id.get(), 1);
        assert_eq!(moved.device_id(), Some(device_id));
        assert_eq!(moved.modifiers(), modifiers);

        let down = submitted(
            mouse
                .button_input(
                    device_id,
                    ElementState::Pressed,
                    MouseButton::Left,
                    Some(point.clone()),
                )
                .unwrap_or_else(|_| unreachable!("primary press is representable")),
        );
        assert_eq!(down.pointer_id(), first_id);
        assert_eq!(down.device_id(), Some(device_id));
        assert_eq!(down.phase(), PointerPhase::Down);
        assert_eq!(down.changed_button(), Some(PointerButton::Primary));
        assert!(down.buttons().contains(PointerButton::Primary));
        assert_eq!(down.modifiers(), modifiers);

        let up = submitted(
            mouse
                .button_input(
                    device_id,
                    ElementState::Released,
                    MouseButton::Left,
                    Some(point.clone()),
                )
                .unwrap_or_else(|_| unreachable!("primary release is representable")),
        );
        assert_eq!(up.pointer_id(), first_id);
        assert_eq!(up.device_id(), Some(device_id));
        assert_eq!(up.phase(), PointerPhase::Up);
        assert!(up.buttons().is_empty());
        assert_eq!(up.modifiers(), modifiers);

        let moved_again = mouse
            .cursor_moved(device_id, point)
            .unwrap_or_else(|_| unreachable!("post-release hover allocates a fresh stream"));
        assert_eq!(moved_again.pointer_id().get(), 3);
        assert_eq!(moved_again.device_id(), Some(device_id));
        assert_eq!(moved_again.modifiers(), modifiers);
    }

    #[test]
    fn point_authority_invalidation_clears_native_position_and_cancels_stream() {
        let mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("the fixture mapping is valid"));
        let displayed = displayed_frame(mapping);
        let point = translated_point(&displayed, mapping);
        let accepted_position = point.position;
        let accepted_context = point.input_context.clone();
        let native_position = PhysicalPosition::new(240.0, 120.0);
        let device_id = input_device(12);
        let mut mouse = MouseInputState::default();
        mouse.note_cursor_position(native_position);

        let moved = mouse
            .cursor_moved(device_id, point.clone())
            .unwrap_or_else(|_| unreachable!("hover allocates a mouse stream"));
        let first_id = moved.pointer_id();
        let _down = submitted(
            mouse
                .button_input(
                    device_id,
                    ElementState::Pressed,
                    MouseButton::Left,
                    Some(point.clone()),
                )
                .unwrap_or_else(|_| unreachable!("primary press is representable")),
        );

        let cancel_modifiers = KeyModifiers::ALT;
        let cancel = mouse
            .invalidate_point_authority(cancel_modifiers)
            .unwrap_or_else(|| unreachable!("active stream produces cancellation"));
        assert_eq!(mouse.last_native_position(), None);
        assert_eq!(cancel.pointer_id(), first_id);
        assert_eq!(cancel.device_id(), Some(device_id));
        assert_eq!(cancel.phase(), PointerPhase::Cancel);
        assert_eq!(cancel.position(), accepted_position);
        assert_eq!(cancel.surface_context(), &accepted_context);
        assert_eq!(cancel.modifiers(), cancel_modifiers);
        assert!(cancel.buttons().contains(PointerButton::Primary));

        mouse.note_cursor_position(native_position);
        let moved_after_loss = mouse
            .cursor_moved(device_id, point.clone())
            .unwrap_or_else(|_| unreachable!("fresh native point allocates a new stream"));
        assert_eq!(moved_after_loss.pointer_id().get(), 3);
        assert!(moved_after_loss.buttons().is_empty());

        let release = mouse
            .button_input(
                device_id,
                ElementState::Released,
                MouseButton::Left,
                Some(point.clone()),
            )
            .unwrap_or_else(|_| unreachable!("suppressed release is representable"));
        assert!(matches!(
            release,
            MouseButtonOutcome::Suppressed(MouseIngressDiagnostic::SuppressedRelease(
                MouseButton::Left
            ))
        ));

        let down_after_release = submitted(
            mouse
                .button_input(
                    device_id,
                    ElementState::Pressed,
                    MouseButton::Left,
                    Some(point),
                )
                .unwrap_or_else(|_| unreachable!("a later real press is admitted")),
        );
        assert_eq!(down_after_release.pointer_id().get(), 3);
        assert_eq!(down_after_release.phase(), PointerPhase::Down);
    }

    #[test]
    fn device_change_cancels_old_stream_without_losing_point_authority() {
        let mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("the fixture mapping is valid"));
        let displayed = displayed_frame(mapping);
        let point = translated_point(&displayed, mapping);
        let native_position = PhysicalPosition::new(240.0, 120.0);
        let first_device = input_device(21);
        let second_device = input_device(22);
        let mut mouse = MouseInputState::default();
        mouse.note_cursor_position(native_position);

        let moved = mouse
            .cursor_moved(first_device, point.clone())
            .unwrap_or_else(|_| unreachable!("first device opens a stream"));
        let first_pointer = moved.pointer_id();
        let _down = submitted(
            mouse
                .button_input(
                    first_device,
                    ElementState::Pressed,
                    MouseButton::Left,
                    Some(point.clone()),
                )
                .unwrap_or_else(|_| unreachable!("first device press is representable")),
        );

        let mismatch = mouse.cursor_moved(second_device, point.clone());
        assert!(matches!(
            mismatch,
            Err(MouseIngressDiagnostic::DeviceMismatch { active, incoming })
                if active == first_device && incoming == second_device
        ));
        assert_eq!(mouse.active_device_id(), Some(first_device));

        let cancel_modifiers = KeyModifiers::META;
        let cancel = mouse
            .cancel_for_device_change(cancel_modifiers)
            .unwrap_or_else(|| unreachable!("device transition closes the old stream"));
        assert_eq!(cancel.pointer_id(), first_pointer);
        assert_eq!(cancel.device_id(), Some(first_device));
        assert_eq!(cancel.phase(), PointerPhase::Cancel);
        assert_eq!(cancel.modifiers(), cancel_modifiers);
        assert_eq!(mouse.last_native_position(), Some(native_position));

        let second_down = submitted(
            mouse
                .button_input(
                    second_device,
                    ElementState::Pressed,
                    MouseButton::Right,
                    Some(point.clone()),
                )
                .unwrap_or_else(|_| unreachable!("second device can use retained point authority")),
        );
        assert_eq!(second_down.pointer_id().get(), 3);
        assert_eq!(second_down.device_id(), Some(second_device));
        assert!(second_down.buttons().contains(PointerButton::Secondary));
        assert!(!second_down.buttons().contains(PointerButton::Primary));

        let old_release = mouse
            .button_input(
                first_device,
                ElementState::Released,
                MouseButton::Left,
                Some(point),
            )
            .unwrap_or_else(|_| unreachable!("old device release is suppressible"));
        assert!(matches!(
            old_release,
            MouseButtonOutcome::Suppressed(MouseIngressDiagnostic::SuppressedRelease(
                MouseButton::Left
            ))
        ));
        assert_eq!(mouse.active_device_id(), Some(second_device));
    }

    #[test]
    fn unavailable_press_and_matching_release_never_fabricate_runtime_transition() {
        let mapping = NativeMapping::from_parts(PhysicalSize::new(1200, 800), 2.0)
            .unwrap_or_else(|| unreachable!("the fixture mapping is valid"));
        let displayed = displayed_frame(mapping);
        let point = translated_point(&displayed, mapping);
        let device_id = input_device(31);
        let mut mouse = MouseInputState::default();

        let press = mouse
            .button_input(device_id, ElementState::Pressed, MouseButton::Left, None)
            .unwrap_or_else(|_| unreachable!("unavailable press is suppressible"));
        assert!(matches!(
            press,
            MouseButtonOutcome::Suppressed(MouseIngressDiagnostic::PointUnavailableAtPress(
                MouseButton::Left
            ))
        ));

        let release = mouse
            .button_input(
                device_id,
                ElementState::Released,
                MouseButton::Left,
                Some(point.clone()),
            )
            .unwrap_or_else(|_| unreachable!("matching release is suppressible"));
        assert!(matches!(
            release,
            MouseButtonOutcome::Suppressed(MouseIngressDiagnostic::SuppressedRelease(
                MouseButton::Left
            ))
        ));

        let moved = mouse
            .cursor_moved(device_id, point)
            .unwrap_or_else(|_| unreachable!("suppressed pair consumes no pointer identity"));
        assert_eq!(moved.pointer_id().get(), 1);
    }

    #[test]
    fn mouse_button_translation_preserves_supported_neutral_button_classes() {
        assert_eq!(
            translate_mouse_button(MouseButton::Left),
            PointerButton::Primary
        );
        assert_eq!(
            translate_mouse_button(MouseButton::Right),
            PointerButton::Secondary
        );
        assert_eq!(
            translate_mouse_button(MouseButton::Middle),
            PointerButton::Middle
        );
        assert_eq!(
            translate_mouse_button(MouseButton::Back),
            PointerButton::Other(4)
        );
        assert_eq!(
            translate_mouse_button(MouseButton::Forward),
            PointerButton::Other(5)
        );
        assert_eq!(
            translate_mouse_button(MouseButton::Other(9)),
            PointerButton::Other(9)
        );
    }
}
