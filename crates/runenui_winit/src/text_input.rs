//! Reusable winit-origin text-input/IME translation and focus-capability gating.
//!
//! Hosts retain the native IME lifecycle, runtime pumping, and document state.
//! This adapter observes typed current runtime focus/capability and translates
//! native candidate facts; it does not create a second editing authority.

use runenui_core::{
    CompositionGeneration, CompositionRange, CompositionRangeError, KeyboardCompositionState,
    LogicalKey, WidgetTextInput,
};
use runenui_runtime::MountedNodeId;
use winit::event::ElementState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeTextSync {
    reset_native_ime: bool,
}

impl RuntimeTextSync {
    #[must_use]
    pub const fn reset_native_ime(self) -> bool {
        self.reset_native_ime
    }
}

pub struct TextInputState {
    focused_owner: Option<MountedNodeId>,
    capability: WidgetTextInput,
    composition_generation: Option<CompositionGeneration>,
    window_focused: bool,
}

impl Default for TextInputState {
    fn default() -> Self {
        Self {
            focused_owner: None,
            capability: WidgetTextInput::NONE,
            composition_generation: None,
            window_focused: false,
        }
    }
}

impl TextInputState {
    pub fn sync_runtime(
        &mut self,
        focused_owner: Option<MountedNodeId>,
        capability: WidgetTextInput,
    ) -> RuntimeTextSync {
        let focus_changed = self.focused_owner.as_ref() != focused_owner.as_ref();
        let reset_native_ime = self.composition_generation.is_some()
            && (focus_changed || !capability.accepts_composition());
        if reset_native_ime {
            self.composition_generation = None;
        }
        self.focused_owner = focused_owner;
        self.capability = capability;
        RuntimeTextSync { reset_native_ime }
    }

    pub const fn set_window_focused(&mut self, focused: bool) {
        self.window_focused = focused;
    }

    #[must_use]
    pub const fn accepts_committed_text(&self) -> bool {
        self.window_focused
            && self.focused_owner.is_some()
            && self.capability.accepts_committed_text()
    }

    #[must_use]
    pub const fn accepts_composition(&self) -> bool {
        self.window_focused && self.focused_owner.is_some() && self.capability.accepts_composition()
    }

    #[must_use]
    pub const fn keyboard_composition_state(&self) -> KeyboardCompositionState {
        if self.composition_generation.is_some() {
            KeyboardCompositionState::Active
        } else {
            KeyboardCompositionState::Inactive
        }
    }

    #[must_use]
    pub const fn composition_generation(&self) -> Option<&CompositionGeneration> {
        self.composition_generation.as_ref()
    }

    pub fn remember_composition_generation(&mut self, generation: CompositionGeneration) {
        debug_assert!(self.composition_generation.is_none());
        self.composition_generation = Some(generation);
    }

    pub fn retire_composition(&mut self) {
        self.composition_generation = None;
    }
}

#[must_use]
pub fn keyboard_committed_text_candidate<'a>(
    state: ElementState,
    synthetic: bool,
    accepts_committed_text: bool,
    composition: KeyboardCompositionState,
    logical_key: &LogicalKey,
    text: Option<&'a str>,
) -> Option<&'a str> {
    if state != ElementState::Pressed
        || synthetic
        || !accepts_committed_text
        || composition == KeyboardCompositionState::Active
        || !matches!(
            logical_key,
            LogicalKey::Character(_) | LogicalKey::Enter | LogicalKey::Space
        )
    {
        return None;
    }
    text.filter(|text| !text.is_empty())
}

/// Converts an optional native preedit range into checked UTF-8 byte offsets.
///
/// # Errors
///
/// Returns [`CompositionRangeError`] if the native range is reversed, out of
/// bounds, or not aligned to Unicode scalar boundaries in `preedit`.
pub fn translate_preedit_range(
    preedit: &str,
    range: Option<(usize, usize)>,
) -> Result<Option<CompositionRange>, CompositionRangeError> {
    range
        .map(|(start, end)| CompositionRange::new(preedit, start, end))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::{TextInputState, keyboard_committed_text_candidate, translate_preedit_range};
    use crate::DemoApp;
    use runenui_core::{KeyboardCompositionState, LogicalKey, WidgetTextInput};
    use runenui_runtime::{AppRuntime, PumpBudget};
    use winit::event::ElementState;

    #[test]
    fn public_adapter_focus_and_capability_are_exactly_gated() {
        let mut runtime = AppRuntime::<DemoApp>::mount(());
        runtime
            .pump(PumpBudget::new(16, 16, 16, 16))
            .unwrap_or_else(|_| unreachable!("standalone public test mounts"));
        let id = runtime.index().nodes()[0].id().clone();
        let mut state = TextInputState::default();
        state.sync_runtime(Some(id), WidgetTextInput::new(true, true));
        assert!(!state.accepts_committed_text());
        state.set_window_focused(true);
        assert!(state.accepts_committed_text());
        assert!(state.accepts_composition());
        state.sync_runtime(None, WidgetTextInput::NONE);
        assert!(!state.accepts_committed_text());
        assert!(!state.accepts_composition());
    }

    #[test]
    fn public_adapter_rejects_untrusted_native_text_or_invalid_preedit() {
        assert_eq!(
            keyboard_committed_text_candidate(
                ElementState::Released,
                false,
                true,
                KeyboardCompositionState::Inactive,
                &LogicalKey::Character(String::from("a")),
                Some("a"),
            ),
            None
        );
        assert!(translate_preedit_range("aß", Some((2, 3))).is_err());
    }
}
