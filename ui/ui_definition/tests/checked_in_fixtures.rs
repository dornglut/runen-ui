use ui_definition::{AuthoredUiTemplate, normalize_authored_template};

const UI_FIXTURES: &[&str] = &[
    include_str!("../../../../assets/editor/ui/toolbar.ron"),
    include_str!("../../../../assets/editor/ui/shell_chrome.ron"),
    include_str!("../../../../assets/editor/ui/surfaces/inspector.ron"),
    include_str!("../../../../assets/editor/ui/surfaces/outliner.ron"),
    include_str!("../../../../assets/editor/ui/surfaces/entity_table.ron"),
    include_str!("../../../../assets/editor/ui/surfaces/console.ron"),
    include_str!("../../../../assets/editor/ui/surfaces/viewport.ron"),
];

#[test]
fn checked_in_ui_fixtures_parse_validate_and_normalize() {
    for source in UI_FIXTURES {
        let template: AuthoredUiTemplate =
            ron::from_str(source).expect("checked-in UI fixture should parse");
        let normalized = normalize_authored_template(template);
        assert!(
            !normalized.has_errors(),
            "fixture should normalize without errors: {:?}",
            normalized.diagnostics
        );
    }
}
