//! Visual layout application tests.

use super::apply_visual_layout_operation;
use crate::{
    AuthoredUiNodePath, AuthoredUiTemplate, UiAxisDefinition, UiNodeDefinition,
    UiScrollInputDefinition, UiScrollOwnershipDefinition, UiTemplateId, UiValueBinding,
    UiVisualLayoutActivationMode, UiVisualLayoutDiffChangeKind, UiVisualLayoutEditContext,
    UiVisualLayoutEditKind, UiVisualLayoutOperation,
};

#[test]
fn visual_layout_move_preserves_stable_ids() {
    let template = layout_template();
    let operation = operation(
        "move.b",
        "root/b",
        "b",
        UiVisualLayoutEditKind::MoveNode {
            new_parent_path: path("root"),
            new_index: 0,
        },
    );

    let report = apply_visual_layout_operation(
        template,
        &operation,
        UiVisualLayoutActivationMode::Activate,
        &context(),
    );

    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let UiNodeDefinition::Column { children, .. } = report.template.root else {
        panic!("root should stay a column");
    };
    let ids: Vec<_> = children
        .iter()
        .map(|node| node.id().as_str().to_string())
        .collect();
    assert_eq!(ids, ["b", "a", "c"]);
    assert_eq!(
        report.diff.unwrap().changes[0].kind,
        UiVisualLayoutDiffChangeKind::Move
    );
}

#[test]
fn visual_layout_reorder_preserves_stable_ids() {
    let template = layout_template();
    let operation = operation(
        "reorder.c",
        "root",
        "root",
        UiVisualLayoutEditKind::ReorderSibling {
            from_index: 2,
            to_index: 0,
            expected_child_id: "c".into(),
        },
    );

    let report = apply_visual_layout_operation(
        template,
        &operation,
        UiVisualLayoutActivationMode::Activate,
        &context(),
    );

    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let UiNodeDefinition::Column { children, .. } = report.template.root else {
        panic!("root should stay a column");
    };
    let ids: Vec<_> = children
        .iter()
        .map(|node| node.id().as_str().to_string())
        .collect();
    assert_eq!(ids, ["c", "a", "b"]);
}

#[test]
fn visual_layout_diff_text_is_deterministic() {
    let first = apply_visual_layout_operation(
        stack_template(),
        &operation(
            "axis.stack",
            "root/stack",
            "stack",
            UiVisualLayoutEditKind::ChangeStackAxis {
                axis: UiAxisDefinition::Horizontal,
            },
        ),
        UiVisualLayoutActivationMode::Activate,
        &context(),
    );
    let second = apply_visual_layout_operation(
        stack_template(),
        &operation(
            "axis.stack",
            "root/stack",
            "stack",
            UiVisualLayoutEditKind::ChangeStackAxis {
                axis: UiAxisDefinition::Horizontal,
            },
        ),
        UiVisualLayoutActivationMode::Activate,
        &context(),
    );

    assert!(!first.has_errors(), "{:?}", first.diagnostics);
    assert_eq!(first.diff, second.diff);
    let change = &first.diff.unwrap().changes[0];
    assert_eq!(change.before.as_deref(), Some("Vertical"));
    assert_eq!(change.after.as_deref(), Some("Horizontal"));
}

#[test]
fn visual_layout_preview_only_edit_rejects_activation() {
    let mut operation = operation(
        "preview.axis",
        "root/stack",
        "stack",
        UiVisualLayoutEditKind::ChangeStackAxis {
            axis: UiAxisDefinition::Horizontal,
        },
    );
    operation.preview_only = true;

    let report = apply_visual_layout_operation(
        stack_template(),
        &operation,
        UiVisualLayoutActivationMode::Activate,
        &context(),
    );

    assert!(report.has_errors());
    assert_eq!(
        report.diagnostics[0].code,
        "ui.visual_layout.preview_only_activation"
    );
    assert!(report.diff.is_none());
}

#[test]
fn visual_layout_invalid_edit_reports_source_mapped_diagnostic() {
    let operation = operation(
        "bad.axis",
        "root/a",
        "a",
        UiVisualLayoutEditKind::ChangeStackAxis {
            axis: UiAxisDefinition::Horizontal,
        },
    );

    let report = apply_visual_layout_operation(
        layout_template(),
        &operation,
        UiVisualLayoutActivationMode::Activate,
        &context(),
    );

    let diagnostic = report
        .diagnostics
        .first()
        .expect("invalid layout edit should produce diagnostic");
    assert_eq!(
        diagnostic.code,
        "ui.visual_layout.layout_feature.unsupported"
    );
    assert_eq!(
        diagnostic.path.as_ref().map(AuthoredUiNodePath::as_str),
        Some("root/a")
    );
    assert_eq!(diagnostic.target_profile.as_str(), "editor.workbench");
    assert_eq!(
        diagnostic.activation_impact,
        crate::UiVisualLayoutActivationImpact::BlocksActivation
    );
    assert!(!diagnostic.suggested_fix.is_empty());
}

