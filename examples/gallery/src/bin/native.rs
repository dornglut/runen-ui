//! Standalone native Gallery host over accepted public `RunenUI` contracts.
//!
//! Only this executable owns its winit window, pump, successful displayed
//! input context, GPU recovery and OS input lifecycle. No widget/editor or
//! runtime authority is reimplemented here.

use std::{
    future::Future,
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
    time::Instant,
};

use gallery::{GalleryApp, GalleryState, style_environment};
use runenui_core::{
    CommittedTextEvent, ElementId, InputDeviceId, KeyModifiers, LogicalDelta, PointerEvent,
    SemanticCommand, StyleEnvironment, SurfaceInputContext,
};
use runenui_render_wgpu::{
    PublicationRenderError, Renderer, RendererOptions, ResourcePayload, ResourceProvider,
    ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{
    AppRuntime, FontSourcePolicy, MonotonicClock, MonotonicInstant, PumpBudget, RedrawRequest,
    RuntimeConfig, SubmitCompositionErrorKind, SubmitKeyboardErrorKind, SubmitTextErrorKind,
    SurfaceBuildContext, SurfacePublication,
};
use runenui_winit::{
    accessibility::{AccessibilityEvent, SemanticAdapter},
    device_identity::{DeviceIdentityError, DeviceIdentityMap},
    keyboard_input::{KeyboardInputOutcome, KeyboardInputState, NativeKeyTransition},
    mouse_input::{MouseButtonOutcome, MouseInputState, TranslatedPointerPoint},
    native_mapping::NativeMapping,
    text_input::{TextInputState, keyboard_committed_text_candidate, translate_preedit_range},
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const INITIAL_SIZE: PhysicalSize<u32> = PhysicalSize::new(980, 700);
const PUMP_BUDGET: PumpBudget = PumpBudget::new(128, 128, 128, 128);
const WHEEL_LINE_STEP: f64 = 48.0;

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

struct NoResources;

impl ResourceProvider for NoResources {
    fn load(
        &self,
        _resource: &runenui_core::ResourceRef,
        _request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Missing,
            "Gallery has no external image resources",
        ))
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

fn block_on<T: Future>(future: T) -> T::Output {
    let mut future = pin!(future);
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(result) => return result,
            Poll::Pending => thread::park(),
        }
    }
}

struct HostClock(Instant);

impl MonotonicClock for HostClock {
    fn now(&self) -> MonotonicInstant {
        MonotonicInstant::ZERO
            .checked_add(self.0.elapsed())
            .unwrap_or_else(|_| unreachable!("host lifetime fits monotonic nanosecond range"))
    }
}

fn modifiers(native: ModifiersState) -> KeyModifiers {
    let mut result = KeyModifiers::NONE;
    if native.shift_key() {
        result = result.with_shift();
    }
    if native.control_key() {
        result = result.with_control();
    }
    if native.alt_key() {
        result = result.with_alt();
    }
    if native.super_key() {
        result = result.with_meta();
    }
    result
}

/// Native wheel policy remains specific to the Gallery rather than the framework.
#[allow(
    clippy::cast_possible_truncation,
    reason = "native f64 wheel values are checked for finite f32-range before conversion"
)]
fn wheel_delta(delta: MouseScrollDelta, displayed_scale: f64) -> Option<LogicalDelta> {
    let (x, y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (
            f64::from(x) * WHEEL_LINE_STEP,
            f64::from(y) * WHEEL_LINE_STEP,
        ),
        MouseScrollDelta::PixelDelta(position) => {
            if !displayed_scale.is_finite() || displayed_scale <= 0.0 {
                return None;
            }
            (position.x / displayed_scale, position.y / displayed_scale)
        }
    };
    if !x.is_finite()
        || !y.is_finite()
        || x < f64::from(f32::MIN)
        || x > f64::from(f32::MAX)
        || y < f64::from(f32::MIN)
        || y > f64::from(f32::MAX)
    {
        return None;
    }
    LogicalDelta::new(x as f32, y as f32).ok()
}

