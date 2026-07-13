use runenui_core::{Element, ElementId, View, Widget, children, column, row, text};
use runenui_runtime::{AppRuntime, ReconciliationDiagnostic, UiApp};

fn id_by_authored<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    authored: &str,
) -> runenui_runtime::MountedNodeId {
    let authored = ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .node_by_authored_id(&authored)
        .unwrap_or_else(|| unreachable!())
        .id()
        .clone()
}

#[derive(Clone, Copy, Debug)]
enum OrdinalAction {
    KeyedInsert,
    UnkeyedInsert,
}
#[derive(Debug)]
struct OrdinalState {
    case: u8,
}
struct OrdinalApp;
impl UiApp for OrdinalApp {
    type State = OrdinalState;
    type Action = OrdinalAction;
    fn root(state: &Self::State) -> Element<Self::Action> {
        let children: Vec<Element<Self::Action>> = match state.case {
            0 => vec![
                text("u1").id("u1").into_element(),
                text("u2").id("u2").into_element(),
            ],
            1 => vec![
                text("keyed").id("keyed").key("keyed").into_element(),
                text("u1").id("u1").into_element(),
                text("u2").id("u2").into_element(),
            ],
            _ => vec![
                text("new").id("new").into_element(),
                text("u1").id("u1").into_element(),
                text("u2").id("u2").into_element(),
            ],
        };
        column(children).key("root").into_element()
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        state.case = match action {
            OrdinalAction::KeyedInsert => 1,
            OrdinalAction::UnkeyedInsert => 2,
        };
    }
}

#[test]
fn keyed_insertion_does_not_shift_unkeyed_ordinals_but_unkeyed_insertion_does() {
    let mut runtime = AppRuntime::<OrdinalApp>::mount(OrdinalState { case: 0 });
    let u1 = id_by_authored(&mut runtime, "u1");
    let u2 = id_by_authored(&mut runtime, "u2");
    runtime
        .dispatch(OrdinalAction::KeyedInsert)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(id_by_authored(&mut runtime, "u1"), u1);
    assert_eq!(id_by_authored(&mut runtime, "u2"), u2);
    runtime
        .dispatch(OrdinalAction::UnkeyedInsert)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(id_by_authored(&mut runtime, "new"), u1);
    assert_eq!(id_by_authored(&mut runtime, "u1"), u2);
}

#[derive(Clone, Copy, Debug)]
enum MoveAction {
    Move,
}
#[derive(Debug)]
struct MoveState {
    moved: bool,
}
struct MoveApp;
impl UiApp for MoveApp {
    type State = MoveState;
    type Action = MoveAction;
    fn root(state: &Self::State) -> Element<Self::Action> {
        let child = || text("child").id("child").key("child");
        row(children![
            column(if state.moved {
                Vec::<Element<Self::Action>>::new()
            } else {
                vec![child().into_element()]
            })
            .id("left")
            .key("left"),
            column(if state.moved {
                vec![child().into_element()]
            } else {
                Vec::<Element<Self::Action>>::new()
            })
            .id("right")
            .key("right"),
        ])
        .key("root")
        .into_element()
    }
    fn update(state: &mut Self::State, MoveAction::Move: Self::Action) {
        state.moved = true;
    }
}

#[test]
fn cross_parent_keyed_move_remounts() {
    let mut runtime = AppRuntime::<MoveApp>::mount(MoveState { moved: false });
    let child = id_by_authored(&mut runtime, "child");
    runtime
        .dispatch(MoveAction::Move)
        .unwrap_or_else(|_| unreachable!());
    assert_ne!(id_by_authored(&mut runtime, "child"), child);
    assert_eq!(runtime.reconciliation_report().moved_count(), 0);
}

#[derive(Clone, Copy, Debug)]
enum DuplicateAction {
    Duplicate,
}
#[derive(Debug)]
struct DuplicateState {
    duplicate: bool,
}
struct DuplicateApp;
impl UiApp for DuplicateApp {
    type State = DuplicateState;
    type Action = DuplicateAction;
    fn root(state: &Self::State) -> Element<Self::Action> {
        let mut items = vec![text("one").id("one").key("same").into_element()];
        if state.duplicate {
            items.push(text("two").id("two").key("same").into_element());
        }
        column(items).key("root").into_element()
    }
    fn update(state: &mut Self::State, DuplicateAction::Duplicate: Self::Action) {
        state.duplicate = true;
    }
}

