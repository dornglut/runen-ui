//! Semantic link authoring. Navigation remains application/host-owned.

use core::fmt;

use crate::{
    Focusability, HitContribution, HitContributionContext, LogicalRect, SemanticAction,
    SemanticContribution, SemanticContributionContext, SemanticNodeContribution, SemanticRole,
    SemanticState, WidgetActivationContext, WidgetInvalidation, WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, View, common_node_builder_methods},
    widget_erasure::WidgetAdapter,
    widget_protocol::{
        Widget, WidgetActivation, WidgetActivationOutput, WidgetMeasure, WidgetMeasureInput,
    },
};

/// A semantic reference activated through the ordinary application action path.
///
/// A `Link` does not parse URLs, own navigation, open a browser, or maintain
/// visited history. The application decides what activation means.
pub struct Link<Action> {
    label: String,
    common: CommonNodeAuthoring,
    enabled: bool,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
}

impl<Action> fmt::Debug for Link<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Link")
            .field("label", &self.label)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("enabled", &self.enabled)
            .field("actionable", &self.activation_factory.is_some())
            .finish_non_exhaustive()
    }
}

impl<Action> Link<Action> {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            common: CommonNodeAuthoring::default(),
            enabled: true,
            activation_factory: None,
        }
    }

    common_node_builder_methods!();

    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Produces an ordinary application action; no URL/router policy is implied.
    #[must_use]
    pub fn on_activate(mut self, action: impl FnMut() -> Action + 'static) -> Self {
        self.activation_factory = Some(Box::new(action));
        self
    }
}

#[derive(Debug)]
struct LinkState {
    label: String,
    enabled: bool,
    actionable: bool,
}

struct LinkWidget<Action> {
    label: String,
    enabled: bool,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
}

impl<Action> fmt::Debug for LinkWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinkWidget")
            .field("label", &self.label)
            .field("enabled", &self.enabled)
            .field("actionable", &self.activation_factory.is_some())
            .finish()
    }
}

impl<Action> Widget<Action> for LinkWidget<Action> {
    type State = LinkState;

    fn create_state(&self) -> Self::State {
        LinkState {
            label: self.label.clone(),
            enabled: self.enabled,
            actionable: self.activation_factory.is_some(),
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.label != self.label {
            context.invalidate(
                WidgetInvalidation::LAYOUT
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        }
        if state.enabled != self.enabled {
            context.invalidate(
                WidgetInvalidation::INTERACTION
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        }
        if state.actionable != self.activation_factory.is_some() {
            context.invalidate(
                WidgetInvalidation::INTERACTION
                    | WidgetInvalidation::HIT_TEST
                    | WidgetInvalidation::SEMANTICS,
            );
        }
        state.label.clone_from(&self.label);
        state.enabled = self.enabled;
        state.actionable = self.activation_factory.is_some();
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        if self.activation_factory.is_some() {
            WidgetActivation::actionable(self.enabled)
        } else {
            WidgetActivation::NONE
        }
    }

    fn activate(
        &mut self,
        _: &mut Self::State,
        _: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        if !self.enabled {
            return WidgetActivationOutput::none();
        }
        self.activation_factory
            .as_mut()
            .map_or_else(WidgetActivationOutput::none, |factory| {
                WidgetActivationOutput::action(factory())
            })
    }

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text(crate::TextLeafMeasure::new(self.label.clone()))
    }

    fn hit_test(&self, state: &Self::State, context: HitContributionContext) -> HitContribution {
        if !state.actionable {
            return HitContribution::empty();
        }
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("measured local rectangle is valid")),
        )
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Link)
            .with_name(state.label.clone())
            .with_state(SemanticState::ENABLED.with_disabled(!state.enabled));
        if state.actionable {
            node = node.with_action(SemanticAction::Activate);
        }
        SemanticContribution::single(node)
    }
}

impl<Action: 'static> View<Action> for Link<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(LinkWidget {
                label: self.label,
                enabled: self.enabled,
                activation_factory: self.activation_factory,
            })),
            Vec::new(),
            diagnostics,
        )
    }
}

/// Builds a semantic Link without taking ownership of application navigation.
#[must_use]
pub fn link<Action>(label: impl Into<String>) -> Link<Action> {
    Link::new(label)
}
