use super::{
    AppRuntime, FocusTargetResult, Key, KeyPhase, KeyboardEvent, KeyboardFocusResult,
    MountedNodeId, MountedTreeIndex, PointerButton, PointerEvent, PointerFocusResult, PointerPhase,
    RuntimeStatus, TargetStatus, UiApp,
};

impl<App: UiApp> AppRuntime<App> {
    #[must_use]
    pub fn index(&mut self) -> MountedTreeIndex<'_, App::Action> {
        self.runtime.tree.index()
    }

    pub fn set_focus(&mut self, id: MountedNodeId) -> FocusTargetResult {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return FocusTargetResult::NotFocusable;
        }
        match self.runtime.tree.target_status(&id) {
            TargetStatus::Foreign => FocusTargetResult::ForeignRuntime,
            TargetStatus::Stale | TargetStatus::Missing => FocusTargetResult::StaleTarget,
            TargetStatus::Live => {
                let activation = self.runtime.tree.activation(&id);
                if activation
                    .is_ok_and(|activation| activation.enabled() && activation.is_actionable())
                {
                    self.runtime.set_focus(id);
                    FocusTargetResult::Focused
                } else {
                    FocusTargetResult::NotFocusable
                }
            }
        }
    }

    pub fn focus_first(&mut self) -> Option<MountedNodeId> {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return None;
        }
        let id = self.index().first_focusable_node().map(|n| n.id().clone());
        self.apply_focus_result(id)
    }
    pub fn focus_last(&mut self) -> Option<MountedNodeId> {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return None;
        }
        let id = self.index().last_focusable_node().map(|n| n.id().clone());
        self.apply_focus_result(id)
    }
    pub fn focus_next(&mut self) -> Option<MountedNodeId> {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return None;
        }
        let current = self.focus().focused_node().cloned();
        let id = {
            let index = self.index();
            current.as_ref().map_or_else(
                || index.first_focusable_node().map(|n| n.id().clone()),
                |current| {
                    index
                        .next_focusable_after(current)
                        .or_else(|| index.first_focusable_node())
                        .map(|n| n.id().clone())
                },
            )
        };
        self.apply_focus_result(id)
    }
    pub fn focus_previous(&mut self) -> Option<MountedNodeId> {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return None;
        }
        let current = self.focus().focused_node().cloned();
        let id = {
            let index = self.index();
            current.as_ref().map_or_else(
                || index.last_focusable_node().map(|n| n.id().clone()),
                |current| {
                    index
                        .previous_focusable_before(current)
                        .or_else(|| index.last_focusable_node())
                        .map(|n| n.id().clone())
                },
            )
        };
        self.apply_focus_result(id)
    }
    fn apply_focus_result(&mut self, id: Option<MountedNodeId>) -> Option<MountedNodeId> {
        if let Some(id) = id {
            self.runtime.set_focus(id.clone());
            Some(id)
        } else {
            self.runtime.clear_focus();
            None
        }
    }
    pub fn clear_focus(&mut self) {
        if matches!(self.status(), RuntimeStatus::Running) {
            self.runtime.clear_focus();
        }
    }
    pub fn handle_keyboard_focus(&mut self, event: &KeyboardEvent) -> KeyboardFocusResult {
        if event.phase() != KeyPhase::Pressed || !matches!(event.key(), Key::Tab) {
            return KeyboardFocusResult::Ignored;
        }
        let id = if event.modifiers().shift() {
            self.focus_previous()
        } else {
            self.focus_next()
        };
        id.map_or(
            KeyboardFocusResult::NoFocusableNode,
            KeyboardFocusResult::Moved,
        )
    }
    pub fn handle_pointer_focus(&mut self, event: &PointerEvent) -> PointerFocusResult {
        if event.phase() != PointerPhase::Pressed || event.button() != Some(PointerButton::Primary)
        {
            return PointerFocusResult::Ignored;
        }
        let Some(id) = event.target().cloned() else {
            return PointerFocusResult::NoTarget;
        };
        match self.set_focus(id.clone()) {
            FocusTargetResult::Focused => PointerFocusResult::Moved(id),
            FocusTargetResult::NotFocusable => PointerFocusResult::NotFocusable,
            FocusTargetResult::StaleTarget | FocusTargetResult::ForeignRuntime => {
                PointerFocusResult::NotFound
            }
        }
    }
}
