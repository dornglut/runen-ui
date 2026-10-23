use super::pointer_mode;
use super::{
    AppRuntime, CommandOrigin, CommittedTextEvent, DemoApp, DemoHistoryEntry, DemoState,
    DisplayedFrame, HOST_PUMP_BUDGET, INITIAL_EDITOR_TEXT, KeyboardEvent, LARGE_DOCUMENT_LINES,
    MAX_EDITOR_HISTORY_ENTRIES, NATIVE_RELEASE_RETRY_INTERVAL, NativeMapping, PendingFrame,
    PointIngressDiagnostic, RAW_MOTION_PROBE_TIMEOUT, ReferenceDocumentPreset,
    STRESS_DOCUMENT_LINES, SemanticAdapter, SemanticCommand, StyleEnvironment, SurfaceBuildContext,
    earliest_host_deadline, native_release_retry_due, push_editor_history,
    raw_motion_probe_expired, translate_modifiers,
};
use runenui_core::{
    InputDeviceId, KeyModifiers, KeyboardPhase, LogicalPoint, PointerButton, PointerPhase,
};
use runenui_runtime::{FontSourcePolicy, LogicalSize, RuntimeConfig};
use runenui_winit::mouse_input::{
    MouseButtonOutcome, MouseIngressDiagnostic, MouseInputState, TranslatedPointerPoint,
    translate_mouse_button,
};
use std::time::Instant;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
    keyboard::ModifiersState,
};

#[derive(Default)]
struct PointerProbe {
    fail_acquire: bool,
    failed_releases: u8,
    native_mode: pointer_mode::Mode,
    visible: bool,
    grabs: usize,
}

impl pointer_mode::Platform for PointerProbe {
    fn grab(&mut self, mode: pointer_mode::Mode) -> Result<(), pointer_mode::Failure> {
        self.grabs += 1;
        if mode == pointer_mode::Mode::Absolute && self.failed_releases > 0 {
            self.failed_releases -= 1;
            return Err(pointer_mode::Failure::Native);
        }
        self.native_mode = mode;
        if mode != pointer_mode::Mode::Absolute && self.fail_acquire {
            return Err(pointer_mode::Failure::Native);
        }
        Ok(())
    }

    fn cursor(&mut self, _shape: runenui_core::CursorShape, visible: bool) {
        self.visible = visible;
    }
}

#[test]
fn acquisition_rollback_failure_arms_retry_without_postponing_it() {
    let mut modes = pointer_mode::PointerModes::default();
    let mut native = PointerProbe {
        fail_acquire: true,
        failed_releases: 1,
        ..PointerProbe::default()
    };
    assert!(modes.focus_changed(true, &mut native).is_ok());
    let scope = pointer_mode::Scope {
        window_epoch: 1,
        surface: None,
    };
    assert_eq!(
        modes.request(scope, pointer_mode::Mode::LockedRelative, &mut native),
        Err(pointer_mode::Failure::ReleaseFailed)
    );
    assert_eq!(native.native_mode, pointer_mode::Mode::LockedRelative);
    assert!(native.visible);
    assert!(!modes.ui_pointer_allowed());
    let now = Instant::now();
    let deadline = super::native_release_retry_deadline(modes.state(), None, now);
    assert_eq!(deadline, Some(now + NATIVE_RELEASE_RETRY_INTERVAL));
    assert_eq!(
        super::native_release_retry_deadline(
            modes.state(),
            deadline,
            now + std::time::Duration::from_millis(500)
        ),
        deadline
    );
    assert!(modes.release(&mut native).is_ok());
    assert_eq!(native.native_mode, pointer_mode::Mode::Absolute);
    assert!(native.visible);
    assert_eq!(
        super::native_release_retry_deadline(modes.state(), deadline, now),
        None
    );
}

