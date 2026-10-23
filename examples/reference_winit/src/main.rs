use std::{
    env,
    fmt::{self, Write as _},
    future::Future,
    path::PathBuf,
    pin::pin,
    sync::{Arc, OnceLock},
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

mod framework_services;
mod pointer_mode;
mod proof_trace;
#[cfg(test)]
mod text_input;
mod wheel_input;

use framework_services::NativeFrameworkServices;
use pointer_mode::{
    Mode as HostPointerMode, Outcome as HostPointerOutcome, Scope as HostPointerScope,
    State as HostPointerState, WinitPointer,
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
    AppRuntime, PumpBudget, RedrawRequest, SubmitCompositionErrorKind, SubmitKeyboardErrorKind,
    SubmitTextErrorKind, SurfaceBuildContext, SurfacePublication,
};
use runenui_winit::accessibility::{AccessibilityEvent, SemanticAdapter};
use runenui_winit::device_identity::{DeviceIdentityError, DeviceIdentityMap};
use runenui_winit::keyboard_input::{
    KeyboardDownDecision, KeyboardIngressDiagnostic, KeyboardInputOutcome, KeyboardInputState,
    NativeKeyTransition,
};
use runenui_winit::mouse_input::{
    MouseButtonOutcome, MouseIngressDiagnostic, MouseInputState, TranslatedPointerPoint,
};
use runenui_winit::native_mapping::{NativeMapping, NativePointError};
use runenui_winit::text_input::{
    TextInputState, keyboard_committed_text_candidate, translate_preedit_range,
};
use runenui_winit::touch_input::{TouchIngressDiagnostic, TouchInputState};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        DeviceEvent, DeviceId, ElementState, Ime, MouseButton, Touch, TouchPhase, WindowEvent,
    },
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

const INITIAL_PHYSICAL_SIZE: PhysicalSize<u32> = PhysicalSize::new(800, 480);
const HOST_PUMP_BUDGET: PumpBudget = PumpBudget::new(64, 64, 64, 64);
const RAW_MOTION_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const NATIVE_RELEASE_RETRY_INTERVAL: Duration = Duration::from_secs(1);

fn native_release_retry_due(deadline: Option<Instant>, now: Instant) -> bool {
    deadline.is_some_and(|deadline| now >= deadline)
}

fn native_release_retry_deadline(
    state: &HostPointerState,
    deadline: Option<Instant>,
    now: Instant,
) -> Option<Instant> {
    if matches!(state, HostPointerState::ReleaseFailed) {
        deadline.or_else(|| now.checked_add(NATIVE_RELEASE_RETRY_INTERVAL))
    } else {
        None
    }
}

fn release_native_pointer(
    modes: &mut pointer_mode::PointerModes,
    platform: &mut impl pointer_mode::Platform,
    retry_at: &mut Option<Instant>,
    now: Instant,
) -> Result<(), pointer_mode::Failure> {
    if matches!(modes.state(), HostPointerState::ReleaseFailed) {
        *retry_at = native_release_retry_deadline(modes.state(), *retry_at, now);
        if !native_release_retry_due(*retry_at, now) {
            return Err(pointer_mode::Failure::ReleaseFailed);
        }
    }
    let result = modes.release(platform);
    *retry_at = native_release_retry_deadline(modes.state(), None, now);
    result
}

fn revoke_pointer_for_runtime_stop(
    status: runenui_runtime::RuntimeStatus,
    modes: &mut pointer_mode::PointerModes,
    platform: &mut impl pointer_mode::Platform,
) -> Result<(), pointer_mode::Failure> {
    if status != runenui_runtime::RuntimeStatus::Running {
        // Revoke eligibility even if acquisition rollback already failed:
        // a later successful retry must not restore a stopped runtime's UI.
        modes.revoke_focus();
        if !matches!(modes.state(), HostPointerState::ReleaseFailed) {
            modes.release(platform)?;
        }
    }
    Ok(())
}

fn earliest_host_deadline(a: Option<Instant>, b: Option<Instant>) -> Option<Instant> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(deadline), None) | (None, Some(deadline)) => Some(deadline),
        (None, None) => None,
    }
}

fn raw_motion_probe_expired(start: Instant, now: Instant) -> bool {
    now.checked_duration_since(start)
        .is_some_and(|elapsed| elapsed >= RAW_MOTION_PROBE_TIMEOUT)
}

