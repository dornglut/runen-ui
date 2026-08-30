use std::{
    env, fmt,
    future::Future,
    pin::pin,
    sync::{Arc, OnceLock},
    task::{Context, Poll, Wake, Waker},
    thread,
};

mod accessibility;
mod device_identity;
mod keyboard_input;
mod mouse_input;
mod proof_trace;
mod text_input;
mod wheel_input;

use accessibility::{AccessibilityEvent, SemanticAdapter};
use device_identity::{DeviceIdentityError, DeviceIdentityMap};
use keyboard_input::{
    KeyboardIngressDiagnostic, KeyboardInputOutcome, KeyboardInputState, NativeKeyTransition,
};
use mouse_input::{
    MouseButtonOutcome, MouseIngressDiagnostic, MouseInputState, TranslatedPointerPoint,
};
use runenui_core::{
    Color, CommandOrigin, CommittedTextEvent, Element, InputDeviceId, IntoEffects, KeyModifiers,
    KeyboardEvent, LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, PointerEvent, SemanticAction, SemanticCommand,
    SemanticContribution, SemanticKey, SemanticNodeContribution, SemanticRole, SemanticText,
    StyleEnvironment, SurfaceInputContext, UiApp, View, Widget, WidgetActivation, WidgetMeasure,
    WidgetTextInput,
};
use runenui_render_wgpu::{
    PublicationRenderError, Renderer, RendererOptions, ResourcePayload, ResourceProvider,
    ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RasterScale, RedrawRequest, SubmitCompositionErrorKind,
    SubmitKeyboardErrorKind, SubmitTextErrorKind, SurfaceBuildContext, SurfacePublication,
};
use text_input::{TextInputState, keyboard_committed_text_candidate, translate_preedit_range};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, Ime, MouseButton, WindowEvent},
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

#[derive(Debug)]
struct DemoSurface;

impl Widget<()> for DemoSurface {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn text_input(&self, _state: &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(400_u16),
            height: LogicalLength::from(240_u16),
        }
    }

    fn paint(&self, _state: &Self::State, context: PaintContributionContext) -> PaintContribution {
        let origin = LogicalPoint::new(0.0, 0.0)
            .unwrap_or_else(|_| unreachable!("the literal demo origin is finite"));
        let rect = LogicalRect::new(origin, context.local_size());
        PaintContribution::single(PaintContributionItem::fill_rect(
            rect,
            Color::rgb(28, 32, 40),
        ))
    }

    fn semantics(
        &self,
        _state: &Self::State,
        _context: runenui_core::SemanticContributionContext,
    ) -> SemanticContribution {
        let status_key = SemanticKey::from_static("status")
            .unwrap_or_else(|_| unreachable!("the static semantic key is valid"));
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name("RunenUI accessibility action")
                .with_description("Activate this control through the native accessibility tree")
                .with_action(SemanticAction::Activate)
                .with_action(SemanticAction::RequestFocus)
                .with_action(SemanticAction::OpenMenu)
                .with_action(SemanticAction::OpenContextMenu)
                .with_child(
                    SemanticNodeContribution::new(status_key, SemanticRole::Text)
                        .with_name("Native AccessKit path ready")
                        .with_text(SemanticText::plain("Native AccessKit path ready")),
                ),
        )
    }
}

struct DemoApp;

