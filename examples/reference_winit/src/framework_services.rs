use arboard::Clipboard;
use runenui_core::{
    ClipboardClassification, ClipboardText, CursorShape, DragDropPhase, FrameworkServiceFailure,
    FrameworkServiceRequest, FrameworkServiceResponse, WorkSequence,
};
use std::{collections::HashMap, path::PathBuf};
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    window::{CursorIcon, Window},
};

/// Host-private native service executor. Clipboard ownership never crosses into core/runtime.
pub struct NativeFrameworkServices {
    clipboard: Option<Clipboard>,
    last_public_clipboard_text: Option<String>,
    pending_drop_paths: HashMap<WorkSequence, Vec<PathBuf>>,
    admitted_drop_paths: HashMap<WorkSequence, Vec<PathBuf>>,
    native_window_focused: bool,
    requested_ime_allowed: bool,
}

impl NativeFrameworkServices {
    pub fn new() -> Self {
        Self {
            clipboard: None,
            last_public_clipboard_text: None,
            pending_drop_paths: HashMap::new(),
            admitted_drop_paths: HashMap::new(),
            native_window_focused: false,
            requested_ime_allowed: false,
        }
    }

    pub fn shutdown(&mut self) {
        // Some native clipboard backends serve their data from this process until it exits.
        self.clipboard = None;
        self.last_public_clipboard_text = None;
        self.pending_drop_paths.clear();
        self.admitted_drop_paths.clear();
        self.native_window_focused = false;
        self.requested_ime_allowed = false;
    }

    /// Applies the platform-focus safety gate to the last committed runtime IME request.
    pub fn set_native_window_focused(&mut self, window: &Window, focused: bool) {
        self.native_window_focused = focused;
        window.set_ime_allowed(self.effective_ime_allowed());
    }

    /// Resets native IME state when a host window is replaced or the host suspends.
    pub fn reset_native_window_ime(&mut self, window: Option<&Window>) {
        self.native_window_focused = false;
        self.requested_ime_allowed = false;
        if let Some(window) = window {
            window.set_ime_allowed(false);
        }
    }

    const fn effective_ime_allowed(&self) -> bool {
        self.native_window_focused && self.requested_ime_allowed
    }

    /// Stages native file custody under a runtime-assigned source sequence.
    pub fn stage_drop_paths(
        &mut self,
        source: WorkSequence,
        paths: Vec<PathBuf>,
    ) -> Result<(), FrameworkServiceFailure> {
        if paths.is_empty()
            || self.pending_drop_paths.contains_key(&source)
            || self.admitted_drop_paths.contains_key(&source)
        {
            return Err(FrameworkServiceFailure::Rejected);
        }
        self.pending_drop_paths.insert(source, paths);
        Ok(())
    }

    /// Discards host-owned paths when runtime admission did not complete.
    pub fn discard_pending_drop_paths(&mut self, source: WorkSequence) {
        self.pending_drop_paths.remove(&source);
    }

    /// Discards an admitted payload if its checked runtime completion became stale.
    pub fn discard_admitted_drop_paths(&mut self, source: WorkSequence) {
        self.admitted_drop_paths.remove(&source);
    }

    /// Commits host custody only after the runtime accepted the exact successful response.
    pub fn commit_drop_path_admission(&mut self, source: WorkSequence) -> bool {
        if self.admitted_drop_paths.contains_key(&source) {
            return false;
        }
        let Some(paths) = self.pending_drop_paths.remove(&source) else {
            return false;
        };
        self.admitted_drop_paths.insert(source, paths);
        true
    }

    pub fn take_admitted_drop_batches(&mut self) -> Vec<(WorkSequence, Vec<PathBuf>)> {
        self.admitted_drop_paths.drain().collect()
    }