#[test]
fn stopped_runtime_revokes_lock_without_requiring_native_focus_loss() {
    for status in [
        runenui_runtime::RuntimeStatus::Closed,
        runenui_runtime::RuntimeStatus::Terminal(runenui_runtime::RuntimeTerminalReason::Poisoned),
    ] {
        let mut modes = pointer_mode::PointerModes::default();
        let mut native = PointerProbe::default();
        assert!(modes.focus_changed(true, &mut native).is_ok());
        let scope = pointer_mode::Scope {
            window_epoch: 1,
            surface: None,
        };
        assert!(
            modes
                .request(
                    scope.clone(),
                    pointer_mode::Mode::LockedRelative,
                    &mut native
                )
                .is_ok()
        );
        assert!(modes.observe_motion(&scope, (1.0, 2.0), &mut native));
        assert!(!native.visible);
        assert!(
            super::revoke_pointer_for_runtime_stop(
                runenui_runtime::RuntimeStatus::Running,
                &mut modes,
                &mut native
            )
            .is_ok()
        );
        assert!(modes.gameplay_motion_allowed());
        native.failed_releases = 1;
        assert_eq!(
            super::revoke_pointer_for_runtime_stop(status, &mut modes, &mut native),
            Err(pointer_mode::Failure::ReleaseFailed)
        );
        assert!(native.visible);
        assert!(!modes.gameplay_motion_allowed());
        assert!(!modes.ui_pointer_allowed());
        let grabs = native.grabs;
        assert!(super::revoke_pointer_for_runtime_stop(status, &mut modes, &mut native).is_ok());
        assert_eq!(
            native.grabs, grabs,
            "stopped-runtime observation must not busy-retry native release"
        );
        assert!(modes.release(&mut native).is_ok());
        assert!(!modes.ui_pointer_allowed());
        assert!(!modes.observe_motion(&scope, (2.0, 1.0), &mut native));
        assert_eq!(native.native_mode, pointer_mode::Mode::Absolute);
    }
}

#[test]
fn stopped_runtime_during_failed_acquisition_cannot_rearm_ui_on_retry() {
    let mut modes = pointer_mode::PointerModes::default();
    let mut native = PointerProbe {
        fail_acquire: true,
        failed_releases: 1,
        ..PointerProbe::default()
    };
    assert!(modes.focus_changed(true, &mut native).is_ok());
    let scope = pointer_mode::Scope {
        window_epoch: 1,
        surface: None,
    };
    assert_eq!(
        modes.request(scope, pointer_mode::Mode::LockedRelative, &mut native),
        Err(pointer_mode::Failure::ReleaseFailed)
    );
    let grabs = native.grabs;
    assert!(
        super::revoke_pointer_for_runtime_stop(
            runenui_runtime::RuntimeStatus::Closed,
            &mut modes,
            &mut native
        )
        .is_ok()
    );
    assert_eq!(
        native.grabs, grabs,
        "terminal observation must preserve the bounded retry"
    );
    assert!(modes.release(&mut native).is_ok());
    assert!(native.visible);
    assert!(
        !modes.ui_pointer_allowed(),
        "successful native retry must not revive stopped-runtime input"
    );
    assert!(!modes.gameplay_motion_allowed());
}

#[test]
fn lifecycle_and_repeated_release_intent_preserve_failed_unlock_deadline() {
    let mut modes = pointer_mode::PointerModes::default();
    let mut native = PointerProbe::default();
    assert!(modes.focus_changed(true, &mut native).is_ok());
    let scope = pointer_mode::Scope {
        window_epoch: 1,
        surface: None,
    };
    assert!(
        modes
            .request(scope, pointer_mode::Mode::ConfinedAbsolute, &mut native)
            .is_ok()
    );
    native.failed_releases = 2;
    let now = Instant::now();
    let mut retry = None;
    assert_eq!(
        super::release_native_pointer(&mut modes, &mut native, &mut retry, now),
        Err(pointer_mode::Failure::ReleaseFailed)
    );
    let first_deadline = Some(now + NATIVE_RELEASE_RETRY_INTERVAL);
    assert_eq!(retry, first_deadline);
    let grabs = native.grabs;
    for elapsed in [0, 100, 500, 999] {
        assert_eq!(
            modes.focus_changed(false, &mut native),
            Err(pointer_mode::Failure::ReleaseFailed)
        );
        assert_eq!(
            super::release_native_pointer(
                &mut modes,
                &mut native,
                &mut retry,
                now + std::time::Duration::from_millis(elapsed)
            ),
            Err(pointer_mode::Failure::ReleaseFailed)
        );
        assert_eq!(native.grabs, grabs);
        assert_eq!(retry, first_deadline);
        assert!(native.visible);
        assert!(!modes.ui_pointer_allowed());
    }
    assert_eq!(
        super::release_native_pointer(
            &mut modes,
            &mut native,
            &mut retry,
            now + NATIVE_RELEASE_RETRY_INTERVAL
        ),
        Err(pointer_mode::Failure::ReleaseFailed)
    );
    assert_eq!(native.grabs, grabs + 1);
    assert_eq!(retry, Some(now + NATIVE_RELEASE_RETRY_INTERVAL * 2));
    assert!(
        super::release_native_pointer(
            &mut modes,
            &mut native,
            &mut retry,
            now + NATIVE_RELEASE_RETRY_INTERVAL * 2
        )
        .is_ok()
    );
    assert_eq!(native.grabs, grabs + 2);
    assert_eq!(retry, None);
    assert_eq!(native.native_mode, pointer_mode::Mode::Absolute);
    assert!(native.visible);
    assert!(!modes.ui_pointer_allowed());
}

