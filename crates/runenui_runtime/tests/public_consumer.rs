#![allow(refining_impl_trait)]

use runenui_core::prelude::*;
use runenui_runtime::prelude::*;

#[derive(Clone, Copy)]
enum Action {
    Press,
}

struct App;
impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Initial mount queues one reconciliation envelope for the button.
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The public button route and activation each have one output allowance.
        runenui_core::ProducerBudget::new(2, 2)
    }

    type State = ();
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Action> {
        button("Press")
            .id("press")
            .on_activate(|| Action::Press)
            .into_element()
    }

    fn update((): &mut (), action: Action) {
        match action {
            Action::Press => {}
        }
    }
}

#[test]
fn ordinary_core_and_runtime_preludes_compile_together() {
    let mut runtime = AppRuntime::<App>::mount(())
        .unwrap_or_else(|_| unreachable!("public consumer fixture admission is valid"));
    let size = LogicalSize::try_new(100.0, 40.0).unwrap_or_else(|_| unreachable!());
    let style_environment = runenui_core::StyleEnvironment::default();
    let _context = SurfaceBuildContext::tight(&style_environment, size);
    assert_eq!(
        runtime.index().nodes()[0]
            .authored_id()
            .map(ElementId::as_str),
        Some("press")
    );
}
