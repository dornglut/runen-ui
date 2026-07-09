//! App-neutral semantic actions for adaptive UI interaction.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticInputSource {
    Pointer,
    Keyboard,
    Touch,
    Controller,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticDirection {
    Previous,
    Next,
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticResizeAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UiSemanticAction {
    Focus(SemanticDirection),
    Activate,
    Cancel,
    CycleTab(SemanticDirection),
    EnterMoveMode,
    EnterResizeMode(SemanticResizeAxis),
    Commit,
    Rollback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticActionEvent {
    pub source: SemanticInputSource,
    pub action: UiSemanticAction,
    pub repeated: bool,
}

impl SemanticActionEvent {
    pub const fn new(source: SemanticInputSource, action: UiSemanticAction) -> Self {
        Self {
            source,
            action,
            repeated: false,
        }
    }

    pub const fn repeated(mut self, repeated: bool) -> Self {
        self.repeated = repeated;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_sources_share_the_same_action_vocabulary() {
        for source in [
            SemanticInputSource::Pointer,
            SemanticInputSource::Keyboard,
            SemanticInputSource::Touch,
            SemanticInputSource::Controller,
        ] {
            assert_eq!(
                SemanticActionEvent::new(source, UiSemanticAction::Cancel).action,
                UiSemanticAction::Cancel
            );
        }
    }

    #[test]
    fn semantic_actions_cover_navigation_move_resize_commit_and_rollback() {
        let actions = [
            UiSemanticAction::Focus(SemanticDirection::Left),
            UiSemanticAction::Activate,
            UiSemanticAction::CycleTab(SemanticDirection::Next),
            UiSemanticAction::EnterMoveMode,
            UiSemanticAction::EnterResizeMode(SemanticResizeAxis::Horizontal),
            UiSemanticAction::Commit,
            UiSemanticAction::Cancel,
            UiSemanticAction::Rollback,
        ];
        assert_eq!(actions.len(), 8);
    }
}