#[test]
fn occlusion_blocks_absolute_input_even_with_native_focus() {
    struct FakePointer;
    impl pointer_mode::Platform for FakePointer {
        fn grab(&mut self, _mode: pointer_mode::Mode) -> Result<(), pointer_mode::Failure> {
            Ok(())
        }
        fn cursor(&mut self, _shape: runenui_core::CursorShape, _visible: bool) {}
    }
    let mut modes = pointer_mode::PointerModes::default();
    assert!(modes.focus_changed(true, &mut FakePointer).is_ok());
    assert!(super::absolute_ui_ingress_allowed(false, &modes));
    assert!(!super::absolute_ui_ingress_allowed(true, &modes));
    assert!(modes.focus_changed(false, &mut FakePointer).is_ok());
    assert!(!super::absolute_ui_ingress_allowed(false, &modes));
    assert!(modes.focus_changed(true, &mut FakePointer).is_ok());
    assert!(!super::absolute_ui_ingress_allowed(true, &modes));
    assert!(!modes.ui_cursor_presentation_applied());
    modes.confirm_fresh_absolute_point(&mut FakePointer);
    assert!(super::absolute_ui_ingress_allowed(false, &modes));
}

#[test]
fn native_release_retry_uses_bounded_nonbusy_deadlines() {
    let now = Instant::now();
    let retry_at = now + NATIVE_RELEASE_RETRY_INTERVAL;
    let pending_at = now + RAW_MOTION_PROBE_TIMEOUT;
    assert!(!native_release_retry_due(Some(retry_at), now));
    assert!(!native_release_retry_due(None, now));
    assert!(native_release_retry_due(Some(retry_at), retry_at));
    assert_eq!(
        earliest_host_deadline(Some(pending_at), Some(retry_at)),
        Some(retry_at)
    );
    assert_eq!(
        earliest_host_deadline(Some(pending_at), None),
        Some(pending_at)
    );
    assert_eq!(earliest_host_deadline(None, Some(retry_at)), Some(retry_at));
    assert_eq!(earliest_host_deadline(None, None), None);
}

#[test]
fn raw_motion_probe_timeout_is_deterministic_at_boundary() {
    let start = Instant::now();
    assert!(!raw_motion_probe_expired(start, start));
    assert!(!raw_motion_probe_expired(
        start,
        start + std::time::Duration::from_secs(4)
    ));
    assert!(raw_motion_probe_expired(
        start,
        start + RAW_MOTION_PROBE_TIMEOUT
    ));
    assert!(raw_motion_probe_expired(
        start,
        start + RAW_MOTION_PROBE_TIMEOUT + std::time::Duration::from_nanos(1)
    ));
}

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

    let default_state = ReferenceDocumentPreset::Default.initial_state();
    assert_eq!(default_state.text, INITIAL_EDITOR_TEXT);
    assert_eq!(
        default_state.selection_seed.anchor(),
        INITIAL_EDITOR_TEXT.len()
    );
    assert_eq!(
        default_state.selection_seed.active(),
        INITIAL_EDITOR_TEXT.len()
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
        assert_eq!(state.selection_seed.anchor(), 0);
        assert_eq!(state.selection_seed.active(), 0);
    }
}

fn input_device(value: u64) -> InputDeviceId {
    InputDeviceId::new(value)
        .unwrap_or_else(|| unreachable!("fixture input device identity is non-zero"))
}

fn displayed_frame(mapping: NativeMapping) -> DisplayedFrame {
    let mut runtime = AppRuntime::<DemoApp>::mount(DemoState::default());
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    let style_environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(&style_environment, mapping.logical_size())
        .with_raster_scale(mapping.raster_scale());
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| unreachable!("fixture surface publication is valid: {error:?}"));
    DisplayedFrame {
        input_context: publication.input_context().clone(),
        mapping,
    }
}