#[derive(Clone)]
struct PendingFrame {
    publication: SurfacePublication,
    mapping: NativeMapping,
}

struct DisplayedFrame {
    input_context: SurfaceInputContext,
    mapping: NativeMapping,
}

// Native OS facts only; not a mirror of runtime focus, editing or surface identity.
#[derive(Default)]
struct WindowFlags {
    needs_publication: bool,
    suppressed: bool,
    focused: bool,
}

#[derive(Default)]
struct InputFlags {
    ime_enabled: bool,
    initialized_focus: bool,
}

fn fatal(event_loop: &ActiveEventLoop, detail: &str) {
    eprintln!("Gallery native host: {detail}");
    event_loop.exit();
}

struct GalleryHost {
    runtime: AppRuntime<GalleryApp>,
    style_environment: StyleEnvironment,
    proxy: EventLoopProxy<HostEvent>,
    window: Option<Arc<Window>>,
    accessibility: Option<accesskit_winit::Adapter>,
    semantics: SemanticAdapter,
    renderer: Option<Renderer>,
    mapping: Option<NativeMapping>,
    pending_redraw: Option<RedrawRequest>,
    pending_frame: Option<PendingFrame>,
    displayed: Option<DisplayedFrame>,
    devices: DeviceIdentityMap,
    mouse: MouseInputState,
    keyboard: KeyboardInputState,
    text: TextInputState,
    modifiers: KeyModifiers,
    window_flags: WindowFlags,
    input_flags: InputFlags,
}

impl GalleryHost {
    fn new(proxy: EventLoopProxy<HostEvent>) -> Self {
        let mut runtime = AppRuntime::<GalleryApp>::mount_with_config(
            GalleryState::default(),
            RuntimeConfig::default()
                .with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
        );
        runtime.set_monotonic_clock(HostClock(Instant::now()));
        let wake = proxy.clone();
        runtime.set_wake_transport(move || {
            let _ = wake.send_event(HostEvent::Wake);
        });
        Self {
            runtime,
            style_environment: style_environment(),
            proxy,
            window: None,
            accessibility: None,
            semantics: SemanticAdapter::new(),
            renderer: None,
            mapping: None,
            pending_redraw: None,
            pending_frame: None,
            displayed: None,
            devices: DeviceIdentityMap::default(),
            mouse: MouseInputState::default(),
            keyboard: KeyboardInputState::default(),
            text: TextInputState::default(),
            modifiers: KeyModifiers::NONE,
            window_flags: WindowFlags {
                focused: true,
                ..WindowFlags::default()
            },
            input_flags: InputFlags::default(),
        }
    }

    fn device(&mut self, event_loop: &ActiveEventLoop, id: DeviceId) -> Option<InputDeviceId> {
        match self.devices.resolve(id) {
            Ok(id) => Some(id),
            Err(DeviceIdentityError::Exhausted) => {
                fatal(event_loop, "native device identity exhausted");
                None
            }
        }
    }

    fn sync_text(&mut self) {
        let owner = self.runtime.focus().focused_node().cloned();
        let capability = self.runtime.focused_text_input_capability();
        let reset = self.text.sync_runtime(owner, capability).reset_native_ime();
        if reset {
            self.set_ime_enabled(false);
        }
        self.set_ime_enabled(!self.window_flags.suppressed && self.text.accepts_composition());
    }

    fn set_ime_enabled(&mut self, enabled: bool) {
        if enabled != self.input_flags.ime_enabled {
            if let Some(window) = self.window.as_ref() {
                window.set_ime_allowed(enabled);
            }
            self.input_flags.ime_enabled = enabled;
        }
    }

