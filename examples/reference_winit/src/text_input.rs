//! Reference-host native M10 integration proofs over the reusable public
//! `runenui_winit::text_input` translation edge. This file contains tests only.

#[cfg(test)]
mod tests {
    use runenui_winit::text_input::{
        TextInputState, keyboard_committed_text_candidate, translate_preedit_range,
    };
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
        let _ = runtime
            .pump(full_pump())
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned();
        let owner = runtime.index().nodes()[0].id().clone();
        runtime
            .submit_command(
                owner.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("the demo probe accepts focus"));
        let _ = runtime
            .pump(full_pump())
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned();
        let generation = runtime
            .start_composition(None)
            .unwrap_or_else(|_| unreachable!("the focused demo probe accepts composition"))
            .generation()
            .clone();
        (owner, generation)
    }

    fn distinct_owner() -> MountedNodeId {
        let mut runtime = AppRuntime::<DemoApp>::mount(DemoState::default());
        let _ = runtime
            .pump(full_pump())
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned();
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