    pub const fn response_outcome(response: &FrameworkServiceResponse) -> &'static str {
        match response {
            FrameworkServiceResponse::ClipboardReadText(Ok(_))
            | FrameworkServiceResponse::ClipboardWriteText(Ok(()))
            | FrameworkServiceResponse::InputMethod(Ok(()))
            | FrameworkServiceResponse::Cursor(Ok(()))
            | FrameworkServiceResponse::DragDrop(Ok(())) => "succeeded",
            FrameworkServiceResponse::ClipboardReadText(Err(_))
            | FrameworkServiceResponse::ClipboardWriteText(Err(_))
            | FrameworkServiceResponse::InputMethod(Err(_))
            | FrameworkServiceResponse::Cursor(Err(_))
            | FrameworkServiceResponse::DragDrop(Err(_)) => "failed",
            _ => "unknown",
        }
    }

    pub fn execute(
        &mut self,
        window: Option<&Window>,
        request: &FrameworkServiceRequest,
    ) -> FrameworkServiceResponse {
        match request {
            FrameworkServiceRequest::ClipboardReadText { max_bytes } => {
                self.read_clipboard_text(*max_bytes)
            }
            FrameworkServiceRequest::ClipboardWriteText { text, .. } => {
                // Runtime only issues copy/cut writes for public editable text. Remember
                // the exact successful payload so a later read can distinguish our own
                // public copy from unknown native clipboard contents.
                self.last_public_clipboard_text = None;
                let clipboard = match self.clipboard() {
                    Ok(clipboard) => clipboard,
                    Err(failure) => {
                        return FrameworkServiceResponse::ClipboardWriteText(Err(failure));
                    }
                };
                let result = clipboard
                    .set_text(text.as_ref())
                    .map_err(|error| map_clipboard_error(&error));
                if result.is_ok() {
                    self.last_public_clipboard_text = Some(text.to_string());
                }
                FrameworkServiceResponse::ClipboardWriteText(result)
            }
            FrameworkServiceRequest::InputMethod {
                enabled,
                candidate_area,
                ..
            } => {
                let Some(window) = window else {
                    return FrameworkServiceResponse::InputMethod(Err(
                        FrameworkServiceFailure::Unavailable,
                    ));
                };
                if *enabled {
                    let Some(area) = candidate_area else {
                        return FrameworkServiceResponse::InputMethod(Err(
                            FrameworkServiceFailure::Rejected,
                        ));
                    };
                    let origin = area.origin();
                    let size = area.size();
                    window.set_ime_cursor_area(
                        LogicalPosition::new(origin.x(), origin.y()),
                        LogicalSize::new(size.width(), size.height()),
                    );
                }
                self.requested_ime_allowed = *enabled;
                window.set_ime_allowed(self.effective_ime_allowed());
                FrameworkServiceResponse::InputMethod(Ok(()))
            }
            FrameworkServiceRequest::Cursor { shape, visible } => {
                let Some(window) = window else {
                    return FrameworkServiceResponse::Cursor(Err(
                        FrameworkServiceFailure::Unavailable,
                    ));
                };
                window.set_cursor_visible(*visible);
                window.set_cursor(cursor_icon(*shape));
                FrameworkServiceResponse::Cursor(Ok(()))
            }
            FrameworkServiceRequest::DragDrop {
                source,
                phase,
                payload,
                accepted,
            } => self.execute_drag_drop(*source, *phase, payload.kind(), *accepted),
            _ => FrameworkServiceResponse::Cursor(Err(FrameworkServiceFailure::Rejected)),
        }
    }

    fn execute_drag_drop(
        &mut self,
        source: WorkSequence,
        phase: DragDropPhase,
        payload: runenui_core::DragDropPayloadKind,
        accepted: bool,
    ) -> FrameworkServiceResponse {
        match phase {
            DragDropPhase::Hover => FrameworkServiceResponse::DragDrop(Ok(())),
            DragDropPhase::Cancel => {
                self.pending_drop_paths.remove(&source);
                self.admitted_drop_paths.remove(&source);
                FrameworkServiceResponse::DragDrop(Ok(()))
            }
            DragDropPhase::Drop if !accepted => {
                self.pending_drop_paths.remove(&source);
                FrameworkServiceResponse::DragDrop(Ok(()))
            }
            DragDropPhase::Drop if payload == runenui_core::DragDropPayloadKind::Files => {
                if !self.pending_drop_paths.contains_key(&source) {
                    return FrameworkServiceResponse::DragDrop(Err(
                        FrameworkServiceFailure::Unavailable,
                    ));
                }
                FrameworkServiceResponse::DragDrop(Ok(()))
            }
            DragDropPhase::Drop => {
                FrameworkServiceResponse::DragDrop(Err(FrameworkServiceFailure::Rejected))
            }
            _ => FrameworkServiceResponse::DragDrop(Err(FrameworkServiceFailure::Rejected)),
        }
    }

    fn read_clipboard_text(&mut self, max_bytes: usize) -> FrameworkServiceResponse {
        let clipboard = match self.clipboard() {
            Ok(clipboard) => clipboard,
            Err(failure) => return FrameworkServiceResponse::ClipboardReadText(Err(failure)),
        };
        match clipboard.get_text() {
            Ok(text) if text.len() <= max_bytes => {
                let classification = self.classify_native_clipboard_text(&text);
                FrameworkServiceResponse::ClipboardReadText(Ok(native_clipboard_text(
                    text,
                    classification,
                )))
            }
            Ok(_) => {
                FrameworkServiceResponse::ClipboardReadText(Err(FrameworkServiceFailure::Rejected))
            }
            Err(error) => {
                FrameworkServiceResponse::ClipboardReadText(Err(map_clipboard_error(&error)))
            }
        }
    }

    fn classify_native_clipboard_text(&mut self, text: &str) -> ClipboardClassification {
        if self.last_public_clipboard_text.as_deref() == Some(text) {
            ClipboardClassification::Public
        } else {
            // A mismatch means another native source replaced the clipboard. Drop
            // the remembered text so stale app provenance cannot be reused later.
            self.last_public_clipboard_text = None;
            ClipboardClassification::Sensitive
        }
    }

    fn clipboard(&mut self) -> Result<&mut Clipboard, FrameworkServiceFailure> {
        if self.clipboard.is_none() {
            self.clipboard = Some(Clipboard::new().map_err(|error| map_clipboard_error(&error))?);
        }
        self.clipboard
            .as_mut()
            .ok_or(FrameworkServiceFailure::Unavailable)
    }
}

