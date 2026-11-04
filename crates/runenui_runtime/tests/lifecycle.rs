#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildBearingWidget, CommandOrigin, Element, NoHostProtocol, SemanticCommand, UiApp, View,
    Widget, WidgetMountContext, WidgetUnmountContext, children, container,
};
use runenui_runtime::{AppRuntime, PumpBudget, SubmitCommandErrorKind};

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
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

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

impl ChildBearingWidget<()> for LifetimeWidget {}

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
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The authored tree has at most 2 initial owners and no callback output.
        runenui_core::ProducerBudget::new(2, 0)
            .with_reconciliation(2, 0, 2)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(2, 0)
            .with_reconciliation(2, 2, 2)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0).with_reconciliation(2, 2, 2)
    }

    type State = AppState;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

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
    })
    .unwrap_or_else(|_| unreachable!("lifecycle fixture admission is valid"));
    let old_root = runtime.index().nodes()[0].id().clone();
    let old_child = runtime.index().nodes()[1].id().clone();
    assert_eq!(
        log.borrow().as_slice(),
        ["mount:old-parent", "mount:old-child"]
    );

    runtime
        .submit_action(Action::Replace)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime
            .pump(PumpBudget::new(3, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .processed_envelopes(),
        3
    );
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
    for stale in [old_root, old_child] {
        let Err(error) = runtime.submit_command(
            stale,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ) else {
            unreachable!("the replaced mounted lifetime is stale")
        };
        assert_eq!(error.kind(), SubmitCommandErrorKind::StaleTarget);
    }

    let _state = runtime.into_state();
    assert_eq!(
        log.borrow()
            .iter()
            .filter(|entry| entry.starts_with("unmount:new-"))
            .count(),
        2
    );
}
