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
    use crate::{DemoApp, DemoState};
    use runenui_core::{
        CommandOrigin, CompositionGeneration, KeyboardCompositionState, LogicalKey,
        SemanticCommand, WidgetTextInput,
    };
    use runenui_runtime::{AppRuntime, MountedNodeId, PumpBudget};
    use winit::event::ElementState;

    const fn full_pump() -> PumpBudget {
        PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
    }

    fn focused_composition() -> (MountedNodeId, CompositionGeneration) {
        let mut runtime = AppRuntime::<DemoApp>::mount(DemoState::default());
        runtime.pump(full_pump());
        let owner = runtime.index().nodes()[0].id().clone();
        runtime
            .submit_command(
                owner.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("the demo probe accepts focus"));
        runtime.pump(full_pump());
        let generation = runtime
            .start_composition(None)
            .unwrap_or_else(|_| unreachable!("the focused demo probe accepts composition"))
            .generation()
            .clone();
        (owner, generation)
    }

    fn distinct_owner() -> MountedNodeId {
        let mut runtime = AppRuntime::<DemoApp>::mount(DemoState::default());
        runtime.pump(full_pump());
        runtime.index().nodes()[0].id().clone()
    }

    #[test]
    fn native_text_ingress_gates_follow_focus_and_committed_capability() {
        let mut state = TextInputState::default();
        state.sync_runtime(Some(distinct_owner()), WidgetTextInput::new(true, true));
        assert!(!state.accepts_committed_text());
        assert!(!state.accepts_composition());
        state.set_window_focused(true);
        assert!(state.accepts_committed_text());
        assert!(state.accepts_composition());
        state.sync_runtime(None, WidgetTextInput::new(true, true));
        assert!(!state.accepts_committed_text());
        assert!(!state.accepts_composition());
    }

    #[test]
    fn runtime_focus_transfer_retires_host_generation_and_requests_native_reset() {
        let (owner, generation) = focused_composition();
        let next_owner = distinct_owner();
        let mut state = TextInputState::default();
        state.set_window_focused(true);
        let _ = state.sync_runtime(Some(owner), WidgetTextInput::new(true, true));
        state.remember_composition_generation(generation);
        assert_eq!(
            state.keyboard_composition_state(),
            KeyboardCompositionState::Active
        );

        let sync = state.sync_runtime(Some(next_owner), WidgetTextInput::new(true, true));

        assert!(sync.reset_native_ime());
        assert_eq!(
            state.keyboard_composition_state(),
            KeyboardCompositionState::Inactive
        );
        assert!(state.accepts_composition());
    }

    #[test]
    fn composition_capability_loss_retires_host_generation_and_requests_native_reset() {
        let (owner, generation) = focused_composition();
        let mut state = TextInputState::default();
        state.set_window_focused(true);
        let _ = state.sync_runtime(Some(owner.clone()), WidgetTextInput::new(true, true));
        state.remember_composition_generation(generation);

        let sync = state.sync_runtime(Some(owner), WidgetTextInput::new(true, false));

        assert!(sync.reset_native_ime());
        assert_eq!(
            state.keyboard_composition_state(),
            KeyboardCompositionState::Inactive
        );
        assert!(state.accepts_committed_text());
    }

    #[test]
    fn keyboard_text_owns_real_pressed_non_composing_commits_only() {
        assert_eq!(
            keyboard_committed_text_candidate(
                ElementState::Pressed,
                false,
                true,
                KeyboardCompositionState::Inactive,
                &LogicalKey::Character(String::from("s")),
                Some("ß"),
            ),
            Some("ß")
        );
        assert_eq!(
            keyboard_committed_text_candidate(
                ElementState::Released,
                false,
                true,
                KeyboardCompositionState::Inactive,
                &LogicalKey::Character(String::from("s")),
                Some("ß"),
            ),
            None
        );
        assert_eq!(
            keyboard_committed_text_candidate(
                ElementState::Pressed,
                true,
                true,
                KeyboardCompositionState::Inactive,
                &LogicalKey::Character(String::from("s")),
                Some("ß"),
            ),
            None
        );
        assert_eq!(
            keyboard_committed_text_candidate(
                ElementState::Pressed,
                false,
                true,
                KeyboardCompositionState::Active,
                &LogicalKey::Character(String::from("s")),
                Some("ß"),
            ),
            None
        );
        assert_eq!(
            keyboard_committed_text_candidate(
                ElementState::Pressed,
                false,
                false,
                KeyboardCompositionState::Inactive,
                &LogicalKey::Character(String::from("s")),
                Some("ß"),
            ),
            None
        );
        assert_eq!(
            keyboard_committed_text_candidate(
                ElementState::Pressed,
                false,
                true,
                KeyboardCompositionState::Inactive,
                &LogicalKey::Character(String::from("s")),
                Some(""),
            ),
            None
        );
        for logical_key in [LogicalKey::Backspace, LogicalKey::Delete] {
            assert_eq!(
                keyboard_committed_text_candidate(
                    ElementState::Pressed,
                    false,
                    true,
                    KeyboardCompositionState::Inactive,
                    &logical_key,
                    Some("\u{8}"),
                ),
                None
            );
        }
        assert_eq!(
            keyboard_committed_text_candidate(
                ElementState::Pressed,
                false,
                true,
                KeyboardCompositionState::Inactive,
                &LogicalKey::Command(SemanticCommand::SelectAll),
                Some("a"),
            ),
            None
        );
    }

    #[test]
    fn only_text_keys_can_become_committed_native_text() {
        for (logical_key, native_text) in [
            (LogicalKey::Escape, "\u{1b}"),
            (LogicalKey::ArrowLeft, "Left"),
        ] {
            assert_eq!(
                keyboard_committed_text_candidate(
                    ElementState::Pressed,
                    false,
                    true,
                    KeyboardCompositionState::Inactive,
                    &logical_key,
                    Some(native_text),
                ),
                None
            );
        }
        for (logical_key, native_text) in [(LogicalKey::Space, " "), (LogicalKey::Enter, "\r")] {
            assert_eq!(
                keyboard_committed_text_candidate(
                    ElementState::Pressed,
                    false,
                    true,
                    KeyboardCompositionState::Inactive,
                    &logical_key,
                    Some(native_text),
                ),
                Some(native_text)
            );
        }
    }

    #[test]
    fn native_preedit_range_is_checked_as_utf8_bytes() {
        let preedit = "aßz";
        let range = translate_preedit_range(preedit, Some((1, 3)))
            .unwrap_or_else(|_| unreachable!("fixture range is on scalar boundaries"))
            .unwrap_or_else(|| unreachable!("fixture range is present"));
        assert_eq!(range.start(), 1);
        assert_eq!(range.end(), 3);
        assert!(translate_preedit_range(preedit, Some((2, 3))).is_err());
        assert!(translate_preedit_range(preedit, Some((3, 2))).is_err());
        assert!(translate_preedit_range(preedit, Some((0, 8))).is_err());
    }
}
