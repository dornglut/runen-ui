use runenui_core::{
    CompositionGeneration, CompositionRange, CompositionRangeError, KeyboardCompositionState,
    WidgetTextInput,
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
    pub const fn ime_allowed(&self) -> bool {
        wants_ime(
            self.window_focused,
            self.focused_owner.is_some(),
            self.capability,
        )
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
const fn wants_ime(
    window_focused: bool,
    has_focused_owner: bool,
    capability: WidgetTextInput,
) -> bool {
    window_focused
        && has_focused_owner
        && (capability.accepts_committed_text() || capability.accepts_composition())
}

#[must_use]
pub fn keyboard_committed_text_candidate(
    state: ElementState,
    synthetic: bool,
    accepts_committed_text: bool,
    composition: KeyboardCompositionState,
    text: Option<&str>,
) -> Option<&str> {
    if state != ElementState::Pressed
        || synthetic
        || !accepts_committed_text
        || composition == KeyboardCompositionState::Active
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
    use super::{
        TextInputState, keyboard_committed_text_candidate, translate_preedit_range, wants_ime,
    };
    use crate::DemoApp;
    use runenui_core::{
        CommandOrigin, CompositionGeneration, KeyboardCompositionState, SemanticCommand,
        WidgetTextInput,
    };
    use runenui_runtime::{AppRuntime, MountedNodeId, PumpBudget};
    use winit::event::ElementState;

    const fn full_pump() -> PumpBudget {
        PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
    }

    fn focused_composition() -> (MountedNodeId, CompositionGeneration) {
        let mut runtime = AppRuntime::<DemoApp>::mount(());
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
        let mut runtime = AppRuntime::<DemoApp>::mount(());
        runtime.pump(full_pump());
        runtime.index().nodes()[0].id().clone()
    }

    #[test]
    fn native_ime_allowance_follows_text_or_composition_capability() {
        assert!(!wants_ime(true, false, WidgetTextInput::new(true, true)));
        assert!(!wants_ime(false, true, WidgetTextInput::new(true, true)));
        assert!(wants_ime(true, true, WidgetTextInput::new(true, false)));
        assert!(wants_ime(true, true, WidgetTextInput::new(false, true)));
        assert!(!wants_ime(true, true, WidgetTextInput::NONE));
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
        assert!(state.ime_allowed());
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
        assert!(state.ime_allowed());
    }

    #[test]
    fn keyboard_text_owns_real_pressed_non_composing_commits_only() {
        assert_eq!(
            keyboard_committed_text_candidate(
                ElementState::Pressed,
                false,
                true,
                KeyboardCompositionState::Inactive,
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
                Some(""),
            ),
            None
        );
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
