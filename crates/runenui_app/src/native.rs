//! Native single-window application host over the public `RunenUI` runtime and renderer.
//! This is an optional outer composition owner, never a second UI runtime.

use crate::NativeAppError;

use std::{
    future::Future,
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

#[cfg(all(debug_assertions, feature = "internal-test-seams"))]
use std::collections::VecDeque;

use runenui_core::{
    ElementId, FrameworkServiceFailure, FrameworkServiceRequest, FrameworkServiceResponse,
    InputDeviceId, KeyModifiers, KeyboardCompositionState, LogicalDelta, NoHostProtocol,
    PointerEvent, SemanticCommand, StyleEnvironment, SurfaceInputContext, UiApp,
};
use runenui_render_wgpu::{PublicationRenderError, Renderer, RendererOptions, ResourceProvider};
use runenui_runtime::{
    AppRuntime, FrameworkServiceResponseError, MonotonicClock, MonotonicInstant, PumpBudget,
    RedrawRequest, RuntimeConfig, RuntimeStatus, SubmitKeyboardErrorKind, SurfaceBuildContext,
    SurfacePublication,
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

/// A native capability not implemented by this first desktop host receives a
/// typed unavailable response rather than hanging forever on an accepted work
/// token. No OS side effect may occur before that response is admitted.
const fn unavailable_framework_response(
    request: &FrameworkServiceRequest,
) -> Option<FrameworkServiceResponse> {
    match request {
        FrameworkServiceRequest::ClipboardReadText { .. } => Some(
            FrameworkServiceResponse::ClipboardReadText(Err(FrameworkServiceFailure::Unavailable)),
        ),
        FrameworkServiceRequest::ClipboardWriteText { .. } => Some(
            FrameworkServiceResponse::ClipboardWriteText(Err(FrameworkServiceFailure::Unavailable)),
        ),
        FrameworkServiceRequest::InputMethod { .. } => Some(FrameworkServiceResponse::InputMethod(
            Err(FrameworkServiceFailure::Unavailable),
        )),
        FrameworkServiceRequest::Cursor { .. } => Some(FrameworkServiceResponse::Cursor(Err(
            FrameworkServiceFailure::Unavailable,
        ))),
        FrameworkServiceRequest::DragDrop { .. } => Some(FrameworkServiceResponse::DragDrop(Err(
            FrameworkServiceFailure::Unavailable,
        ))),
        _ => None,
    }
}

const INITIAL_PHYSICAL_SIZE: PhysicalSize<u32> = PhysicalSize::new(640, 420);
const NATIVE_LINE_SCROLL_LOGICAL_UNITS: f64 = 48.0;

#[cfg(all(debug_assertions, feature = "internal-test-seams"))]
fn parse_native_render_faults(spec: &str) -> VecDeque<PublicationRenderError> {
    spec.split(',')
        .filter(|token| !token.is_empty())
        .map(|token| match token {
            "timeout" => PublicationRenderError::SurfaceTimeout,
            "occluded" => PublicationRenderError::SurfaceOccluded,
            "outdated" => PublicationRenderError::SurfaceOutdated,
            "suboptimal" => PublicationRenderError::SurfaceSuboptimal,
            "not-configured" => PublicationRenderError::SurfaceNotConfigured,
            "lost" => PublicationRenderError::SurfaceLost,
            _ => panic!("unknown native render test fault: {token}"),
        })
        .collect()
}

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

// Rendering retry belongs to the native presentation owner. It must never
// republish a runtime frame or advance displayed input mapping.
#[derive(Default)]
struct PresentationRetry {
    consecutive_failures: u8,
    next_attempt: Option<Instant>,
}

impl PresentationRetry {
    fn defer(&mut self, now: Instant) {
        // Bounded exponential backoff: 16, 32, 64, 128, then 256 ms.
        // Occluded surfaces remain retryable even when the OS does not emit a
        // matching WindowEvent::Occluded notification.
        let shift = u32::from(self.consecutive_failures.min(4));
        let delay = Duration::from_millis(16u64 << shift);
        self.next_attempt = now.checked_add(delay);
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
    }

    fn is_due(&self, now: Instant) -> bool {
        self.next_attempt.is_none_or(|deadline| now >= deadline)
    }

    fn future_deadline(&self, now: Instant) -> Option<Instant> {
        self.next_attempt.filter(|deadline| *deadline > now)
    }

    const fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.next_attempt = None;
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

#[derive(Clone, Copy)]
struct NativeMonotonicClock {
    origin: Instant,
}

impl NativeMonotonicClock {
    fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }

    fn host_deadline(self, deadline: MonotonicInstant) -> Option<Instant> {
        self.origin
            .checked_add(Duration::from_nanos(deadline.as_nanos()))
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
    exit_requested: bool,
    initial_focus: Option<ElementId>,
    native_clock: NativeMonotonicClock,
    next_deadline: Option<MonotonicInstant>,
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
    presentation_retry: PresentationRetry,
    #[cfg(all(debug_assertions, feature = "internal-test-seams"))]
    test_render_faults: VecDeque<PublicationRenderError>,
    #[cfg(all(debug_assertions, feature = "internal-test-seams"))]
    test_first_pending: Option<SurfacePublication>,
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
        let wake_proxy = proxy.clone();
        let native_clock = NativeMonotonicClock::new();
        let runtime =
            AppRuntime::<A>::mount_with_host_adapters(model, config, native_clock, move || {
                let _ = wake_proxy.send_event(HostEvent::Wake);
            });
        Self {
            title,
            resources,
            fatal: None,
            cleanup_error: None,
            exit_requested: false,
            initial_focus,
            native_clock,
            next_deadline: None,
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
            presentation_retry: PresentationRetry::default(),
            #[cfg(all(debug_assertions, feature = "internal-test-seams"))]
            test_render_faults: parse_native_render_faults(
                &std::env::var("RUNENUI_NATIVE_TEST_RENDER_FAULTS").unwrap_or_default(),
            ),
            #[cfg(all(debug_assertions, feature = "internal-test-seams"))]
            test_first_pending: None,
            device_identities: DeviceIdentityMap::default(),
            mouse: MouseInputState::default(),
            keyboard: KeyboardInputState::default(),
            modifiers: KeyModifiers::NONE,
            mapping_publication_needed: false,
            presentation_suppressed: false,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, detail: &str) {
        self.exit_requested = true;
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
        self.presentation_retry.reset();
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
        self.next_deadline = batch.report().next_deadline();
        // Motion/deadline-driven publications can become dirty without a
        // freshly consumed explicit redraw token. This is a renderer-neutral
        // readiness fact, not permission to synthesize extra frame updates.
        if batch.report().publication_dirty() {
            self.mapping_publication_needed = true;
        }
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

    fn reject_unsupported_framework_services(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Option<bool> {
        let requests = self
            .runtime
            .pending_framework_services()
            .into_iter()
            .map(|service| (service.token(), service.request().clone()))
            .collect::<Vec<_>>();
        let mut queued_response = false;
        for (token, request) in requests {
            let Some(response) = unavailable_framework_response(&request) else {
                self.fail(
                    event_loop,
                    "unknown framework-service request cannot be fulfilled",
                );
                return None;
            };
            match self.runtime.complete_framework_service(&token, response) {
                Ok(_) => queued_response = true,
                // The original work request stays live; retry after the next
                // bounded pump frees queue capacity rather than losing it.
                Err(FrameworkServiceResponseError::Full(_)) => break,
                Err(FrameworkServiceResponseError::Stale(_)) => {}
                Err(_) => {
                    self.fail(
                        event_loop,
                        "framework-service completion failed before admission",
                    );
                    return None;
                }
            }
        }
        Some(queued_response)
    }

    fn drive_runtime(&mut self, event_loop: &ActiveEventLoop) {
        self.pump_runtime_once();
        if let RuntimeStatus::Terminal(reason) = self.runtime.status() {
            self.fail(event_loop, &format!("runtime terminal: {reason:?}"));
            return;
        }
        let Some(responded) = self.reject_unsupported_framework_services(event_loop) else {
            return;
        };
        if responded {
            self.pump_runtime_once();
            if let RuntimeStatus::Terminal(reason) = self.runtime.status() {
                self.fail(
                    event_loop,
                    &format!("framework response terminal: {reason:?}"),
                );
                return;
            }
        }
        // Consume this one-time intent on the first genuinely focused
        // native window, even if the app already chose a runtime focus target.
        // Do not replay an obsolete initial intent after focus is lost.
        let initial_focus_target = if self
            .window
            .as_ref()
            .is_some_and(|window| window.has_focus())
        {
            self.initial_focus.take()
        } else {
            None
        };
        if let Some(target) = initial_focus_target
            && self.runtime.focus().focused_node().is_none()
        {
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
        self.collect_redraw_request();
    }

    fn request_pending_redraw(&self) {
        if self.presentation_suppressed || !self.presentation_retry.is_due(Instant::now()) {
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
            self.presentation_retry.reset();
            if let Err(error) = self.publish_if_needed() {
                self.fail(event_loop, &error);
            }
            return;
        }
        if self.presentation_suppressed || !self.presentation_retry.is_due(Instant::now()) {
            return;
        }
        let Some(window) = self.window.as_ref() else {
            return;
        };
        #[cfg(all(debug_assertions, feature = "internal-test-seams"))]
        if self.test_first_pending.as_ref().is_some_and(|original| original != &pending.publication) {
            self.fail(event_loop, "test: immutable pending publication changed during presentation recovery");
            return;
        }
        let render_result = {
            #[cfg(all(debug_assertions, feature = "internal-test-seams"))]
            if let Some(error) = self.test_render_faults.pop_front() {
                if self.test_first_pending.is_none() {
                    self.test_first_pending = Some(pending.publication.clone());
                }
                eprintln!("RUNENUI_NATIVE_TEST_RENDER_FAULT={error:?}");
                Err(error)
            } else {
                self.renderer
                    .as_mut()
                    .ok_or(PublicationRenderError::SurfaceUnavailable)
                    .and_then(|renderer| {
                        renderer.render_surface_publication(
                            pending.publication.paint_publication(),
                            self.resources.as_ref(),
                            || window.pre_present_notify(),
                        )
                    })
            }
            #[cfg(not(all(debug_assertions, feature = "internal-test-seams")))]
            self.renderer
                .as_mut()
                .ok_or(PublicationRenderError::SurfaceUnavailable)
                .and_then(|renderer| {
                    renderer.render_surface_publication(
                        pending.publication.paint_publication(),
                        self.resources.as_ref(),
                        || window.pre_present_notify(),
                    )
                })
        };
        match render_result {
            Ok(observation) => {
                if !observation.presented() {
                    self.fail(
                        event_loop,
                        "renderer reported success without native presentation",
                    );
                    return;
                }
                #[cfg(all(debug_assertions, feature = "internal-test-seams"))]
                if self.test_first_pending.take().is_some() {
                    eprintln!("RUNENUI_NATIVE_TEST_RENDER_RECOVERED=same_immutable_publication");
                }
                self.displayed_frame = Some(DisplayedFrame::from_pending(&pending));
                self.pending_frame = None;
                self.presentation_retry.reset();
                self.drive_runtime(event_loop);
                self.request_pending_redraw();
            }
            Err(
                PublicationRenderError::SurfaceTimeout | PublicationRenderError::SurfaceOccluded,
            ) => self.presentation_retry.defer(Instant::now()),
            Err(
                PublicationRenderError::SurfaceOutdated
                | PublicationRenderError::SurfaceSuboptimal
                | PublicationRenderError::SurfaceNotConfigured,
            ) => {
                if let Err(error) = self.configure_renderer(true) {
                    self.fail(event_loop, &error);
                } else {
                    self.presentation_retry.defer(Instant::now());
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
                    self.presentation_retry.defer(Instant::now());
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
        if self.exit_requested {
            return;
        }
        if let Err(error) = self.prepare_native_target(event_loop) {
            self.fail(event_loop, &error);
            return;
        }
        self.presentation_suppressed = false;
        self.drive_runtime(event_loop);
        self.request_pending_redraw();
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            return;
        }
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
        self.presentation_retry.reset();
        self.mapping_publication_needed = false;
        self.presentation_suppressed = true;
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: HostEvent) {
        if self.exit_requested {
            return;
        }
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
        if self.exit_requested {
            return;
        }
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
                // A later already-queued redraw/input event must not publish
                // against this now-closed runtime during OS-loop teardown.
                self.exit_requested = true;
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
                    // An OS visibility transition permits an immediate retry.
                    self.presentation_retry.reset();
                    self.request_pending_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.render_pending(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            return;
        }
        // Due timer/motion/type-ahead work needs an explicit bounded canonical
        // pump. The OS is not required to send another window event for it.
        if self
            .next_deadline
            .is_some_and(|deadline| deadline <= self.native_clock.now())
        {
            self.drive_runtime(event_loop);
        }
        // Direct input/AccessKit paths also may terminalize. Never park a
        // terminal runtime merely because the platform became idle.
        if let RuntimeStatus::Terminal(reason) = self.runtime.status() {
            self.fail(event_loop, &format!("runtime terminal: {reason:?}"));
            return;
        }
        let now = Instant::now();
        let future_deadline = self
            .next_deadline
            .and_then(|deadline| self.native_clock.host_deadline(deadline))
            .filter(|deadline| *deadline > now);
        // A recoverable presentation failure schedules a native-only wake.
        // It never re-enters the runtime, invents a publication, or polls
        // continuously while a surface is occluded or timing out.
        let presentation_deadline = if self.presentation_suppressed || self.pending_frame.is_none()
        {
            None
        } else {
            self.presentation_retry.future_deadline(now)
        };
        let next_wake = future_deadline
            .into_iter()
            .chain(presentation_deadline)
            .min();
        // If a runtime deadline remains due after this one bounded pump,
        // rely on the canonical pending-wake handshake rather than a spin.
        event_loop.set_control_flow(next_wake.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_framework_services_preserve_exact_response_kind() {
        let requests = [
            FrameworkServiceRequest::ClipboardReadText { max_bytes: 64 },
            FrameworkServiceRequest::InputMethod {
                enabled: false,
                candidate_area: None,
                composition: None,
            },
            FrameworkServiceRequest::Cursor {
                shape: runenui_core::CursorShape::Default,
                visible: true,
            },
        ];
        for request in &requests {
            let response = unavailable_framework_response(request)
                .unwrap_or_else(|| unreachable!("known service request must be representable"));
            assert_eq!(response.kind(), request.response_kind());
        }
    }

    #[cfg(all(debug_assertions, feature = "internal-test-seams"))]
    #[test]
    fn native_fault_injection_accepts_only_named_recoverable_surface_statuses() {
        let errors = parse_native_render_faults(
            "timeout,occluded,outdated,suboptimal,not-configured,lost",
        );
        assert_eq!(
            errors.into_iter().collect::<Vec<_>>(),
            vec![
                PublicationRenderError::SurfaceTimeout,
                PublicationRenderError::SurfaceOccluded,
                PublicationRenderError::SurfaceOutdated,
                PublicationRenderError::SurfaceSuboptimal,
                PublicationRenderError::SurfaceNotConfigured,
                PublicationRenderError::SurfaceLost,
            ]
        );
    }

    #[test]
    fn recoverable_presentation_retry_is_deferred_and_bounded() {
        let start = Instant::now();
        let mut retry = PresentationRetry::default();
        assert!(retry.is_due(start));
        for (failure, delay_ms) in [16, 32, 64, 128, 256, 256, 256].into_iter().enumerate() {
            let attempted_at = start + Duration::from_secs(failure as u64);
            retry.defer(attempted_at);
            let deadline = attempted_at + Duration::from_millis(delay_ms);
            assert_eq!(retry.future_deadline(attempted_at), Some(deadline));
            assert!(!retry.is_due(attempted_at));
            assert!(!retry.is_due(attempted_at + Duration::from_millis(delay_ms / 2)));
            assert!(retry.is_due(deadline));
        }
        retry.reset();
        assert!(retry.is_due(start));
        retry.defer(start);
        assert_eq!(
            retry.future_deadline(start),
            Some(start + Duration::from_millis(16))
        );
    }

    #[test]
    fn native_monotonic_deadline_uses_identical_runtime_origin() {
        let origin = Instant::now();
        let clock = NativeMonotonicClock { origin };
        let delta = Duration::from_millis(17);
        let logical = MonotonicInstant::ZERO
            .checked_add(delta)
            .unwrap_or_else(|_| unreachable!("17 milliseconds fit runtime time"));
        assert_eq!(clock.host_deadline(logical), origin.checked_add(delta));
    }

    #[test]
    fn native_line_wheel_uses_documented_logical_units() {
        let event = normalize_wheel_delta(MouseScrollDelta::LineDelta(0.0, -1.0), 2.0)
            .unwrap_or_else(|| unreachable!("finite line delta must be representable"));
        assert_eq!(event.x(), 0.0);
        assert_eq!(event.y(), -48.0);
    }

    #[test]
    fn native_pixel_wheel_uses_displayed_scale() {
        let event = normalize_wheel_delta(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(40.0, -20.0)),
            2.0,
        )
        .unwrap_or_else(|| unreachable!("finite pixel delta must be representable"));
        assert_eq!(event.x(), 20.0);
        assert_eq!(event.y(), -10.0);
    }

    #[test]
    fn native_pixel_wheel_fails_closed_on_invalid_display_scale() {
        let wheel = MouseScrollDelta::PixelDelta(PhysicalPosition::new(5.0, 7.0));
        assert!(normalize_wheel_delta(wheel, 0.0).is_none());
        assert!(
            normalize_wheel_delta(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(5.0, 7.0)),
                f64::NAN,
            )
            .is_none()
        );
    }
}