impl UiApp for DemoApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_state: &Self::State) -> impl View<Self::Action> {
        Element::new(DemoSurface).focusable(true)
    }

    fn update(
        _state: &mut Self::State,
        _action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
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
            "the reference host demo publishes only literal paint",
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
    keyboard: KeyboardInputState,
    text_input: TextInputState,
    applied_ime_allowed: Option<bool>,
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
    fn new(proxy: EventLoopProxy<HostEvent>) -> Self {
        let (runtime, trace_sink) = proof_trace::mount::<DemoApp>((), proof_enabled());
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
            keyboard: KeyboardInputState::default(),
            text_input: TextInputState::default(),
            applied_ime_allowed: None,
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
            .with_title("RunenUI M7 reference host")
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
        self.window = Some(Arc::new(window));
        self.accessibility = Some(accessibility);
        self.applied_ime_allowed = None;
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

    fn apply_native_ime_policy(&mut self, reset_native_ime: bool) {
        let Some(window) = self.window.as_ref() else {
            self.applied_ime_allowed = None;
            return;
        };
        if reset_native_ime {
            window.set_ime_allowed(false);
            self.applied_ime_allowed = Some(false);
            proof!("stage=ime_policy reset=true allowed=false");
        }
        let desired = self.text_input.ime_allowed();
        if self.applied_ime_allowed != Some(desired) {
            window.set_ime_allowed(desired);
            self.applied_ime_allowed = Some(desired);
            proof!("stage=ime_policy reset=false allowed={desired}");
        }
    }

    fn sync_runtime_text_input(&mut self) {
        let focused_owner = self.runtime.focus().focused_node().cloned();
        let capability = self.runtime.focused_text_input_capability();
        let sync = self.text_input.sync_runtime(focused_owner, capability);
        self.apply_native_ime_policy(sync.reset_native_ime());
    }

    fn pump_runtime_once(&mut self) {
        let _ = self.runtime.pump(HOST_PUMP_BUDGET);
        self.drain_runtime_trace();
        self.sync_runtime_text_input();
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
        let Some(mapping) = self.mapping else {
            return Ok(false);
        };
        if !self.renderer_addresses_mapping(mapping) {
            return Ok(false);
        }

        if self.pending_redraw.is_none() {
            self.pending_redraw = self.runtime.take_redraw_request();
            if self.pending_redraw.is_some() {
                proof!("stage=redraw_taken");
            }
        }
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

    fn drive_runtime(&mut self, event_loop: &ActiveEventLoop) {
        self.pump_runtime_once();
        if let Err(error) = self.publish_if_needed() {
            self.fail(event_loop, &error);
        }
    }

    fn submit_pointer_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: PointerEvent,
        stage: &str,
    ) -> bool {
        self.pump_runtime_once();
        proof!("stage=pointer_translated source={stage:?} event={event:?}");
        if let Err(error) = self.runtime.submit_pointer(event) {
            self.fail(
                event_loop,
                &format!("{stage} could not enter runtime input: {error}"),
            );
            return false;
        }
        self.pump_runtime_once();
        true
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
                eprintln!("reference_winit committed text accepted");
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
        self.apply_native_ime_policy(true);
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
                eprintln!("reference_winit composition preedit accepted");
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
                self.applied_ime_allowed = Some(true);
                self.apply_native_ime_policy(false);
            }
            Ime::Preedit(preedit, range) => self.handle_ime_preedit(event_loop, preedit, range),
            Ime::Commit(text) => self.handle_ime_commit(event_loop, &text),
            Ime::Disabled => {
                proof!("stage=native_ime state=disabled");
                self.applied_ime_allowed = Some(false);
                if !self.cancel_native_composition(event_loop, "native IME disabled") {
                    return;
                }
                self.apply_native_ime_policy(false);
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
        self.drive_runtime(event_loop);
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
        self.drive_runtime(event_loop);
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
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn handle_keyboard_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_device_id: DeviceId,
        event: &winit::event::KeyEvent,
        is_synthetic: bool,
    ) {
        self.pump_runtime_once();
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
        let committed_text = keyboard_committed_text_candidate(
            event.state,
            is_synthetic,
            self.text_input.accepts_committed_text(),
            composition,
            event.text.as_deref(),
        )
        .map(str::to_owned);
        let transition = NativeKeyTransition::from_event(event, is_synthetic);
        let outcome = self
            .keyboard
            .key_input(device_id, &transition, self.modifiers, composition);
        match outcome {
            KeyboardInputOutcome::Submit(event) => {
                self.last_keyboard_ingress_diagnostic = None;
                if !self.submit_keyboard_event(event_loop, event, "native keyboard translation") {
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
        self.apply_native_ime_policy(false);
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
            proof!("stage=window_shown");
        }
        Ok(())
    }

    fn handle_mapping_change(&mut self, event_loop: &ActiveEventLoop) {
        let changed = self.refresh_mapping();
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
                self.displayed_frame = Some(DisplayedFrame::from_pending(&pending));
                self.pending_frame = None;
                proof!(
                    "stage=presented input_context={:?} physical={}x{} native_scale={}",
                    pending.publication.input_context(),
                    pending.mapping.physical_size.width,
                    pending.mapping.physical_size.height,
                    pending.mapping.native_scale_factor
                );
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
        event_loop: &ActiveEventLoop,
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
        if let Err(error) = self.publish_if_needed() {
            self.fail(event_loop, &error);
        }
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
        if !self.cancel_focus_sensitive_input(event_loop, "native host suspended") {
            return;
        }
        self.text_input.set_window_focused(false);
        self.apply_native_ime_policy(false);
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
                let _ = self.runtime.shutdown();
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
            WindowEvent::Focused(false) => {
                proof!("stage=window_focus focused=false");
                if !self.cancel_focus_sensitive_input(event_loop, "native window lost focus") {
                    return;
                }
                self.text_input.set_window_focused(false);
                self.apply_native_ime_policy(false);
                self.handle_native_point_authority_loss(event_loop, "native window lost focus");
                self.modifiers = KeyModifiers::NONE;
            }
            WindowEvent::Focused(true) => {
                proof!("stage=window_focus focused=true");
                self.pump_runtime_once();
                self.text_input.set_window_focused(true);
                self.apply_native_ime_policy(false);
            }
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
        self.drain_runtime_trace();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<HostEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let mut host = ReferenceHost::new(proxy);
    event_loop.run_app(&mut host)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        AppRuntime, DemoApp, DisplayedFrame, HOST_PUMP_BUDGET, NativeMapping,
        PointIngressDiagnostic, StyleEnvironment, SurfaceBuildContext,
        mouse_input::{
            MouseButtonOutcome, MouseIngressDiagnostic, MouseInputState, TranslatedPointerPoint,
            translate_mouse_button,
        },
        translate_modifiers,
    };
    use runenui_core::{InputDeviceId, KeyModifiers, PointerButton, PointerPhase};
    use winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{ElementState, MouseButton},
        keyboard::ModifiersState,
    };

    fn input_device(value: u64) -> InputDeviceId {
        InputDeviceId::new(value)
            .unwrap_or_else(|| unreachable!("fixture input device identity is non-zero"))
    }

    fn displayed_frame(mapping: NativeMapping) -> DisplayedFrame {
        let mut runtime = AppRuntime::<DemoApp>::mount(());
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
        assert_eq!(moved_again.pointer_id().get(), 2);
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
        assert_eq!(moved_after_loss.pointer_id().get(), 2);
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
        assert_eq!(down_after_release.pointer_id().get(), 2);
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
        assert_eq!(second_down.pointer_id().get(), 2);
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