#[test]
#[allow(clippy::assert_is_empty)]
fn reference_editor_publishes_visible_editable_text_and_accepts_commits() {
    let mut runtime = AppRuntime::<DemoApp>::mount_with_config(
        DemoState::default(),
        RuntimeConfig::default().with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
    );
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    let mapping = NativeMapping::from_parts(PhysicalSize::new(800, 480), 1.0)
        .unwrap_or_else(|| unreachable!("the fixture mapping is valid"));
    let style_environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(&style_environment, mapping.logical_size())
        .with_raster_scale(mapping.raster_scale());
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| unreachable!("reference editor publication is valid: {error:?}"));

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
                node.role() == runenui_core::SemanticRole::EditableText && node.editable().is_some()
            })
    );
    let accessibility_update = SemanticAdapter::new().update(publication.semantic_publication());
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    runtime
        .submit_text(
            CommittedTextEvent::new("!", None)
                .unwrap_or_else(|_| unreachable!("the fixture commit is valid")),
        )
        .unwrap_or_else(|_| unreachable!("reference editor accepts committed text"));
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();

    assert!(runtime.state().text.ends_with('!'));
    assert_eq!(runtime.state().revision, 1);
}

#[test]
#[allow(clippy::too_many_lines)] // Exercises the full native mouse ingress through publication.
fn native_mouse_drag_selects_preloaded_text_before_keyboard_input() {
    fn expect_ok<T, E: std::fmt::Debug>(result: Result<T, E>, message: &str) -> T {
        assert!(result.is_ok(), "{message}: {:?}", result.as_ref().err());
        result.unwrap_or_else(|_| unreachable!("the assertion above reports the unexpected error"))
    }

    let mapping = NativeMapping::from_parts(PhysicalSize::new(800, 480), 1.0)
        .unwrap_or_else(|| unreachable!("fixture native mapping is valid"));
    let mut runtime = AppRuntime::<DemoApp>::mount_with_config(
        DemoState::default(),
        RuntimeConfig::default().with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
    );
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the reference host focuses the editor at startup"));
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state().text, INITIAL_EDITOR_TEXT);
    let style_environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(&style_environment, mapping.logical_size())
        .with_raster_scale(mapping.raster_scale());
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();

    let selection_publication = runtime.publish_surface(&context);
    let selection_publication = expect_ok(selection_publication, "dragged selection republishes");
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    let movement = expect_ok(
        mouse.cursor_moved(device_id, next_point(second_end)),
        "second drag movement is translated",
    );
    expect_ok(
        runtime.submit_pointer(movement),
        "second drag movement is routed",
    );
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
#[allow(clippy::assert_is_empty)]
fn reference_editor_undo_and_redo_restore_application_owned_text_history() {
    let mut runtime = AppRuntime::<DemoApp>::mount_with_config(
        DemoState::default(),
        RuntimeConfig::default().with_text_font_source_policy(FontSourcePolicy::SystemAndBundled),
    );
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    let owner = runtime.index().nodes()[0].id().clone();
    let initial_selection = runtime.state().selection_seed;
    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::Undo,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("empty reference undo is accepted"));
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    runtime
        .submit_text(
            CommittedTextEvent::new("Q", None)
                .unwrap_or_else(|_| unreachable!("fixture text is valid")),
        )
        .unwrap_or_else(|_| unreachable!("reference editor accepts committed text"));
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::Undo,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("reference editor accepts undo"));
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state().text, INITIAL_EDITOR_TEXT);
    assert_eq!(runtime.state().selection_seed, initial_selection);

    let redo = runtime.submit_command(owner, SemanticCommand::Redo, CommandOrigin::programmatic());
    assert!(redo.is_ok(), "reference editor accepts redo: {redo:?}");
    let _ = runtime
        .pump(HOST_PUMP_BUDGET)
        .unwrap_or_else(|_| unreachable!("test pump observation"))
        .report()
        .to_owned();
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
    let context = SurfaceBuildContext::tight(&environment, mapping.logical_size())
        .with_raster_scale(mapping.raster_scale());
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
        let _ = runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("test pump observation"))
            .report()
            .to_owned();
        let owner = runtime.index().nodes()[0].id().clone();
        runtime
            .submit_command(
                owner.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("measurement editor accepts focus"));
        let _ = runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("test pump observation"))
            .report()
            .to_owned();
        if extra_lines > 0 {
            let text =
                "multiline responsiveness fixture — retained text layout\n".repeat(extra_lines);
            runtime
                .submit_text(
                    CommittedTextEvent::new(text, None)
                        .unwrap_or_else(|_| unreachable!("measurement text is valid")),
                )
                .unwrap_or_else(|_| unreachable!("measurement editor accepts long text"));
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
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
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
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
        let _ = drag_runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("test pump observation"))
            .report()
            .to_owned();
        let drag_owner = drag_runtime.index().nodes()[0].id().clone();
        drag_runtime
            .submit_command(
                drag_owner,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("drag measurement editor accepts focus"));
        let _ = drag_runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("test pump observation"))
            .report()
            .to_owned();
        if extra_lines > 0 {
            let text =
                "multiline responsiveness fixture — retained text layout\n".repeat(extra_lines);
            drag_runtime
                .submit_text(
                    CommittedTextEvent::new(text, None)
                        .unwrap_or_else(|_| unreachable!("measurement text is valid")),
                )
                .unwrap_or_else(|_| unreachable!("drag measurement text is accepted"));
            let _ = drag_runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
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
        let _ = drag_runtime
            .pump(HOST_PUMP_BUDGET)
            .unwrap_or_else(|_| unreachable!("test pump observation"))
            .report()
            .to_owned();
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
            let _ = drag_runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
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
    let context = SurfaceBuildContext::tight(&environment, mapping.logical_size())
        .with_raster_scale(mapping.raster_scale());
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
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
            let owner = runtime.index().nodes()[0].id().clone();
            runtime
                .submit_command(
                    owner,
                    SemanticCommand::RequestFocus,
                    CommandOrigin::programmatic(),
                )
                .unwrap_or_else(|_| unreachable!("paste fixture accepts focus"));
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
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
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
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
    let context = SurfaceBuildContext::tight(&environment, mapping.logical_size())
        .with_raster_scale(mapping.raster_scale());
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
                .unwrap_or_else(|_| unreachable!("controlled profile generic mapping is valid"));
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
            let owner = runtime.index().nodes()[0].id().clone();
            runtime
                .submit_command(
                    owner.clone(),
                    SemanticCommand::RequestFocus,
                    CommandOrigin::programmatic(),
                )
                .unwrap_or_else(|_| unreachable!("profile editor accepts focus"));
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
            let _ = publish_profile(&mut runtime, &context);

            runtime
                .submit_command(
                    owner.clone(),
                    SemanticCommand::SelectAll,
                    CommandOrigin::programmatic(),
                )
                .unwrap_or_else(|_| unreachable!("profile editor accepts select-all"));
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
            let mutation_started = Instant::now();
            runtime
                .submit_text(
                    CommittedTextEvent::new(text.clone(), None)
                        .unwrap_or_else(|_| unreachable!("profile replacement is valid")),
                )
                .unwrap_or_else(|_| unreachable!("profile replacement is admitted"));
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
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
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
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
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
            let mutation_started = Instant::now();
            runtime
                .submit_text(
                    CommittedTextEvent::new(replacement.clone(), None)
                        .unwrap_or_else(|_| unreachable!("second replacement is valid")),
                )
                .unwrap_or_else(|_| unreachable!("second replacement is admitted"));
            let _ = runtime
                .pump(HOST_PUMP_BUDGET)
                .unwrap_or_else(|_| unreachable!("test pump observation"))
                .report()
                .to_owned();
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

fn translated_point(displayed: &DisplayedFrame, mapping: NativeMapping) -> TranslatedPointerPoint {
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
    assert_eq!(mapping.physical_size(), PhysicalSize::new(1200, 800));
    assert!((mapping.logical_size().width() - 600.0).abs() < f32::EPSILON);
    assert!((mapping.logical_size().height() - 400.0).abs() < f32::EPSILON);
    assert!((mapping.raster_scale().get() - 2.0).abs() < f32::EPSILON);
    assert!((mapping.native_scale_factor() - 2.0).abs() < f64::EPSILON);
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
#[allow(clippy::assert_is_empty)]
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
#[allow(clippy::assert_is_empty)]
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