#[test]
fn duplicate_keys_reuse_no_ambiguous_lifetime() {
    let mut runtime = AppRuntime::<DuplicateApp>::mount(DuplicateState { duplicate: false });
    let old = id_by_authored(&mut runtime, "one");
    runtime
        .dispatch(DuplicateAction::Duplicate)
        .unwrap_or_else(|_| unreachable!());
    assert_ne!(id_by_authored(&mut runtime, "one"), old);
    assert_eq!(runtime.reconciliation_report().mounted_count(), 2);
    assert_eq!(runtime.reconciliation_report().unmounted_count(), 1);
    assert_eq!(
        runtime.reconciliation_report().diagnostics(),
        &[ReconciliationDiagnostic::DuplicateSiblingKey {
            key: runenui_core::ElementKey::new("same").unwrap_or_else(|_| unreachable!()),
            parent_path: "root".to_owned(),
            old_occurrence_paths: vec!["root/0".to_owned()],
            new_occurrence_paths: vec!["root/0".to_owned(), "root/1".to_owned()],
        }]
    );
}

#[derive(Clone, Copy, Debug)]
enum DuplicateTransition {
    KeepDuplicate,
    RemoveDuplicate,
    RemoveAll,
}

struct DuplicateTransitionApp;

impl UiApp for DuplicateTransitionApp {
    type State = u8;
    type Action = DuplicateTransition;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let count = match *state {
            2 => 1,
            3 => 0,
            _ => 2,
        };
        column(
            (0..count)
                .map(|position| text(format!("item-{position}")).key("same").into_element())
                .collect::<Vec<_>>(),
        )
        .key("root")
        .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        *state = match action {
            DuplicateTransition::KeepDuplicate => 1,
            DuplicateTransition::RemoveDuplicate => 2,
            DuplicateTransition::RemoveAll => 3,
        };
    }
}

#[test]
fn duplicate_diagnostics_include_complete_old_and_new_occurrences() {
    let key = runenui_core::ElementKey::new("same").unwrap_or_else(|_| unreachable!());
    let mut runtime = AppRuntime::<DuplicateTransitionApp>::mount(0);
    runtime
        .dispatch(DuplicateTransition::KeepDuplicate)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime.reconciliation_report().diagnostics(),
        &[ReconciliationDiagnostic::DuplicateSiblingKey {
            key: key.clone(),
            parent_path: "root".to_owned(),
            old_occurrence_paths: vec!["root/0".to_owned(), "root/1".to_owned()],
            new_occurrence_paths: vec!["root/0".to_owned(), "root/1".to_owned()],
        }]
    );
    runtime
        .dispatch(DuplicateTransition::RemoveDuplicate)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime.reconciliation_report().diagnostics(),
        &[ReconciliationDiagnostic::DuplicateSiblingKey {
            key,
            parent_path: "root".to_owned(),
            old_occurrence_paths: vec!["root/0".to_owned(), "root/1".to_owned()],
            new_occurrence_paths: vec!["root/0".to_owned()],
        }]
    );

    let mut disappearing = AppRuntime::<DuplicateTransitionApp>::mount(0);
    disappearing
        .dispatch(DuplicateTransition::RemoveAll)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        disappearing.reconciliation_report().diagnostics(),
        &[ReconciliationDiagnostic::DuplicateSiblingKey {
            key: runenui_core::ElementKey::new("same").unwrap_or_else(|_| unreachable!()),
            parent_path: "root".to_owned(),
            old_occurrence_paths: vec!["root/0".to_owned(), "root/1".to_owned()],
            new_occurrence_paths: Vec::new(),
        }]
    );
}

#[derive(Clone, Copy, Debug)]
enum NestedDuplicateAction {
    Duplicate,
}

struct NestedDuplicateApp;

impl UiApp for NestedDuplicateApp {
    type State = bool;
    type Action = NestedDuplicateAction;

