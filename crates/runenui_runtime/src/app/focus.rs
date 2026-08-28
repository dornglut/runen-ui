use runenui_core::WidgetTextInput;

use super::{AppRuntime, MountedTreeIndex, UiApp};

impl<App: UiApp> AppRuntime<App> {
    #[must_use]
    pub fn index(&mut self) -> MountedTreeIndex<'_, App::Action> {
        self.runtime.tree.index()
    }

    /// Returns the focused mounted owner's current host-neutral text-input capability.
    ///
    /// Missing focus and an invalid capability bridge both fail closed to
    /// [`WidgetTextInput::NONE`]. The focused mounted identity remains runtime-owned.
    #[must_use]
    pub fn focused_text_input_capability(&mut self) -> WidgetTextInput {
        let focused = self.runtime.focus().focused_node().cloned();
        let Some(focused) = focused else {
            return WidgetTextInput::NONE;
        };
        self.runtime
            .tree
            .text_input_probe(&focused)
            .unwrap_or(WidgetTextInput::NONE)
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        CommandOrigin, Element, IntoEffects, NoHostProtocol, SemanticCommand, UiApp, View, Widget,
        WidgetActivation, WidgetTextInput,
    };

    use crate::{AppRuntime, PumpBudget};

    #[derive(Debug)]
    struct TextProbe {
        capability: WidgetTextInput,
    }

    impl Widget<()> for TextProbe {
        type State = ();

        fn create_state(&self) -> Self::State {}

        fn activation(&self, (): &Self::State) -> WidgetActivation {
            WidgetActivation::actionable(true)
        }

        fn text_input(&self, (): &Self::State) -> WidgetTextInput {
            self.capability
        }
    }

    struct App;

    impl UiApp for App {
        type State = WidgetTextInput;
        type Action = ();
        type HostProtocol = NoHostProtocol;

        fn root(capability: &Self::State) -> impl View<Self::Action> {
            Element::new(TextProbe {
                capability: *capability,
            })
            .id("text")
            .focusable(true)
        }

        fn update(
            _: &mut Self::State,
            (): Self::Action,
        ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        }
    }

    const fn full_pump() -> PumpBudget {
        PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
    }

    fn project_after_focus(capability: WidgetTextInput) -> WidgetTextInput {
        let mut runtime = AppRuntime::<App>::mount(capability);
        runtime.pump(full_pump());
        assert_eq!(
            runtime.focused_text_input_capability(),
            WidgetTextInput::NONE,
            "unfocused mounted text capability is not host-visible"
        );

        let target = runtime.index().nodes()[0].id().clone();
        runtime
            .submit_command(
                target,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("the live text probe accepts focus"));
        runtime.pump(full_pump());
        runtime.focused_text_input_capability()
    }

    #[test]
    fn focused_text_input_capability_preserves_independent_text_and_composition_bits() {
        let committed_only = WidgetTextInput::new(true, false);
        assert_eq!(project_after_focus(committed_only), committed_only);

        let composition_only = WidgetTextInput::new(false, true);
        assert_eq!(project_after_focus(composition_only), composition_only);
    }
}
