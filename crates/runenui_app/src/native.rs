//! Native single-window application host over the public `RunenUI` runtime and renderer.
//! This is an optional outer composition owner, never a second UI runtime.

use crate::NativeAppError;

use std::{
    future::Future,
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
    time::Instant,
};

use runenui_core::{
    ElementId, InputDeviceId, KeyModifiers, KeyboardCompositionState, LogicalDelta, NoHostProtocol,
    PointerEvent, SemanticCommand, StyleEnvironment, SurfaceInputContext, UiApp,
};
use runenui_render_wgpu::{PublicationRenderError, Renderer, RendererOptions, ResourceProvider};
use runenui_runtime::{
    AppRuntime, MonotonicClock, MonotonicInstant, PumpBudget, RedrawRequest, RuntimeConfig,
    RuntimeStatus, SubmitKeyboardErrorKind, SurfaceBuildContext, SurfacePublication,
};
use runenui_winit::{
    accessibility::{AccessibilityEvent, SemanticAdapter},
    device_identity::{DeviceIdentityError, DeviceIdentityMap},
    keyboard_input::{
        KeyboardDownDecision, KeyboardInputOutcome, KeyboardInputState, NativeKeyTransition,
    },
    mouse_input::{MouseButtonOutcome, MouseInputState, TranslatedPointerPoint},
    native_mapping::{NativeMapping, NativePointError},
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const INITIAL_PHYSICAL_SIZE: PhysicalSize<u32> = PhysicalSize::new(640, 420);
const NATIVE_LINE_SCROLL_LOGICAL_UNITS: f64 = 48.0;
const HOST_PUMP_BUDGET: PumpBudget = PumpBudget::new(64, 64, 64, 64);

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

#[allow(
    clippy::cast_possible_truncation,
    reason = "native wheel deltas are finite and f32-range-checked before conversion into RunenUI logical coordinates"
)]
fn normalize_wheel_delta(
    delta: MouseScrollDelta,
    displayed_scale_factor: f64,
) -> Option<LogicalDelta> {
    let (logical_x, logical_y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (
            f64::from(x) * NATIVE_LINE_SCROLL_LOGICAL_UNITS,
            f64::from(y) * NATIVE_LINE_SCROLL_LOGICAL_UNITS,
        ),
        MouseScrollDelta::PixelDelta(delta) => {
            if !displayed_scale_factor.is_finite() || displayed_scale_factor <= 0.0 {
                return None;
            }
            (
                delta.x / displayed_scale_factor,
                delta.y / displayed_scale_factor,
            )
        }
    };
    if !logical_x.is_finite()
        || !logical_y.is_finite()
        || logical_x < f64::from(f32::MIN)
        || logical_x > f64::from(f32::MAX)
        || logical_y < f64::from(f32::MIN)
        || logical_y > f64::from(f32::MAX)
    {
        return None;
    }
    LogicalDelta::new(logical_x as f32, logical_y as f32).ok()
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

impl DisplayedFrame {
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

struct NativeMonotonicClock {
    origin: Instant,
}

impl NativeMonotonicClock {
    fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl MonotonicClock for NativeMonotonicClock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::ZERO
            .checked_add(self.origin.elapsed())
            .unwrap_or_else(|_| {
                unreachable!("native host lifetime fits RunenUI monotonic nanosecond range")
            })
    }
}

struct NativeHost<A: UiApp<HostProtocol = NoHostProtocol>> {
    title: String,
    resources: Box<dyn ResourceProvider>,
    fatal: Option<String>,
    cleanup_error: Option<String>,
    initial_focus: Option<ElementId>,
    runtime: AppRuntime<A>,
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
    modifiers: KeyModifiers,
    mapping_publication_needed: bool,
    presentation_suppressed: bool,
}

impl<A: UiApp<HostProtocol = NoHostProtocol>> NativeHost<A> {
    fn new(
        proxy: EventLoopProxy<HostEvent>,
        title: String,
        model: A::State,
        style_environment: StyleEnvironment,
        initial_focus: Option<ElementId>,
        resources: Box<dyn ResourceProvider>,
        config: RuntimeConfig,
    ) -> Self {
        let mut runtime = AppRuntime::<A>::mount_with_config(model, config);
        runtime.set_monotonic_clock(NativeMonotonicClock::new());
        let wake_proxy = proxy.clone();
        runtime.set_wake_transport(move || {
            let _ = wake_proxy.send_event(HostEvent::Wake);
        });
        Self {
            title,
            resources,
            fatal: None,
            cleanup_error: None,
            initial_focus,
            runtime,
            style_environment,
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
            modifiers: KeyModifiers::NONE,
            mapping_publication_needed: false,
            presentation_suppressed: false,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, detail: &str) {
        if self.fatal.is_none() {
            self.fatal = Some(detail.to_owned());
        }
        self.shutdown_runtime_checked();
        event_loop.exit();
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
            .with_title(&self.title)
            .with_inner_size(INITIAL_PHYSICAL_SIZE)
            .with_visible(false);
        let window = event_loop
            .create_window(attributes)
            .map_err(|error| format!("native window creation failed: {error}"))?;
        let accessibility = accesskit_winit::Adapter::with_mixed_handlers(
            event_loop,
            &window,
            self.semantic_adapter.activation_handler(),
            self.event_loop_proxy.clone(),
        );
        self.window = Some(Arc::new(window));
        self.accessibility = Some(accessibility);
        Ok(())
    }

    fn ensure_renderer(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.renderer.is_some() {
            return Ok(());
        }
        let window = self
            .window
            .clone()
            .ok_or_else(|| "renderer creation requires the native window".to_owned())?;
        let display = event_loop.owned_display_handle();
        self.renderer = Some(
            block_on(Renderer::request_with_surface_target(
                RendererOptions::new(),
                Box::new(display),
                window,
            ))
            .map_err(|error| format!("native renderer creation failed: {error}"))?,
        );
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

    fn pump_runtime_once(&mut self) {
        let batch = match self.runtime.pump(HOST_PUMP_BUDGET) {
            Ok(batch) => batch,
            Err(error) => {
                // The canonical runtime re-requests a wake after a failed
                // initial observation; never treat a pending receipt as settled.
                eprintln!("RunenUI native input observation deferred: {error}");
                return;
            }
        };
        if !batch.ordered_records().is_empty() {
            eprintln!(
                "RunenUI native input batch: {} ordered records, revision {}",
                batch.ordered_records().len(),
                batch.final_ownership().revision().get()
            );
        }
        if let Some(reason) = batch.pause_reason() {
            eprintln!("RunenUI native input observation paused: {reason:?}");
        }
    }

    fn shutdown_runtime_checked(&mut self) {
        let batch = match self.runtime.shutdown() {
            Ok(batch) => batch,
            Err(error) => {
                // The shutdown transition was not observed; report the error, not success.
                self.cleanup_error.get_or_insert_with(|| error.to_string());
                return;
            }
        };
        if !batch.ordered_records().is_empty() {
            eprintln!(
                "RunenUI native close: {} ordered records, revision {}",
                batch.ordered_records().len(),
                batch.final_ownership().revision().get()
            );
        }
    }

    fn collect_redraw_request(&mut self) {
        if self.pending_redraw.is_none() {
            self.pending_redraw = self.runtime.take_redraw_request();
        }
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
            .map_err(|error| format!("RunenUI surface publication failed: {error:?}"))?;
        let accessibility_update = self
            .semantic_adapter
            .update(publication.semantic_publication());
        for diagnostic in &accessibility_update.diagnostics {
            eprintln!("RunenUI accessibility diagnostic: {diagnostic:?}");
        }
        if let Some(accessibility) = self.accessibility.as_mut() {
            let tree_update = accessibility_update.tree_update;
            accessibility.update_if_active(|| tree_update);
        }
        if let Some(request) = self.pending_redraw.take() {
            self.runtime
                .acknowledge_redraw(&request)
                .map_err(|error| format!("RunenUI redraw acknowledgement failed: {error:?}"))?;
        }
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
        if let RuntimeStatus::Terminal(reason) = self.runtime.status() {
            self.fail(event_loop, &format!("runtime terminal: {reason:?}"));
            return;
        }
        if self
            .window
            .as_ref()
            .is_some_and(|window| window.has_focus())
            && let Some(target) = self.initial_focus.take()
        {
            if self.runtime.focus().focused_node().is_none() {
                if let Err(error) = self
                    .runtime
                    .submit_automation_command(target, SemanticCommand::RequestFocus)
                {
                    self.fail(
                        event_loop,
                        &format!("initial focus target rejected: {error:?}"),
                    );
                    return;
                }
                self.pump_runtime_once();
                if let RuntimeStatus::Terminal(reason) = self.runtime.status() {
                    self.fail(event_loop, &format!("initial focus terminal: {reason:?}"));
                    return;
                }
            }
        }
        self.collect_redraw_request();
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

    fn submit_pointer_event(&mut self, event_loop: &ActiveEventLoop, event: PointerEvent) -> bool {
        self.pump_runtime_once();
        if let Err(error) = self.runtime.submit_pointer(event) {
            self.fail(event_loop, &format!("native pointer input failed: {error}"));
            return false;
        }
        self.pump_runtime_once();
        true
    }

    fn submit_keyboard_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: runenui_core::KeyboardEvent,
    ) -> Option<KeyboardDownDecision> {
        match self.runtime.submit_keyboard(event) {
            Ok(_) => Some(KeyboardDownDecision::Queued),
            Err(error)
                if error.kind() == SubmitKeyboardErrorKind::NoFocusedTarget
                    && error.event().phase() == runenui_core::KeyboardPhase::Down
                    && !error.event().is_repeat() =>
            {
                Some(KeyboardDownDecision::RejectedBeforeQueue)
            }
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!("native keyboard input failed: {error}"),
                );
                None
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
        eprintln!("RunenUI mouse stream cancelled: {reason}");
        self.submit_pointer_event(event_loop, event)
    }

    fn handle_native_point_authority_loss(
        &mut self,
        event_loop: &ActiveEventLoop,
        reason: &str,
    ) -> bool {
        if !self.invalidate_mouse_point_authority(event_loop, reason) {
            return false;
        }
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
        true
    }

    fn cancel_mouse_for_device_change(
        &mut self,
        event_loop: &ActiveEventLoop,
        reason: &str,
    ) -> bool {
        let Some(event) = self.mouse.cancel_for_device_change(self.modifiers) else {
            return true;
        };
        eprintln!("RunenUI mouse stream cancelled: {reason}");
        self.submit_pointer_event(event_loop, event)
    }

    fn cancel_keyboard_authority(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let events = match self
            .keyboard
            .cancel_all(self.modifiers, KeyboardCompositionState::Inactive)
        {
            Ok(events) => events,
            Err(error) => {
                self.fail(
                    event_loop,
                    &format!("unresolved native key admission at source cancellation: {error:?}"),
                );
                return false;
            }
        };
        for event in events {
            if self.submit_keyboard_event(event_loop, event).is_none() {
                return false;
            }
        }
        self.pump_runtime_once();
        true
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
        let Ok(translated) = translated else {
            return;
        };
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
        if !self.submit_pointer_event(event_loop, event) {
            return;
        }
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn handle_mouse_wheel(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_device_id: DeviceId,
        delta: MouseScrollDelta,
    ) {
        let zero = match &delta {
            MouseScrollDelta::LineDelta(x, y) => *x == 0.0 && *y == 0.0,
            MouseScrollDelta::PixelDelta(delta) => delta.x == 0.0 && delta.y == 0.0,
        };
        if zero {
            return;
        }
        let Some(device_id) = self.resolve_native_device_id(event_loop, native_device_id) else {
            return;
        };
        let Ok(translated) = self.translate_latest_cursor() else {
            let _ = self.handle_native_point_authority_loss(
                event_loop,
                "native wheel arrived without matching displayed point authority",
            );
            return;
        };
        let Some(displayed_mapping) = self.displayed_frame.as_ref().map(|frame| frame.mapping)
        else {
            return;
        };
        let Some(scroll_delta) =
            normalize_wheel_delta(delta, displayed_mapping.native_scale_factor())
        else {
            eprintln!("RunenUI native wheel input withheld: unrepresentable delta");
            return;
        };
        if self
            .mouse
            .active_device_id()
            .is_some_and(|active| active != device_id)
            && !self.cancel_mouse_for_device_change(event_loop, "native mouse wheel device changed")
        {
            return;
        }
        let event = match self.mouse.wheel(device_id, translated, scroll_delta) {
            Ok(event) => event,
            Err(diagnostic) => {
                self.fail(
                    event_loop,
                    &format!("native mouse wheel could not be represented: {diagnostic:?}"),
                );
                return;
            }
        };
        if !self.submit_pointer_event(event_loop, event) {
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
        let translated = if let Ok(translated) = self.translate_latest_cursor() {
            Some(translated)
        } else {
            if !self.invalidate_mouse_point_authority(
                event_loop,
                "button transition arrived without matching displayed point authority",
            ) {
                return;
            }
            None
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
                if !self.submit_pointer_event(event_loop, event) {
                    return;
                }
            }
            MouseButtonOutcome::Suppressed(diagnostic) => {
                eprintln!("RunenUI mouse input withheld: {diagnostic:?}");
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
        let transition = NativeKeyTransition::from_event(event, is_synthetic);
        match self.keyboard.key_input(
            device_id,
            &transition,
            self.modifiers,
            KeyboardCompositionState::Inactive,
        ) {
            KeyboardInputOutcome::Submit {
                event,
                down_admission,
            } => {
                let Some(decision) = self.submit_keyboard_event(event_loop, event) else {
                    return;
                };
                if let Some(admission) = down_admission
                    && let Err(error) = self.keyboard.resolve_down_admission(admission, decision)
                {
                    self.fail(
                        event_loop,
                        &format!("keyboard admission lifetime changed: {error:?}"),
                    );
                    return;
                }
            }
            KeyboardInputOutcome::Suppressed(diagnostic) => {
                eprintln!("RunenUI keyboard input withheld: {diagnostic:?}");
                return;
            }
            KeyboardInputOutcome::Failed(failure) => {
                self.fail(
                    event_loop,
                    &format!("RunenUI native keyboard ingress failed closed: {failure:?}"),
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
        let render_result = self
            .renderer
            .as_mut()
            .ok_or(PublicationRenderError::SurfaceUnavailable)
            .and_then(|renderer| {
                renderer.render_surface_publication(
                    pending.publication.paint_publication(),
                    self.resources.as_ref(),
                    || window.pre_present_notify(),
                )
            });
        match render_result {
            Ok(observation) => {
                if !observation.presented() {
                    self.fail(
                        event_loop,
                        "renderer reported success without native presentation",
                    );
                    return;
                }
                self.displayed_frame = Some(DisplayedFrame::from_pending(&pending));
                self.pending_frame = None;
                self.drive_runtime(event_loop);
                self.request_pending_redraw();
            }
            Err(
                PublicationRenderError::SurfaceTimeout | PublicationRenderError::SurfaceOccluded,
            ) => self.request_pending_redraw(),
            Err(
                PublicationRenderError::SurfaceOutdated
                | PublicationRenderError::SurfaceSuboptimal
                | PublicationRenderError::SurfaceNotConfigured,
            ) => {
                if let Err(error) = self.configure_renderer(true) {
                    self.fail(event_loop, &error);
                } else {
                    self.request_pending_redraw();
                }
            }
            Err(PublicationRenderError::SurfaceLost) => {
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
                    self.request_pending_redraw();
                }
            }
            Err(error) => self.fail(event_loop, &format!("native presentation failed: {error}")),
        }
    }

    fn handle_accessibility_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: AccessibilityEvent,
    ) {
        if let AccessibilityEvent::ActionRequested(request) = event {
            match self.semantic_adapter.action_request(&request) {
                Ok(semantic_request) => match self.runtime.submit_semantic_action(semantic_request)
                {
                    Ok(_) => self.pump_runtime_once(),
                    Err(error) => {
                        eprintln!("RunenUI accessibility action rejected by runtime: {error:?}");
                    }
                },
                Err(diagnostic) => {
                    eprintln!("RunenUI accessibility action withheld: {diagnostic:?}");
                }
            }
        }
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }
}

impl<A: UiApp<HostProtocol = NoHostProtocol>> ApplicationHandler<HostEvent> for NativeHost<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.prepare_native_target(event_loop) {
            self.fail(event_loop, &error);
            return;
        }
        self.presentation_suppressed = false;
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if !self.cancel_keyboard_authority(event_loop) {
            return;
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
            HostEvent::Wake => self.drive_runtime(event_loop),
            HostEvent::Accessibility(event) => {
                self.handle_accessibility_event(event_loop, event);
            }
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
        }
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                self.shutdown_runtime_checked();
                event_loop.exit();
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.handle_mapping_change(event_loop);
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => self.handle_cursor_moved(event_loop, device_id, position),
            WindowEvent::CursorLeft { .. } => {
                let _ = self
                    .handle_native_point_authority_loss(event_loop, "native cursor left window");
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => self.handle_mouse_input(event_loop, device_id, state, button),
            WindowEvent::MouseWheel {
                device_id, delta, ..
            } => self.handle_mouse_wheel(event_loop, device_id, delta),
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => self.handle_keyboard_input(event_loop, device_id, &event, is_synthetic),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = translate_modifiers(modifiers.state());
            }
            WindowEvent::Focused(false) => {
                if !self.cancel_keyboard_authority(event_loop) {
                    return;
                }
                if !self.handle_native_point_authority_loss(event_loop, "native window lost focus")
                {
                    return;
                }
                self.modifiers = KeyModifiers::NONE;
            }
            WindowEvent::Focused(true) => {
                self.drive_runtime(event_loop);
            }
            WindowEvent::Occluded(occluded) => {
                self.presentation_suppressed = occluded;
                if !occluded {
                    self.request_pending_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.render_pending(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.request_pending_redraw();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.shutdown_runtime_checked();
    }
}

pub fn run_native<A: UiApp<HostProtocol = NoHostProtocol>>(
    title: String,
    state: A::State,
    environment: StyleEnvironment,
    initial_focus: Option<ElementId>,
    resources: Box<dyn ResourceProvider>,
    config: RuntimeConfig,
) -> Result<(), NativeAppError> {
    let event_loop = EventLoop::<HostEvent>::with_user_event()
        .build()
        .map_err(|error| NativeAppError::EventLoop(error.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    let mut host = NativeHost::<A>::new(
        proxy,
        title,
        state,
        environment,
        initial_focus,
        resources,
        config,
    );
    if let RuntimeStatus::Terminal(reason) = host.runtime.status() {
        host.shutdown_runtime_checked();
        return NativeAppError::combine_exit(
            Some(NativeAppError::Fatal(format!(
                "initial runtime work terminated: {reason:?}"
            ))),
            host.cleanup_error.map(NativeAppError::Shutdown),
        );
    }
    let event_loop_result = event_loop.run_app(&mut host);
    // An event-loop error is not proof that the exiting callback ran.
    // Always attempt required observed runtime shutdown afterwards.
    host.shutdown_runtime_checked();
    let primary = host.fatal.map(NativeAppError::Fatal);
    let event_loop_error = event_loop_result
        .err()
        .map(|error| NativeAppError::EventLoop(error.to_string()));
    let primary = NativeAppError::combine_exit(primary, event_loop_error).err();
    NativeAppError::combine_exit(primary, host.cleanup_error.map(NativeAppError::Shutdown))
}