    fn root(duplicate: &Self::State) -> Element<Self::Action> {
        let outer_count = usize::from(*duplicate) + 1;
        let nested_count = usize::from(*duplicate) + 1;
        column(
            (0..outer_count)
                .map(|position| {
                    text(format!("outer-{position}"))
                        .key("outer-duplicate")
                        .into_element()
                })
                .chain(core::iter::once(
                    column(
                        (0..nested_count)
                            .map(|position| {
                                text(format!("nested-{position}"))
                                    .key("nested-duplicate")
                                    .into_element()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .key("stable-group")
                    .into_element(),
                ))
                .collect::<Vec<_>>(),
        )
        .key("root")
        .into_element()
    }

    fn update(duplicate: &mut Self::State, NestedDuplicateAction::Duplicate: Self::Action) {
        *duplicate = true;
    }
}

#[test]
fn nested_duplicate_sets_report_complete_paths_in_deterministic_preorder() {
    let mut runtime = AppRuntime::<NestedDuplicateApp>::mount(false);
    runtime
        .dispatch(NestedDuplicateAction::Duplicate)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime.reconciliation_report().diagnostics(),
        &[
            ReconciliationDiagnostic::DuplicateSiblingKey {
                key: runenui_core::ElementKey::new("outer-duplicate")
                    .unwrap_or_else(|_| unreachable!()),
                parent_path: "root".to_owned(),
                old_occurrence_paths: vec!["root/0".to_owned()],
                new_occurrence_paths: vec!["root/0".to_owned(), "root/1".to_owned()],
            },
            ReconciliationDiagnostic::DuplicateSiblingKey {
                key: runenui_core::ElementKey::new("nested-duplicate")
                    .unwrap_or_else(|_| unreachable!()),
                parent_path: "root/2".to_owned(),
                old_occurrence_paths: vec!["root/2/0".to_owned()],
                new_occurrence_paths: vec!["root/2/0".to_owned(), "root/2/1".to_owned()],
            },
        ]
    );
}

#[derive(Clone, Copy, Debug)]
enum RootAction {
    Replace,
}
struct RootApp;
impl UiApp for RootApp {
    type State = bool;
    type Action = RootAction;
    fn root(replaced: &bool) -> Element<Self::Action> {
        text("root")
            .id(if *replaced { "after" } else { "before" })
            .key(if *replaced { "after" } else { "before" })
            .into_element()
    }
    fn update(replaced: &mut bool, RootAction::Replace: Self::Action) {
        *replaced = true;
    }
}

#[test]
fn root_key_change_replaces_while_authored_id_change_alone_does_not_define_identity() {
    let mut runtime = AppRuntime::<RootApp>::mount(false);
    let old = runtime.index().nodes()[0].id().clone();
    runtime
        .dispatch(RootAction::Replace)
        .unwrap_or_else(|_| unreachable!());
    assert_ne!(runtime.index().nodes()[0].id(), &old);
    assert_eq!(runtime.reconciliation_report().mounted_count(), 1);
    assert_eq!(runtime.reconciliation_report().unmounted_count(), 1);
}

#[derive(Clone, Copy, Debug)]
enum RenameAction {
    Rename,
}
struct RenameApp;
impl UiApp for RenameApp {
    type State = bool;
    type Action = RenameAction;
    fn root(renamed: &bool) -> Element<Self::Action> {
        text("root")
            .id(if *renamed { "renamed" } else { "original" })
            .key("stable")
            .into_element()
    }
    fn update(renamed: &mut bool, RenameAction::Rename: Self::Action) {
        *renamed = true;
    }
}

#[test]
fn authored_id_change_preserves_mounted_identity() {
    let mut runtime = AppRuntime::<RenameApp>::mount(false);
    let old = runtime.index().nodes()[0].id().clone();
    runtime
        .dispatch(RenameAction::Rename)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(runtime.index().nodes()[0].id(), &old);
    assert_eq!(
        runtime.index().nodes()[0]
            .authored_id()
            .map(ElementId::as_str),
        Some("renamed")
    );
}

#[derive(Debug)]
struct WidgetA;
#[derive(Debug)]
struct WidgetB;
impl Widget<()> for WidgetA {
    type State = ();
    fn create_state(&self) -> Self::State {}
}
impl Widget<()> for WidgetB {
    type State = ();
    fn create_state(&self) -> Self::State {}
}
#[derive(Clone, Copy, Debug)]
enum WidgetAction {
    Replace,
}
struct WidgetApp;
impl UiApp for WidgetApp {
    type State = bool;
    type Action = WidgetAction;
    fn root(replaced: &bool) -> Element<Self::Action> {
        let element = if *replaced {
            Element::new(WidgetB)
        } else {
            Element::new(WidgetA)
        };
        element.key("same").map_action(|()| WidgetAction::Replace)
    }
    fn update(replaced: &mut bool, WidgetAction::Replace: Self::Action) {
        *replaced = true;
    }
}

#[test]
fn same_key_with_incompatible_widget_remounts_even_when_state_type_matches() {
    let mut runtime = AppRuntime::<WidgetApp>::mount(false);
    let old = runtime.index().nodes()[0].id().clone();
    runtime
        .dispatch(WidgetAction::Replace)
        .unwrap_or_else(|_| unreachable!());
    assert_ne!(runtime.index().nodes()[0].id(), &old);
}