#[test]
fn visual_layout_target_profile_compatibility_is_fail_closed() {
    let operation = operation(
        "profile.axis",
        "root/stack",
        "stack",
        UiVisualLayoutEditKind::ChangeStackAxis {
            axis: UiAxisDefinition::Horizontal,
        },
    );
    let edit_context =
        UiVisualLayoutEditContext::with_supported_target_profiles(["game.runtime".into()]);

    let report = apply_visual_layout_operation(
        stack_template(),
        &operation,
        UiVisualLayoutActivationMode::Activate,
        &edit_context,
    );

    assert!(report.has_errors());
    assert_eq!(
        report.diagnostics[0].code,
        "ui.visual_layout.target_profile.unsupported"
    );
    assert_eq!(
        report.diagnostics[0].target_profile.as_str(),
        "editor.workbench"
    );
}

#[test]
fn game_runtime_visual_layout_edits_are_profile_gated() {
    let mut operation = operation(
        "game.split",
        "root/stack",
        "stack",
        UiVisualLayoutEditKind::ChangeStackAxis {
            axis: UiAxisDefinition::Horizontal,
        },
    );
    operation.target_profile = "game.runtime".into();
    let edit_context =
        UiVisualLayoutEditContext::with_supported_target_profiles(["game.runtime".into()]);

    let report = apply_visual_layout_operation(
        stack_template(),
        &operation,
        UiVisualLayoutActivationMode::Activate,
        &edit_context,
    );

    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert_eq!(
        report
            .diff
            .expect("successful game runtime edit has diff")
            .target_profile
            .as_str(),
        "game.runtime"
    );
}

#[test]
fn visual_layout_insert_rejects_duplicate_ids_inside_new_subtree() {
    let operation = operation(
        "insert.duplicate.subtree",
        "root",
        "root",
        UiVisualLayoutEditKind::InsertNode {
            index: 0,
            node: UiNodeDefinition::Column {
                id: "new-root".into(),
                children: vec![label("dup"), label("dup")],
            },
        },
    );

    let report = apply_visual_layout_operation(
        layout_template(),
        &operation,
        UiVisualLayoutActivationMode::Activate,
        &context(),
    );

    assert!(report.has_errors());
    assert_eq!(
        report.diagnostics[0].code,
        "ui.visual_layout.authored_id.duplicate"
    );
}

fn context() -> UiVisualLayoutEditContext {
    UiVisualLayoutEditContext::with_supported_target_profiles(["editor.workbench".into()])
}

fn operation(
    id: &str,
    target_path: &str,
    expected_node_id: &str,
    kind: UiVisualLayoutEditKind,
) -> UiVisualLayoutOperation {
    UiVisualLayoutOperation {
        id: id.into(),
        source_document: UiTemplateId::from("test.template"),
        target_path: path(target_path),
        expected_node_id: expected_node_id.into(),
        target_profile: "editor.workbench".into(),
        kind,
        source_location: None,
        preview_only: false,
    }
}

fn layout_template() -> AuthoredUiTemplate {
    AuthoredUiTemplate {
        id: "test.template".into(),
        root: UiNodeDefinition::Column {
            id: "root".into(),
            children: vec![label("a"), label("b"), label("c")],
        },
        templates: Vec::new(),
        menus: Vec::new(),
    }
}

fn stack_template() -> AuthoredUiTemplate {
    AuthoredUiTemplate {
        id: "test.template".into(),
        root: UiNodeDefinition::Column {
            id: "root".into(),
            children: vec![UiNodeDefinition::Stack {
                id: "stack".into(),
                axis: UiAxisDefinition::Vertical,
                children: vec![label("a")],
            }],
        },
        templates: Vec::new(),
        menus: Vec::new(),
    }
}

fn label(id: &str) -> UiNodeDefinition {
    UiNodeDefinition::Label {
        id: id.into(),
        label: UiValueBinding::static_text(id),
        availability: None,
    }
}

#[allow(dead_code)]
fn scroll(id: &str) -> UiNodeDefinition {
    UiNodeDefinition::Scroll {
        id: id.into(),
        axis: crate::UiScrollAxisDefinition::Vertical,
        input: UiScrollInputDefinition::default(),
        ownership: UiScrollOwnershipDefinition::default(),
        children: Vec::new(),
    }
}

fn path(value: &str) -> AuthoredUiNodePath {
    AuthoredUiNodePath(value.to_string())
}
