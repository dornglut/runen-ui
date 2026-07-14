use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildLayout, ChildLayoutWidget, Element, View, Widget, WidgetMountContext,
    WidgetUnmountContext, children, container,
};
use runenui_runtime::{AppRuntime, PumpBudget, UiApp};

#[derive(Debug)]
struct LifetimeState {
    name: &'static str,
    log: Rc<RefCell<Vec<String>>>,
}

impl Drop for LifetimeState {
    fn drop(&mut self) {
        self.log.borrow_mut().push(format!("drop:{}", self.name));
    }
}

#[derive(Debug)]
struct LifetimeWidget {
    name: &'static str,
    log: Rc<RefCell<Vec<String>>>,
}

impl Widget<()> for LifetimeWidget {
    type State = LifetimeState;

    fn create_state(&self) -> Self::State {
        LifetimeState {
            name: self.name,
            log: Rc::clone(&self.log),
        }
    }

    fn mount(&self, _: &mut Self::State, _: &mut WidgetMountContext) {
        self.log.borrow_mut().push(format!("mount:{}", self.name));
    }

    fn unmount(&self, _: &mut Self::State, _: &mut WidgetUnmountContext) {
        self.log.borrow_mut().push(format!("unmount:{}", self.name));
    }
}

impl ChildLayoutWidget<()> for LifetimeWidget {
    fn child_layout(&self, _: &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: runenui_core::Axis::Vertical,
        }
    }
}

#[derive(Debug)]
struct AppState {
    replacement: bool,
    log: Rc<RefCell<Vec<String>>>,
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Replace,
}

struct App;

impl UiApp for App {
    type State = AppState;
    type Action = Action;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let prefix = if state.replacement { "new" } else { "old" };
        container(
            LifetimeWidget {
                name: if state.replacement {
                    "new-parent"
                } else {
                    "old-parent"
                },
                log: Rc::clone(&state.log),
            },
            children![Element::new(LifetimeWidget {
                name: if state.replacement {
                    "new-child"
                } else {
                    "old-child"
                },
                log: Rc::clone(&state.log),
            })],
        )
        .key(format!("{prefix}-root"))
        .into_element()
        .map_action(|()| Action::Replace)
    }

    fn update(state: &mut Self::State, Action::Replace: Self::Action) {
        state.replacement = true;
    }
}

#[test]
fn replacement_unmounts_live_postorder_then_drops_before_new_preorder_mount() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(AppState {
        replacement: false,
        log: Rc::clone(&log),
    });
    let old_root = runtime.index().nodes()[0].id().clone();
    let old_child = runtime.index().nodes()[1].id().clone();
    assert_eq!(
        log.borrow().as_slice(),
        ["mount:old-parent", "mount:old-child"]
    );

    runtime
        .submit_action(Action::Replace)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(runtime.pump(PumpBudget::new(1)).processed_envelopes(), 1);
    assert_eq!(
        log.borrow().as_slice(),
        [
            "mount:old-parent",
            "mount:old-child",
            "unmount:old-child",
            "drop:old-child",
            "unmount:old-parent",
            "drop:old-parent",
            "mount:new-parent",
            "mount:new-child",
        ]
    );
    assert_ne!(runtime.index().nodes()[0].id(), &old_root);
    assert_ne!(runtime.index().nodes()[1].id(), &old_child);
    assert_eq!(
        runtime.activate_node(&old_root),
        runenui_runtime::ActivationResult::StaleTarget
    );
    assert_eq!(
        runtime.activate_node(&old_child),
        runenui_runtime::ActivationResult::StaleTarget
    );

    let _state = runtime.into_state();
    assert_eq!(
        log.borrow()
            .iter()
            .filter(|entry| entry.starts_with("unmount:new-"))
            .count(),
        2
    );
}