const fn absolute_ui_ingress_allowed(
    presentation_suppressed: bool,
    modes: &pointer_mode::PointerModes,
) -> bool {
    !presentation_suppressed && modes.ui_pointer_allowed()
}

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
        let (byte_offset, affinity) = if self == Self::Default {
            (text.len(), TextAffinity::Upstream)
        } else {
            (0, TextAffinity::Downstream)
        };
        DemoState::with_selection(text, byte_offset, affinity)
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
        WidgetMeasure::Text(runenui_core::TextLeafMeasure::new(self.text.clone()))
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

    fn translate_cursor(
        &self,
        current_mapping: Option<NativeMapping>,
        physical_position: PhysicalPosition<f64>,
    ) -> Result<TranslatedPointerPoint, PointIngressDiagnostic> {
        let position = self
            .mapping
            .translate_cursor(current_mapping, physical_position)
            .map_err(PointIngressDiagnostic::from)?;
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

impl From<NativePointError> for PointIngressDiagnostic {
    fn from(error: NativePointError) -> Self {
        match error {
            NativePointError::MappingUnavailable => Self::NativeMappingUnavailable,
            NativePointError::DisplayedMappingMismatch => Self::DisplayedMappingMismatch,
            NativePointError::NonFiniteNativePosition => Self::NonFiniteNativePosition,
            NativePointError::LogicalPositionOutOfRange => Self::LogicalPositionOutOfRange,
        }
    }
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
    window_epoch: u64,
    raw_motion_samples: u64,
    pending_raw_motion_since: Option<Instant>,
    native_release_retry_at: Option<Instant>,
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
            window_epoch: 0,
            raw_motion_samples: 0,
            pending_raw_motion_since: None,
            native_release_retry_at: None,
        };
        host.drain_runtime_trace();
        host
    }

    fn drain_runtime_trace(&self) {
        proof_trace::drain(self.trace_sink.as_ref());
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, detail: &str) {
        eprintln!("reference_winit fatal: {detail}");
        self.release_host_pointer("fatal error");
        self.shutdown_runtime_checked();
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
        self.window_epoch = self
            .window_epoch
            .checked_add(1)
            .ok_or_else(|| String::from("native host window epoch exhausted"))?;
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
                mapping.physical_size().width,
                mapping.physical_size().height,
                mapping.native_scale_factor(),
                mapping.logical_size().width(),
                mapping.logical_size().height(),
                mapping.raster_scale().get()
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
            extent.width() == mapping.physical_size().width
                && extent.height() == mapping.physical_size().height
        });
        if force || !configured_matches {
            renderer
                .configure_surface(
                    mapping.physical_size().width,
                    mapping.physical_size().height,
                )
                .map_err(|error| format!("native surface configuration failed: {error}"))?;
            proof!(
                "stage=surface_configured physical={}x{} force={force}",
                mapping.physical_size().width,
                mapping.physical_size().height
            );
        }
        Ok(())
    }

    fn renderer_addresses_mapping(&self, mapping: NativeMapping) -> bool {
        self.renderer
            .as_ref()
            .and_then(Renderer::configured_surface_extent)
            .is_some_and(|extent| {
                extent.width() == mapping.physical_size().width
                    && extent.height() == mapping.physical_size().height
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

    fn observe_runtime_pump(&mut self) -> bool {
        // Observation errors are typed, recoverable preflight failures, not
        // unreachable invariants. The runtime re-wakes an initial failure;
        // later framework-service work must not run as if it had settled.
        let batch = match self.runtime.pump(HOST_PUMP_BUDGET) {
            Ok(batch) => batch,
            Err(error) => {
                eprintln!("reference_winit input observation deferred: {error}");
                self.revoke_stopped_runtime_pointer();
                return false;
            }
        };
        self.revoke_stopped_runtime_pointer();
        if self.runtime.status() != runenui_runtime::RuntimeStatus::Running {
            return false;
        }
        if !batch.ordered_records().is_empty() {
            proof!(
                "stage=host_input_batch ordered_records={} final_revision={}",
                batch.ordered_records().len(),
                batch.final_ownership().revision().get()
            );
        }
        if let Some(reason) = batch.pause_reason() {
            proof!("stage=host_input_observation_paused reason={reason:?}");
            return false;
        }
        true
    }

    fn revoke_stopped_runtime_pointer(&mut self) {
        if self.runtime.status() == runenui_runtime::RuntimeStatus::Running {
            return;
        }
        self.pending_raw_motion_since = None;
        if let Some(window) = self.window.as_ref() {
            let result = revoke_pointer_for_runtime_stop(
                self.runtime.status(),
                self.framework_services.pointer_modes_mut(),
                &mut WinitPointer(window),
            );
            self.framework_services
                .set_native_window_focused(window, false);
            if let Err(error) = result {
                eprintln!("reference_winit stopped-runtime pointer release failed: {error:?}");
            }
        }
    }

    fn shutdown_runtime_checked(&mut self) {
        let batch = match self.runtime.shutdown() {
            Ok(batch) => batch,
            Err(error) => {
                // The host is closing its runtime lifetime regardless. No
                // unobserved native receipt may survive slot disposal.
                eprintln!("reference_winit shutdown observation unavailable: {error}");
                return;
            }
        };
        proof!(
            "stage=host_input_shutdown ordered_records={} final_revision={}",
            batch.ordered_records().len(),
            batch.final_ownership().revision().get()
        );
    }

    fn pump_runtime_once(&mut self) {
        if !self.observe_runtime_pump() {
            return;
        }
        self.retry_deferred_framework_service_completions();
        if !self.observe_runtime_pump() {
            return;
        }
        self.execute_pending_framework_services();
        if !self.observe_runtime_pump() {
            return;
        }
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

        let context = SurfaceBuildContext::tight(&self.style_environment, mapping.logical_size())
            .with_raster_scale(mapping.raster_scale());
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
            mapping.physical_size().width,
            mapping.physical_size().height,
            mapping.native_scale_factor()
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
        if !absolute_ui_ingress_allowed(
            self.presentation_suppressed,
            self.framework_services.pointer_modes(),
        ) {
            return;
        }
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

    /// Queue admission is not M13 processing/finality. Return the exact
    /// pre-queue disposition to the adapter only for the original Down.
    fn submit_keyboard_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: KeyboardEvent,
        stage: &str,
    ) -> Option<KeyboardDownDecision> {
        proof!("stage=keyboard_translated source={stage:?} event={event:?}");
        match self.runtime.submit_keyboard(event) {
            Ok(_) => Some(KeyboardDownDecision::Queued),
            Err(error)
                if error.kind() == SubmitKeyboardErrorKind::NoFocusedTarget
                    && error.event().phase() == runenui_core::KeyboardPhase::Down
                    && !error.event().is_repeat() =>
            {
                self.note_keyboard_ingress_diagnostic(
                    KeyboardIngressDiagnostic::NoFocusedRuntimeTarget,
                );
                Some(KeyboardDownDecision::RejectedBeforeQueue)
            }
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!("{stage} could not enter runtime input: {error}"),
                );
                None
            }
        }
    }

    fn handle_window_occlusion(&mut self, event_loop: &ActiveEventLoop, occluded: bool) {
        proof!("stage=window_occluded occluded={occluded}");
        self.presentation_suppressed = occluded;
        if occluded {
            self.release_host_pointer("window occluded");
            if let Some(window) = self.window.as_ref() {
                let result = self
                    .framework_services
                    .pointer_modes_mut()
                    .focus_changed(false, &mut WinitPointer(window));
                self.framework_services
                    .set_native_window_focused(window, false);
                proof!("stage=host_pointer_occluded_focus_revoked result={result:?}");
            }
            if !self.cancel_native_touch_contacts(event_loop, "window occluded")
                || !self.cancel_focus_sensitive_input(event_loop, "window occluded")
            {
                return;
            }
            self.text_input.set_window_focused(false);
            self.handle_native_point_authority_loss(event_loop, "window occluded");
        } else {
            if self
                .window
                .as_ref()
                .is_some_and(|window| window.has_focus())
            {
                self.handle_window_focus(event_loop, true);
            }
            self.request_pending_redraw();
        }
    }

    fn handle_window_focus(&mut self, event_loop: &ActiveEventLoop, focused: bool) {
        proof!("stage=window_focus focused={focused}");
        if focused {
            if self.presentation_suppressed
                || self.runtime.status() != runenui_runtime::RuntimeStatus::Running
            {
                return;
            }
            if let Some(window) = self.window.as_ref() {
                let result = self
                    .framework_services
                    .pointer_modes_mut()
                    .focus_changed(true, &mut WinitPointer(window));
                proof!("stage=host_pointer_focus_acquired result={result:?}");
            }
            self.pump_runtime_once();
            self.text_input.set_window_focused(true);
            if let Some(window) = self.window.as_ref() {
                self.framework_services
                    .set_native_window_focused(window, true);
            }
            return;
        }

        self.release_host_pointer("native window lost focus");
        if let Some(window) = self.window.as_ref() {
            let result = self
                .framework_services
                .pointer_modes_mut()
                .focus_changed(false, &mut WinitPointer(window));
            proof!("stage=host_pointer_focus_lost result={result:?}");
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

    fn current_host_pointer_scope(&self) -> Option<HostPointerScope> {
        if self.runtime.status() != runenui_runtime::RuntimeStatus::Running {
            return None;
        }
        let surface = self
            .displayed_frame
            .as_ref()?
            .input_context
            .surface_id()
            .clone();
        (self.window_epoch != 0 && self.window.is_some()).then_some(HostPointerScope {
            window_epoch: self.window_epoch,
            surface: Some(surface),
        })
    }

    /// Never carry a native mode across a different or unpublished UI surface.
    fn release_if_pointer_scope_changed(
        &mut self,
        candidate: Option<&HostPointerScope>,
        reason: &str,
    ) {
        if self
            .framework_services
            .pointer_modes()
            .scope_changed(candidate)
        {
            proof!("stage=host_pointer_scope_revoked reason={reason:?}");
            self.release_host_pointer(reason);
        }
    }

    fn release_host_pointer(&mut self, reason: &str) {
        self.pending_raw_motion_since = None;
        // Native unlock cannot depend on a running UI pump.
        let Some(window) = self.window.as_ref() else {
            // No native window remains on which a failed release can be retried.
            self.native_release_retry_at = None;
            return;
        };
        let previous_mode_state = self.framework_services.pointer_modes().state().clone();
        let returning_from_relative = matches!(
            previous_mode_state,
            HostPointerState::AwaitingMotion(_)
                | HostPointerState::Active(pointer_mode::Lease {
                    mode: HostPointerMode::LockedRelative,
                    ..
                })
        );
        let result = release_native_pointer(
            self.framework_services.pointer_modes_mut(),
            &mut WinitPointer(window),
            &mut self.native_release_retry_at,
            Instant::now(),
        );
        self.framework_services.refresh_native_input_method(window);
        proof!(
            "stage=host_pointer_release previous={previous_mode_state:?} reason={reason:?} result={result:?}"
        );
        if let Err(error) = result {
            eprintln!("reference_winit native pointer release failed ({reason}): {error:?}");
        }
        // Relative acquisition already cancelled all UI pointer streams.
        // Never reuse its old coordinates on return to absolute input.
        if returning_from_relative && self.framework_services.pointer_modes().ui_pointer_allowed() {
            let _ = self.mouse.invalidate_point_authority(self.modifiers);
        }
    }

    fn toggle_host_pointer(&mut self, event_loop: &ActiveEventLoop, desired: HostPointerMode) {
        if !matches!(
            self.framework_services.pointer_modes().state(),
            HostPointerState::Absolute
        ) {
            self.release_host_pointer("host pointer mode toggle to UI");
            if !self.invalidate_mouse_point_authority(event_loop, "host mode returned to UI") {
                return;
            }
            return;
        }
        if self.presentation_suppressed {
            proof!("stage=host_pointer_acquire_rejected reason=occluded_or_suspended");
            return;
        }
        let Some(scope) = self.current_host_pointer_scope() else {
            proof!("stage=host_pointer_acquire_rejected reason=no_displayed_surface");
            return;
        };
        // Cancel before acquisition: held UI buttons must not turn into camera actions.
        if !self.cancel_native_touch_contacts(event_loop, "switching to gameplay")
            || !self.invalidate_mouse_point_authority(event_loop, "switching to gameplay")
        {
            return;
        }
        // Retire native composition/keyboard state before gameplay consumes input.
        if desired == HostPointerMode::LockedRelative
            && !self.cancel_focus_sensitive_input(event_loop, "switching to gameplay")
        {
            return;
        }
        self.pump_runtime_once();
        if self.current_host_pointer_scope().as_ref() != Some(&scope) {
            proof!("stage=host_pointer_acquire_rejected reason=retired_runtime_or_surface");
            return;
        }
        let Some(window) = self.window.as_ref() else {
            return;
        };
        self.raw_motion_samples = 0;
        let result = self.framework_services.pointer_modes_mut().request(
            scope,
            desired,
            &mut WinitPointer(window),
        );
        self.framework_services.refresh_native_input_method(window);
        proof!("stage=host_pointer_acquire result={result:?}");
        match result {
            Ok(HostPointerOutcome::WaitingForMotion(_)) => {
                self.pending_raw_motion_since = Some(Instant::now());
                proof!("stage=host_pointer_waiting_for_native_raw_motion");
            }
            Ok(HostPointerOutcome::Realized(_) | HostPointerOutcome::Released) => {
                proof!("stage=host_pointer_realized");
            }
            Err(error) => {
                eprintln!("reference_winit native gameplay pointer mode unavailable: {error:?}");
            }
        }
    }

    /// One host-owned native release and shutdown transaction, independent of UI shutdown health.
    fn close_native_window(&mut self, event_loop: &ActiveEventLoop) {
        proof!("stage=window_exit");
        self.release_host_pointer("window destroy/close");
        if let Some(window) = self.window.as_ref() {
            let result = self
                .framework_services
                .pointer_modes_mut()
                .retire_window(&mut WinitPointer(window));
            proof!("stage=host_pointer_window_retired result={result:?}");
        }
        if !self.cancel_native_touch_contacts(event_loop, "native window destroyed") {
            return;
        }
        self.shutdown_runtime_checked();
        self.framework_services
            .reset_native_window_ime(self.window.as_deref());
        self.framework_services.shutdown();
        self.deferred_framework_service_completions.clear();
        self.drain_runtime_trace();
        event_loop.exit();
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
        let events = match self
            .keyboard
            .cancel_all(self.modifiers, self.text_input.keyboard_composition_state())
        {
            Ok(events) => events,
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!("unresolved native key admission at {reason}: {error:?}"),
                );
                return false;
            }
        };
        if events.is_empty() {
            return true;
        }
        eprintln!("reference_winit keyboard lifetimes cancelled: {reason}");
        for event in events {
            if self
                .submit_keyboard_event(event_loop, event, "native keyboard cancellation")
                .is_none()
            {
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
        if !absolute_ui_ingress_allowed(
            self.presentation_suppressed,
            self.framework_services.pointer_modes(),
        ) {
            return;
        }
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
        if let Some(window) = self.window.as_ref() {
            self.framework_services
                .pointer_modes_mut()
                .confirm_fresh_absolute_point(&mut WinitPointer(window));
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
        if !absolute_ui_ingress_allowed(
            self.presentation_suppressed,
            self.framework_services.pointer_modes(),
        ) {
            return;
        }
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
        if !absolute_ui_ingress_allowed(
            self.presentation_suppressed,
            self.framework_services.pointer_modes(),
        ) {
            return;
        }
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

    #[allow(
        clippy::too_many_lines,
        reason = "one native callback must keep original admission, held-repeat, text, and failure ordering correlated"
    )]
    fn handle_keyboard_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_device_id: DeviceId,
        event: &winit::event::KeyEvent,
        is_synthetic: bool,
    ) {
        if self.presentation_suppressed {
            return;
        }
        if !is_synthetic && !event.repeat && event.state == ElementState::Pressed {
            if matches!(&event.logical_key, Key::Named(NamedKey::F7)) {
                self.toggle_host_pointer(event_loop, HostPointerMode::ConfinedAbsolute);
                return;
            }
            if matches!(&event.logical_key, Key::Named(NamedKey::F8)) {
                self.toggle_host_pointer(event_loop, HostPointerMode::LockedRelative);
                return;
            }
            if matches!(&event.logical_key, Key::Named(NamedKey::Escape))
                && !matches!(
                    self.framework_services.pointer_modes().state(),
                    HostPointerState::Absolute
                )
            {
                self.release_host_pointer("escape/menu");
                // Confined absolute mode can still own a pressed/captured UI
                // stream. Route its cancellation rather than retaining stale
                // coordinates or a pressed owner across the return to UI.
                if !self.invalidate_mouse_point_authority(event_loop, "escape/menu") {
                    return;
                }
                return;
            }
        }
        if !self.framework_services.pointer_modes().ui_pointer_allowed() {
            return;
        }
        let Some(device_id) = self.resolve_native_device_id(event_loop, native_device_id) else {
            return;
        };
        // No new UI work is required for a repeat when the native reference
        // host has no focused target. Preserve the adapter's original key
        // lifetime; a later Up/device loss still owns its terminal path.
        // Original nonrepeat Down MUST enter the adapter and canonical runtime
        // so a genuine NoFocusedTarget rejection is explicitly acknowledged.
        if event.repeat
            && matches!(event.state, ElementState::Pressed)
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
            KeyboardInputOutcome::Submit {
                event: keyboard_event,
                down_admission,
            } => {
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
                let Some(decision) = self.submit_keyboard_event(
                    event_loop,
                    keyboard_event,
                    "native keyboard translation",
                ) else {
                    return;
                };
                if let Some(admission) = down_admission
                    && let Err(error) = self.keyboard.resolve_down_admission(admission, decision)
                {
                    self.fail(
                        event_loop,
                        &format!("original keyboard admission lifetime changed: {error:?}"),
                    );
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
            KeyboardInputOutcome::Failed(failure) => {
                self.fail(
                    event_loop,
                    &format!("native keyboard ingress failed closed: {failure:?}"),
                );
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
        if changed {
            self.release_host_pointer("native mapping changed");
        }
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
        let next = HostPointerScope {
            window_epoch: self.window_epoch,
            surface: Some(pending.publication.input_context().surface_id().clone()),
        };
        self.release_if_pointer_scope_changed(Some(&next), "displayed surface identity changed");
        self.displayed_frame = Some(DisplayedFrame::from_pending(pending));
        self.pending_frame = None;
        proof!(
            "stage=presented input_context={:?} physical={}x{} native_scale={}",
            pending.publication.input_context(),
            pending.mapping.physical_size().width,
            pending.mapping.physical_size().height,
            pending.mapping.native_scale_factor()
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
            pending.mapping.physical_size().width,
            pending.mapping.physical_size().height,
            pending.mapping.native_scale_factor()
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
                self.release_host_pointer("native surface lost");
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
        // Winit allows redundant suspend/resume transitions. A retained window
        // may already have native focus without emitting another Focused event.
        if self
            .window
            .as_ref()
            .is_some_and(|window| window.has_focus())
        {
            self.handle_window_focus(event_loop, true);
        }
        if !self.establish_initial_runtime_focus(event_loop) {
            return;
        }
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        proof!("stage=host_suspended");
        self.release_host_pointer("host suspended");
        if let Some(window) = self.window.as_ref() {
            let result = self
                .framework_services
                .pointer_modes_mut()
                .focus_changed(false, &mut WinitPointer(window));
            proof!("stage=host_pointer_suspended_focus_revoked result={result:?}");
            if result.is_ok() {
                self.native_release_retry_at = None;
            }
        }
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

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if matches!(event, DeviceEvent::Removed) {
            // This single-window probe does not own a seat/device manager.
            // Conservatively revoke on native source loss instead of guessing
            // whether the removed source can still drive this lease.
            self.release_host_pointer("native device removed");
            let _ = self.invalidate_mouse_point_authority(event_loop, "native device removed");
            return;
        }
        let DeviceEvent::MouseMotion { delta } = event else {
            return;
        };
        if self.presentation_suppressed
            || self.runtime.status() != runenui_runtime::RuntimeStatus::Running
        {
            return;
        }
        let current_scope = self.current_host_pointer_scope();
        self.release_if_pointer_scope_changed(
            current_scope.as_ref(),
            "raw-device event without current displayed surface",
        );
        let Some(scope) = current_scope else {
            return;
        };
        if self
            .pending_raw_motion_since
            .is_some_and(|start| raw_motion_probe_expired(start, Instant::now()))
        {
            self.release_host_pointer("raw device motion arrived after acquisition deadline");
            proof!("stage=host_pointer_raw_motion_timeout");
            return;
        }
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if !self.framework_services.pointer_modes_mut().observe_motion(
            &scope,
            delta,
            &mut WinitPointer(window),
        ) || !self
            .framework_services
            .pointer_modes()
            .gameplay_motion_allowed()
        {
            return;
        }
        // Device-native units stay in the host. Never synthesize RunenUI PointerEvent.
        self.raw_motion_samples = self.raw_motion_samples.saturating_add(1);
        if self.raw_motion_samples == 1 {
            self.pending_raw_motion_since = None;
            proof!("stage=host_gameplay_raw_motion_started device={device_id:?} scope={scope:?}");
            #[cfg(target_os = "windows")]
            {
                use winit::platform::windows::DeviceIdExtWindows;
                // Query only availability, never log a persistent device path.
                proof!(
                    "stage=host_raw_windows_device available={}",
                    device_id.persistent_identifier().is_some()
                );
            }
        }
        if self.raw_motion_samples <= 8 {
            proof!(
                "stage=host_gameplay_raw_motion_sample sample={} device={device_id:?} delta={delta:?}",
                self.raw_motion_samples
            );
        }
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
                self.close_native_window(event_loop);
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
                if self.framework_services.pointer_modes().ui_pointer_allowed() {
                    self.handle_native_drag_drop_event(event_loop, drag_event);
                }
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
                if absolute_ui_ingress_allowed(
                    self.presentation_suppressed,
                    self.framework_services.pointer_modes(),
                ) {
                    wheel_input::handle_mouse_wheel(self, event_loop, device_id, delta);
                }
            }
            WindowEvent::Touch(touch) => self.handle_native_touch(event_loop, touch),
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                self.handle_keyboard_input(event_loop, device_id, &event, is_synthetic);
            }
            WindowEvent::Ime(ime) => {
                if absolute_ui_ingress_allowed(
                    self.presentation_suppressed,
                    self.framework_services.pointer_modes(),
                ) {
                    self.handle_ime_state(event_loop, ime);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = translate_modifiers(modifiers.state());
                proof!("stage=modifiers_changed modifiers={:?}", self.modifiers);
            }
            WindowEvent::Focused(focused) => self.handle_window_focus(event_loop, focused),
            WindowEvent::Occluded(occluded) => {
                self.handle_window_occlusion(event_loop, occluded);
            }
            WindowEvent::RedrawRequested => {
                proof!("stage=redraw_event");
                self.render_pending(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.revoke_stopped_runtime_pointer();
        // Derive retry admission from controller state, including acquisition
        // rollback failure and focus/occlusion cleanup. An operation result
        // alone misses those paths; preserve deadlines to avoid starvation.
        self.native_release_retry_at = native_release_retry_deadline(
            self.framework_services.pointer_modes().state(),
            self.native_release_retry_at,
            now,
        );
        if self
            .pending_raw_motion_since
            .is_some_and(|start| raw_motion_probe_expired(start, now))
        {
            self.release_host_pointer("raw device motion unavailable within acquisition deadline");
            proof!("stage=host_pointer_raw_motion_timeout");
        }
        if native_release_retry_due(self.native_release_retry_at, now) {
            self.release_host_pointer("retry failed native cursor unlock");
            proof!("stage=host_pointer_release_retry");
        }
        let raw_deadline = self
            .pending_raw_motion_since
            .and_then(|start| start.checked_add(RAW_MOTION_PROBE_TIMEOUT));
        let next_deadline = earliest_host_deadline(raw_deadline, self.native_release_retry_at);
        event_loop
            .set_control_flow(next_deadline.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
        self.request_pending_redraw();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        proof!("stage=host_exiting");
        self.release_host_pointer("host exiting");
        self.shutdown_runtime_checked();
        self.framework_services
            .reset_native_window_ime(self.window.as_deref());
        self.framework_services.shutdown();
        self.deferred_framework_service_completions.clear();
        self.drain_runtime_trace();
    }
}

impl Drop for ReferenceHost {
    fn drop(&mut self) {
        if let Some(window) = self.window.as_ref()
            && let Err(error) = self
                .framework_services
                .pointer_modes_mut()
                .retire_window(&mut WinitPointer(window))
        {
            eprintln!("reference_winit drop could not confirm native pointer release: {error:?}");
        }
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
mod tests;