    fn pump(&mut self) {
        match self.runtime.pump(PUMP_BUDGET) {
            Ok(batch) => {
                if let Some(reason) = batch.pause_reason() {
                    eprintln!("Gallery runtime input observation paused: {reason:?}");
                }
                self.sync_text();
            }
            Err(error) => eprintln!("Gallery runtime pump deferred: {error}"),
        }
    }

    fn initial_focus(&mut self, event_loop: &ActiveEventLoop) {
        if self.input_flags.initialized_focus || !self.window_flags.focused {
            return;
        }
        self.input_flags.initialized_focus = true;
        let id = ElementId::new("gallery.nav.basics")
            .unwrap_or_else(|_| unreachable!("static Gallery focus id is valid"));
        if let Err(error) = self
            .runtime
            .submit_automation_command(id, SemanticCommand::RequestFocus)
        {
            fatal(
                event_loop,
                &format!("initial focus could not be requested: {error:?}"),
            );
        } else {
            self.pump();
        }
    }

    fn drive(&mut self, event_loop: &ActiveEventLoop) {
        self.pump();
        self.initial_focus(event_loop);
        if self.pending_redraw.is_none() {
            self.pending_redraw = self.runtime.take_redraw_request();
        }
    }

    fn create_target(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        if self.window.is_none() {
            let window = event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("RunenUI Gallery")
                        .with_inner_size(INITIAL_SIZE)
                        .with_visible(false),
                )
                .map_err(|error| format!("window creation failed: {error}"))?;
            let accessibility = accesskit_winit::Adapter::with_mixed_handlers(
                event_loop,
                &window,
                self.semantics.activation_handler(),
                self.proxy.clone(),
            );
            self.window = Some(Arc::new(window));
            self.accessibility = Some(accessibility);
        }
        if self.renderer.is_none() {
            let window = self
                .window
                .clone()
                .ok_or_else(|| "renderer requires the Gallery window".to_owned())?;
            self.renderer = Some(
                block_on(Renderer::request_with_surface_target(
                    RendererOptions::new(),
                    Box::new(event_loop.owned_display_handle()),
                    window,
                ))
                .map_err(|error| format!("GPU target unavailable: {error}"))?,
            );
        }
        self.refresh_mapping();
        self.configure_renderer(false)?;
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
        }
        Ok(())
    }

    fn refresh_mapping(&mut self) -> bool {
        let next = self.window.as_deref().and_then(NativeMapping::from_window);
        if self.mapping == next {
            return false;
        }
        self.mapping = next;
        self.pending_frame = None;
        self.window_flags.needs_publication = next.is_some();
        true
    }

    fn configure_renderer(&mut self, force: bool) -> Result<(), String> {
        let Some(mapping) = self.mapping else {
            return Ok(());
        };
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| "renderer is missing".to_owned())?;
        let size = mapping.physical_size();
        let configured = renderer
            .configured_surface_extent()
            .is_some_and(|extent| extent.width() == size.width && extent.height() == size.height);
        if force || !configured {
            renderer
                .configure_surface(size.width, size.height)
                .map_err(|error| format!("surface configuration failed: {error}"))?;
        }
        Ok(())
    }

    fn publish(&mut self) -> Result<(), String> {
        if self.pending_frame.is_some() {
            return Ok(());
        }
        let Some(mapping) = self.mapping else {
            return Ok(());
        };
        let configured = self
            .renderer
            .as_ref()
            .and_then(Renderer::configured_surface_extent);
        if !configured.is_some_and(|extent| {
            extent.width() == mapping.physical_size().width
                && extent.height() == mapping.physical_size().height
        }) {
            return Ok(());
        }
        if self.pending_redraw.is_none() {
            self.pending_redraw = self.runtime.take_redraw_request();
        }
        if self.pending_redraw.is_none() && !self.window_flags.needs_publication {
            return Ok(());
        }
        let context = SurfaceBuildContext::tight(&self.style_environment, mapping.logical_size())
            .with_raster_scale(mapping.raster_scale());
        let publication = self
            .runtime
            .publish_surface(&context)
            .map_err(|error| format!("surface publication failed: {error:?}"))?;
        let semantic = self.semantics.update(publication.semantic_publication());
        for diagnostic in &semantic.diagnostics {
            eprintln!("Gallery accessibility: {diagnostic:?}");
        }
        if let Some(accessibility) = self.accessibility.as_mut() {
            let tree = semantic.tree_update;
            accessibility.update_if_active(|| tree);
        }
        if let Some(redraw) = self.pending_redraw.take() {
            self.runtime
                .acknowledge_redraw(&redraw)
                .map_err(|error| format!("redraw acknowledgement failed: {error:?}"))?;
        }
        self.window_flags.needs_publication = false;
        self.pending_frame = Some(PendingFrame {
            publication,
            mapping,
        });
        self.redraw();
        Ok(())
    }

    fn redraw(&self) {
        if self.window_flags.suppressed {
            return;
        }
        if (self.pending_frame.is_some()
            || self.pending_redraw.is_some()
            || self.window_flags.needs_publication)
            && let Some(window) = self.window.as_ref()
        {
            window.request_redraw();
        }
    }

    fn cancel_pointer(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(cancel) = self.mouse.invalidate_point_authority(self.modifiers) {
            self.submit_pointer(event_loop, cancel);
        }
    }

    fn cancel_keyboard(&mut self, event_loop: &ActiveEventLoop) {
        for cancel in self
            .keyboard
            .cancel_all(self.modifiers, self.text.keyboard_composition_state())
        {
            match self.runtime.submit_keyboard(cancel) {
                Ok(_) => {}
                Err(error) if error.kind() == SubmitKeyboardErrorKind::NoFocusedTarget => {}
                Err(error) => {
                    fatal(
                        event_loop,
                        &format!("keyboard cancellation failed: {error}"),
                    );
                    return;
                }
            }
        }
        self.pump();
    }

    fn submit_pointer(&mut self, event_loop: &ActiveEventLoop, event: PointerEvent) {
        self.pump();
        if let Err(error) = self.runtime.submit_pointer(event) {
            fatal(event_loop, &format!("native pointer input failed: {error}"));
            return;
        }
        self.drive(event_loop);
        self.redraw();
    }

    fn pointer_point(&self) -> Option<TranslatedPointerPoint> {
        let displayed = self.displayed.as_ref()?;
        let native = self.mouse.last_native_position()?;
        let position = displayed
            .mapping
            .translate_cursor(self.mapping, native)
            .ok()?;
        Some(TranslatedPointerPoint {
            position,
            input_context: displayed.input_context.clone(),
            modifiers: self.modifiers,
        })
    }

    fn cursor_moved(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_id: DeviceId,
        position: PhysicalPosition<f64>,
    ) {
        let Some(device) = self.device(event_loop, native_id) else {
            return;
        };
        if self.mouse.active_device_id().is_some_and(|id| id != device)
            && let Some(cancel) = self.mouse.cancel_for_device_change(self.modifiers)
        {
            self.submit_pointer(event_loop, cancel);
        }
        self.mouse.note_cursor_position(position);
        let Some(point) = self.pointer_point() else {
            self.cancel_pointer(event_loop);
            return;
        };
        match self.mouse.cursor_moved(device, point) {
            Ok(event) => self.submit_pointer(event_loop, event),
            Err(error) => fatal(
                event_loop,
                &format!("native cursor input invalid: {error:?}"),
            ),
        }
    }

    fn mouse_button(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) {
        let Some(device) = self.device(event_loop, native_id) else {
            return;
        };
        if state == ElementState::Pressed
            && self.mouse.active_device_id().is_some_and(|id| id != device)
            && let Some(cancel) = self.mouse.cancel_for_device_change(self.modifiers)
        {
            self.submit_pointer(event_loop, cancel);
        }
        let point = self.pointer_point();
        if point.is_none() {
            self.cancel_pointer(event_loop);
        }
        match self.mouse.button_input(device, state, button, point) {
            Ok(MouseButtonOutcome::Submit(event)) => self.submit_pointer(event_loop, event),
            Ok(MouseButtonOutcome::Suppressed(_)) => {}
            Err(error) => fatal(
                event_loop,
                &format!("native mouse button invalid: {error:?}"),
            ),
        }
    }

    fn mouse_wheel(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_id: DeviceId,
        delta: MouseScrollDelta,
    ) {
        let zero = match &delta {
            MouseScrollDelta::LineDelta(x, y) => *x == 0.0 && *y == 0.0,
            MouseScrollDelta::PixelDelta(position) => position.x == 0.0 && position.y == 0.0,
        };
        if zero {
            return;
        }
        let Some(device) = self.device(event_loop, native_id) else {
            return;
        };
        let Some(point) = self.pointer_point() else {
            self.cancel_pointer(event_loop);
            return;
        };
        let Some(scale) = self
            .displayed
            .as_ref()
            .map(|frame| frame.mapping.native_scale_factor())
        else {
            return;
        };
        let Some(delta) = wheel_delta(delta, scale) else {
            return;
        };
        if self.mouse.active_device_id().is_some_and(|id| id != device)
            && let Some(cancel) = self.mouse.cancel_for_device_change(self.modifiers)
        {
            self.submit_pointer(event_loop, cancel);
        }
        match self.mouse.wheel(device, point, delta) {
            Ok(event) => self.submit_pointer(event_loop, event),
            Err(error) => fatal(
                event_loop,
                &format!("native wheel input invalid: {error:?}"),
            ),
        }
    }

    fn commit_text(
        &mut self,
        event_loop: &ActiveEventLoop,
        text: &str,
        device: Option<InputDeviceId>,
    ) {
        if text.is_empty() || !self.text.accepts_committed_text() {
            return;
        }
        let event = CommittedTextEvent::new(text.to_owned(), device)
            .unwrap_or_else(|_| unreachable!("nonempty native text is representable"));
        match self.runtime.submit_text(event) {
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    SubmitTextErrorKind::NoFocusedTarget
                        | SubmitTextErrorKind::FocusedTargetNotTextCapable
                ) => {}
            Err(error) => fatal(
                event_loop,
                &format!("native committed text failed: {error}"),
            ),
        }
    }

    fn key_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        native_id: DeviceId,
        event: &winit::event::KeyEvent,
        synthetic: bool,
    ) {
        self.pump();
        let Some(device) = self.device(event_loop, native_id) else {
            return;
        };
        let composition = self.text.keyboard_composition_state();
        let transition = NativeKeyTransition::from_event(event, synthetic);
        match self
            .keyboard
            .key_input(device, &transition, self.modifiers, composition)
        {
            KeyboardInputOutcome::Submit(key) => {
                let committed = keyboard_committed_text_candidate(
                    event.state,
                    synthetic,
                    self.text.accepts_committed_text(),
                    composition,
                    key.logical_key(),
                    event.text.as_deref(),
                )
                .map(str::to_owned);
                match self.runtime.submit_keyboard(key) {
                    Ok(_) => {}
                    Err(error) if error.kind() == SubmitKeyboardErrorKind::NoFocusedTarget => {}
                    Err(error) => {
                        fatal(
                            event_loop,
                            &format!("native keyboard input failed: {error}"),
                        );
                        return;
                    }
                }
                if let Some(text) = committed {
                    self.commit_text(event_loop, &text, Some(device));
                }
            }
            KeyboardInputOutcome::Suppressed(_) => return,
        }
        self.drive(event_loop);
        self.redraw();
    }

    fn cancel_composition(&mut self, event_loop: &ActiveEventLoop) {
        let Some(generation) = self.text.composition_generation().cloned() else {
            return;
        };
        match self.runtime.cancel_composition(generation) {
            Ok(_) => self.text.retire_composition(),
            Err(error)
                if matches!(
                    error.kind(),
                    SubmitCompositionErrorKind::MissingGeneration
                        | SubmitCompositionErrorKind::StaleGeneration
                        | SubmitCompositionErrorKind::NoFocusedTarget
                        | SubmitCompositionErrorKind::FocusedTargetNotCompositionCapable
                ) =>
            {
                self.text.retire_composition();
            }
            Err(error) => fatal(event_loop, &format!("native IME cancel failed: {error}")),
        }
    }

    fn ime_event(&mut self, event_loop: &ActiveEventLoop, ime: Ime) {
        self.pump();
        match ime {
            Ime::Enabled => {}
            Ime::Disabled => self.cancel_composition(event_loop),
            Ime::Preedit(preedit, native_range) => {
                let Ok(range) = translate_preedit_range(&preedit, native_range) else {
                    eprintln!("Gallery rejected invalid native preedit UTF-8 range");
                    return;
                };
                if preedit.is_empty() && self.text.composition_generation().is_none() {
                    return;
                }
                let generation = if let Some(generation) = self.text.composition_generation() {
                    Some(generation.clone())
                } else if self.text.accepts_composition() {
                    match self.runtime.start_composition(None) {
                        Ok(submission) => {
                            let generation = submission.generation().clone();
                            self.text
                                .remember_composition_generation(generation.clone());
                            Some(generation)
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                SubmitCompositionErrorKind::NoFocusedTarget
                                    | SubmitCompositionErrorKind::FocusedTargetNotCompositionCapable
                            ) =>
                        {
                            None
                        }
                        Err(error) => {
                            fatal(event_loop, &format!("native IME start failed: {error}"));
                            None
                        }
                    }
                } else {
                    None
                };
                let Some(generation) = generation else {
                    return;
                };
                match self
                    .runtime
                    .submit_composition_update(generation, preedit, range)
                {
                    Ok(_) => {}
                    Err(error)
                        if matches!(
                            error.kind(),
                            SubmitCompositionErrorKind::StaleGeneration
                                | SubmitCompositionErrorKind::MissingGeneration
                        ) =>
                    {
                        self.text.retire_composition();
                    }
                    Err(error) => {
                        fatal(event_loop, &format!("native IME preedit failed: {error}"));
                        return;
                    }
                }
            }
            Ime::Commit(text) => {
                if let Some(generation) = self.text.composition_generation().cloned() {
                    match self.runtime.submit_composition_end(generation) {
                        Ok(_) => self.text.retire_composition(),
                        Err(error)
                            if matches!(
                                error.kind(),
                                SubmitCompositionErrorKind::StaleGeneration
                                    | SubmitCompositionErrorKind::MissingGeneration
                            ) =>
                        {
                            self.text.retire_composition();
                        }
                        Err(error) => {
                            fatal(event_loop, &format!("native IME end failed: {error}"));
                            return;
                        }
                    }
                }
                self.commit_text(event_loop, &text, None);
            }
        }
        self.drive(event_loop);
        self.redraw();
    }

    fn present(&mut self, event_loop: &ActiveEventLoop) {
        self.drive(event_loop);
        if let Err(error) = self.publish() {
            fatal(event_loop, &error);
            return;
        }
        let Some(frame) = self.pending_frame.clone() else {
            return;
        };
        if self.mapping != Some(frame.mapping) {
            self.pending_frame = None;
            self.window_flags.needs_publication = self.mapping.is_some();
            self.redraw();
            return;
        }
        if self.window_flags.suppressed {
            return;
        }
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let result = self
            .renderer
            .as_mut()
            .ok_or(PublicationRenderError::SurfaceUnavailable)
            .and_then(|renderer| {
                renderer.render_surface_publication(
                    frame.publication.paint_publication(),
                    &NoResources,
                    || window.pre_present_notify(),
                )
            });
        match result {
            Ok(observation) if observation.presented() => {
                self.displayed = Some(DisplayedFrame {
                    input_context: frame.publication.input_context().clone(),
                    mapping: frame.mapping,
                });
                self.pending_frame = None;
                self.drive(event_loop);
                self.redraw();
            }
            Ok(_) => fatal(event_loop, "renderer did not actually present the Gallery"),
            Err(
                PublicationRenderError::SurfaceTimeout | PublicationRenderError::SurfaceOccluded,
            ) => {
                self.redraw();
            }
            Err(
                PublicationRenderError::SurfaceOutdated
                | PublicationRenderError::SurfaceSuboptimal
                | PublicationRenderError::SurfaceNotConfigured,
            ) => {
                if let Err(error) = self.configure_renderer(true) {
                    fatal(event_loop, &error);
                } else {
                    self.redraw();
                }
            }
            Err(PublicationRenderError::SurfaceLost) => {
                self.cancel_pointer(event_loop);
                self.displayed = None;
                self.pending_frame = None;
                self.renderer = None;
                self.window_flags.needs_publication = self.mapping.is_some();
                if let Err(error) = self
                    .create_target(event_loop)
                    .and_then(|()| self.configure_renderer(false))
                {
                    fatal(event_loop, &error);
                } else {
                    self.redraw();
                }
            }
            Err(error) => fatal(event_loop, &format!("native presentation failed: {error}")),
        }
    }

    fn accessibility_event(&mut self, event_loop: &ActiveEventLoop, event: AccessibilityEvent) {
        if let AccessibilityEvent::ActionRequested(request) = event
            && let Ok(action) = self.semantics.action_request(&request)
            && let Err(error) = self.runtime.submit_semantic_action(action)
        {
            eprintln!("Gallery accessibility action withheld: {error:?}");
        }
        self.drive(event_loop);
        self.redraw();
    }

    fn mapping_changed(&mut self, event_loop: &ActiveEventLoop) {
        if self.refresh_mapping() {
            self.cancel_pointer(event_loop);
            if let Err(error) = self.configure_renderer(false) {
                fatal(event_loop, &error);
                return;
            }
        }
        self.drive(event_loop);
        self.redraw();
    }

    fn lose_native_focus(&mut self, event_loop: &ActiveEventLoop) {
        self.window_flags.focused = false;
        self.cancel_keyboard(event_loop);
        self.cancel_pointer(event_loop);
        self.cancel_composition(event_loop);
        self.text.set_window_focused(false);
        self.set_ime_enabled(false);
        self.modifiers = KeyModifiers::NONE;
        self.drive(event_loop);
    }
}

