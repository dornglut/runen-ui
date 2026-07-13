use runenui_core::prelude::*;
use runenui_runtime::prelude::*;

#[derive(Clone, Copy)]
enum Action {
    Press,
}

struct App;
impl UiApp for App {
    type State = ();
    type Action = Action;

    fn root((): &()) -> Element<Action> {
        button("Press")
            .id("press")
            .on_press(Action::Press)
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
    let mut runtime = AppRuntime::<App>::mount(());
    let size = LogicalSize::try_new(100.0, 40.0).unwrap_or_else(|_| unreachable!());
    let tokens = runenui_core::StyleTokens::new();
    let _context = SurfaceBuildContext::tight(&tokens, size);
    assert_eq!(
        runtime.index().nodes()[0]
            .authored_id()
            .map(ElementId::as_str),
        Some("press")
    );
}
