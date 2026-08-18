use crate::SemanticAction;

fn assert_exact_m5_action(action: &SemanticAction) {
    match action {
        SemanticAction::Activate
        | SemanticAction::RequestFocus
        | SemanticAction::OpenMenu
        | SemanticAction::OpenContextMenu => {}
    }
}

#[test]
fn semantic_action_vocabulary_remains_exact_m5() {
    for action in [
        SemanticAction::Activate,
        SemanticAction::RequestFocus,
        SemanticAction::OpenMenu,
        SemanticAction::OpenContextMenu,
    ] {
        assert_exact_m5_action(&action);
    }
}