impl ApplicationHandler<HostEvent> for GalleryHost {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.create_target(event_loop) {
            fatal(event_loop, &error);
            return;
        }
        self.window_flags.suppressed = false;
        self.window_flags.focused = self
            .window
            .as_ref()
            .is_some_and(|window| window.has_focus());
        self.text.set_window_focused(self.window_flags.focused);
        self.drive(event_loop);
        self.redraw();
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.lose_native_focus(event_loop);
        self.renderer = None;
        self.mapping = None;
        self.pending_frame = None;
        self.displayed = None;
        self.window_flags.needs_publication = false;
        self.window_flags.suppressed = true;
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: HostEvent) {
        match event {
            HostEvent::Wake => self.drive(event_loop),
            HostEvent::Accessibility(event) => self.accessibility_event(event_loop, event),
        }
        self.redraw();
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
                event_loop.exit();
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.mapping_changed(event_loop);
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } if !self.window_flags.suppressed => {
                self.cursor_moved(event_loop, device_id, position);
            }
            WindowEvent::CursorLeft { .. } => self.cancel_pointer(event_loop),
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } if !self.window_flags.suppressed => {
                self.mouse_button(event_loop, device_id, state, button);
            }
            WindowEvent::MouseWheel {
                device_id, delta, ..
            } if !self.window_flags.suppressed => {
                self.mouse_wheel(event_loop, device_id, delta);
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } if !self.window_flags.suppressed && self.window_flags.focused => {
                self.key_input(event_loop, device_id, &event, is_synthetic);
            }
            WindowEvent::Ime(ime) if !self.window_flags.suppressed && self.window_flags.focused => {
                self.ime_event(event_loop, ime);
            }
            WindowEvent::ModifiersChanged(state) => self.modifiers = modifiers(state.state()),
            WindowEvent::Focused(false) => self.lose_native_focus(event_loop),
            WindowEvent::Focused(true) => {
                self.window_flags.focused = true;
                self.text.set_window_focused(true);
                self.drive(event_loop);
            }
            WindowEvent::Occluded(occluded) => {
                if occluded {
                    self.lose_native_focus(event_loop);
                }
                self.window_flags.suppressed = occluded;
                if !occluded {
                    self.window_flags.focused = self
                        .window
                        .as_ref()
                        .is_some_and(|window| window.has_focus());
                    self.text.set_window_focused(self.window_flags.focused);
                    self.drive(event_loop);
                }
                self.redraw();
            }
            WindowEvent::RedrawRequested => self.present(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.redraw();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.set_ime_enabled(false);
        if let Err(error) = self.runtime.shutdown() {
            eprintln!("Gallery shutdown observation unavailable: {error}");
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<HostEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();
    event_loop.run_app(&mut GalleryHost::new(proxy))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_text_adapter_revokes_capability_when_page_unmounts_editor() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        app.pump(PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("Gallery root settles"));
        let inputs = ElementId::new("gallery.nav.inputs")
            .unwrap_or_else(|_| unreachable!("static navigation id is valid"));
        app.submit_automation_command(inputs, SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("Inputs navigation is mounted"));
        app.pump(PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("Inputs selection settles"));
        let editor = ElementId::new("gallery.inputs.textfield")
            .unwrap_or_else(|_| unreachable!("static editor id is valid"));
        app.submit_automation_command(editor, SemanticCommand::RequestFocus)
            .unwrap_or_else(|_| unreachable!("TextField accepts focus"));
        app.pump(PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("TextField focus settles"));

        let mut adapter = TextInputState::default();
        adapter.set_window_focused(true);
        adapter.sync_runtime(
            app.focus().focused_node().cloned(),
            app.focused_text_input_capability(),
        );
        assert!(adapter.accepts_committed_text());
        assert!(adapter.accepts_composition());

        let basics = ElementId::new("gallery.nav.basics")
            .unwrap_or_else(|_| unreachable!("static navigation id is valid"));
        app.submit_automation_command(basics, SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("Basics navigation is mounted"));
        app.pump(PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("TextField unmount settles"));
        adapter.sync_runtime(
            app.focus().focused_node().cloned(),
            app.focused_text_input_capability(),
        );
        assert!(!adapter.accepts_committed_text());
        assert!(!adapter.accepts_composition());
    }

    #[test]
    fn gallery_native_wheel_policy_uses_displayed_pixel_scale() {
        let line = wheel_delta(MouseScrollDelta::LineDelta(1.0, -2.0), 4.0)
            .unwrap_or_else(|| unreachable!("line wheel fixture valid"));
        assert_eq!(line.x(), 48.0);
        assert_eq!(line.y(), -96.0);
        let pixel = wheel_delta(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(120.0, -30.0)),
            2.0,
        )
        .unwrap_or_else(|| unreachable!("pixel wheel fixture valid"));
        assert_eq!(pixel.x(), 60.0);
        assert_eq!(pixel.y(), -15.0);
        assert!(
            wheel_delta(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(12.0, 12.0)),
                0.0
            )
            .is_none()
        );
    }
}