fn native_clipboard_text(text: String, classification: ClipboardClassification) -> ClipboardText {
    // Native clipboard formats do not carry RunenUI confidentiality labels. Treat
    // unknown provenance as sensitive: secret destinations may accept it, while
    // public destinations remain fail-closed.
    ClipboardText::new(text, classification)
}

const fn map_clipboard_error(error: &arboard::Error) -> FrameworkServiceFailure {
    match error {
        arboard::Error::ContentNotAvailable | arboard::Error::ClipboardNotSupported => {
            FrameworkServiceFailure::Unavailable
        }
        arboard::Error::ConversionFailure => FrameworkServiceFailure::Rejected,
        _ => FrameworkServiceFailure::Failed,
    }
}

const fn cursor_icon(shape: CursorShape) -> CursorIcon {
    match shape {
        CursorShape::Text => CursorIcon::Text,
        CursorShape::Pointer => CursorIcon::Pointer,
        CursorShape::Crosshair => CursorIcon::Crosshair,
        CursorShape::Move => CursorIcon::Move,
        CursorShape::NotAllowed => CursorIcon::NotAllowed,
        CursorShape::Wait => CursorIcon::Wait,
        CursorShape::Progress => CursorIcon::Progress,
        CursorShape::ResizeHorizontal => CursorIcon::EwResize,
        CursorShape::ResizeVertical => CursorIcon::NsResize,
        CursorShape::ResizeDiagonalNorthwestSoutheast => CursorIcon::NwseResize,
        CursorShape::ResizeDiagonalNortheastSouthwest => CursorIcon::NeswResize,
        CursorShape::Grab => CursorIcon::Grab,
        CursorShape::Grabbing => CursorIcon::Grabbing,
        _ => CursorIcon::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeFrameworkServices, cursor_icon, map_clipboard_error, native_clipboard_text};
    use std::{
        num::{NonZeroU32, NonZeroU64},
        path::PathBuf,
    };

    use runenui_core::{
        ClipboardClassification, CursorShape, DragDropPayloadKind, DragDropPayloadMetadata,
        DragDropPhase, FrameworkServiceFailure, FrameworkServiceRequest, FrameworkServiceResponse,
        WorkSequence,
    };
    use winit::window::CursorIcon;

    #[test]
    fn native_clipboard_error_mapping_is_typed_and_does_not_collapse_to_empty_text() {
        assert_eq!(
            map_clipboard_error(&arboard::Error::ContentNotAvailable),
            FrameworkServiceFailure::Unavailable
        );
        assert_eq!(
            map_clipboard_error(&arboard::Error::ClipboardOccupied),
            FrameworkServiceFailure::Failed
        );
        assert_eq!(
            map_clipboard_error(&arboard::Error::ConversionFailure),
            FrameworkServiceFailure::Rejected
        );
        assert_eq!(
            map_clipboard_error(&arboard::Error::Unknown {
                description: "private native detail".to_owned(),
            }),
            FrameworkServiceFailure::Failed
        );
    }

    #[test]
    fn native_clipboard_text_uses_the_conservative_sensitive_classification() {
        let text = native_clipboard_text(
            "clipboard contents".to_owned(),
            ClipboardClassification::Sensitive,
        );

        assert_eq!(text.text(), "clipboard contents");
        assert_eq!(text.classification(), ClipboardClassification::Sensitive);
        assert!(!format!("{text:?}").contains("clipboard contents"));
    }

    #[test]
    fn only_exact_app_written_public_clipboard_text_is_public_on_read() {
        let mut services = NativeFrameworkServices::new();
        services.last_public_clipboard_text = Some("copied public text".to_owned());

        assert_eq!(
            services.classify_native_clipboard_text("copied public text"),
            ClipboardClassification::Public
        );
        assert_eq!(
            services.classify_native_clipboard_text("external clipboard text"),
            ClipboardClassification::Sensitive
        );
        assert_eq!(services.last_public_clipboard_text, None);
    }

    #[test]
    fn cursor_translation_stays_inside_the_selected_winit_host() {
        assert_eq!(cursor_icon(CursorShape::Text), CursorIcon::Text);
        assert_eq!(cursor_icon(CursorShape::Pointer), CursorIcon::Pointer);
        assert_eq!(
            cursor_icon(CursorShape::ResizeDiagonalNorthwestSoutheast),
            CursorIcon::NwseResize
        );
    }

    #[test]
    fn unavailable_window_is_a_typed_native_service_failure() {
        let mut services = NativeFrameworkServices::new();
        let response = services.execute(
            None,
            &runenui_core::FrameworkServiceRequest::Cursor {
                shape: CursorShape::Text,
                visible: true,
            },
        );
        assert_eq!(
            response,
            runenui_core::FrameworkServiceResponse::Cursor(Err(
                FrameworkServiceFailure::Unavailable
            ))
        );
        assert_eq!(
            NativeFrameworkServices::response_outcome(&response),
            "failed"
        );
    }

    #[test]
    fn native_ime_focus_is_only_a_gate_over_committed_runtime_request() {
        let mut services = NativeFrameworkServices::new();
        services.requested_ime_allowed = true;
        assert!(!services.effective_ime_allowed());
        services.native_window_focused = true;
        assert!(services.effective_ime_allowed());
        services.reset_native_window_ime(None);
        assert!(!services.effective_ime_allowed());
    }

    #[test]
    fn native_drop_custody_moves_only_after_exact_runtime_admission() {
        let mut services = NativeFrameworkServices::new();
        let source = WorkSequence::__runtime_new(
            NonZeroU64::new(41).unwrap_or_else(|| unreachable!("non-zero test sequence")),
        );
        let path = PathBuf::from("/private/native/notes.txt");
        services
            .stage_drop_paths(source, vec![path.clone()])
            .unwrap_or_else(|_| unreachable!("first exact source may be staged"));
        let request = FrameworkServiceRequest::DragDrop {
            source,
            phase: DragDropPhase::Drop,
            payload: DragDropPayloadMetadata::new(
                DragDropPayloadKind::Files,
                NonZeroU32::new(1).unwrap_or_else(|| unreachable!("non-zero item count")),
                None,
            ),
            accepted: true,
        };

        let response = services.execute(None, &request);

        assert_eq!(response, FrameworkServiceResponse::DragDrop(Ok(())));
        assert!(!format!("{request:?}").contains("notes.txt"));
        assert!(services.take_admitted_drop_batches().is_empty());
        assert!(services.commit_drop_path_admission(source));
        assert!(!services.commit_drop_path_admission(source));
        assert_eq!(
            services.take_admitted_drop_batches(),
            vec![(source, vec![path])]
        );
        assert!(services.take_admitted_drop_batches().is_empty());
    }

    #[test]
    fn native_drop_rejection_cancellation_and_shutdown_release_host_paths() {
        let mut services = NativeFrameworkServices::new();
        let first = WorkSequence::__runtime_new(
            NonZeroU64::new(51).unwrap_or_else(|| unreachable!("non-zero test sequence")),
        );
        let second = WorkSequence::__runtime_new(
            NonZeroU64::new(52).unwrap_or_else(|| unreachable!("non-zero test sequence")),
        );
        services
            .stage_drop_paths(first, vec![PathBuf::from("/private/rejected.txt")])
            .unwrap_or_else(|_| unreachable!("first path can be staged"));
        let declined = FrameworkServiceRequest::DragDrop {
            source: first,
            phase: DragDropPhase::Drop,
            payload: DragDropPayloadMetadata::new(
                DragDropPayloadKind::Files,
                NonZeroU32::MIN,
                None,
            ),
            accepted: false,
        };
        assert_eq!(
            services.execute(None, &declined),
            FrameworkServiceResponse::DragDrop(Ok(()))
        );
        assert!(services.take_admitted_drop_batches().is_empty());

        services
            .stage_drop_paths(second, vec![PathBuf::from("/private/cancelled.txt")])
            .unwrap_or_else(|_| unreachable!("second path can be staged"));
        let cancelled = FrameworkServiceRequest::DragDrop {
            source: second,
            phase: DragDropPhase::Cancel,
            payload: DragDropPayloadMetadata::new(
                DragDropPayloadKind::Files,
                NonZeroU32::MIN,
                None,
            ),
            accepted: false,
        };
        assert_eq!(
            services.execute(None, &cancelled),
            FrameworkServiceResponse::DragDrop(Ok(()))
        );
        assert!(services.take_admitted_drop_batches().is_empty());

        services
            .stage_drop_paths(second, vec![PathBuf::from("/private/shutdown.txt")])
            .unwrap_or_else(|_| unreachable!("second source was released by cancellation"));
        services.shutdown();
        assert!(services.take_admitted_drop_batches().is_empty());
        assert!(services.stage_drop_paths(first, Vec::new()).is_err());
    }

    #[test]
    fn accepted_native_file_drop_without_exact_host_payload_fails_closed() {
        let mut services = NativeFrameworkServices::new();
        let source = WorkSequence::__runtime_new(
            NonZeroU64::new(61).unwrap_or_else(|| unreachable!("non-zero test sequence")),
        );
        let request = FrameworkServiceRequest::DragDrop {
            source,
            phase: DragDropPhase::Drop,
            payload: DragDropPayloadMetadata::new(
                DragDropPayloadKind::Files,
                NonZeroU32::MIN,
                None,
            ),
            accepted: true,
        };
        assert_eq!(
            services.execute(None, &request),
            FrameworkServiceResponse::DragDrop(Err(FrameworkServiceFailure::Unavailable))
        );
        assert!(!format!("{request:?}").contains("private"));
    }
}
