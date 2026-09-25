use core::fmt;

use crate::{
    FlexContainerStyle, FlexDirection, HitContribution, HitContributionContext, LayoutContainer,
    LayoutStyle, LogicalLength, LogicalRect, LogicalSize, SemanticAction, SemanticCheckedState, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticRole, SemanticState,
    SemanticText, WidgetActivationContext, WidgetInvalidation, WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, View, Views, common_node_builder_methods},
    widget_erasure::{ErasedWidget, WidgetAdapter},
    widget_protocol::{
        ChildBearingWidget, Widget, WidgetActivation, WidgetActivationOutput, WidgetMeasure,
        WidgetMeasureInput,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    content: String,
    common: CommonNodeAuthoring,
}

impl Text {
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            common: CommonNodeAuthoring::default(),
        }
    }
    common_node_builder_methods!();
    #[must_use]
    pub const fn content(&self) -> &str {
        self.content.as_str()
    }
}

#[derive(Debug)]
struct TextWidget {
    content: String,
}

impl<Action> Widget<Action> for TextWidget {
    type State = String;
    fn create_state(&self) -> Self::State {
        self.content.clone()
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.content {
            context.invalidate(
                WidgetInvalidation::LAYOUT
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
            state.clone_from(&self.content);
        }
    }
    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.content.clone(),
        }
    }
    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Text)
                .with_name(state.clone())
                .with_text(SemanticText::plain(state.clone())),
        )
    }
}

impl<Action: 'static> View<Action> for Text {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(crate::Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(TextWidget {
                content: self.content,
            })),
            Vec::new(),
            diagnostics,
        )
    }
}

pub struct Button<Action> {
    label: String,
    common: CommonNodeAuthoring,
    enabled: bool,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
    actionable: bool,
}

impl<Action> fmt::Debug for Button<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Button")
            .field("label", &self.label)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("layout", &self.common.layout)
            .field("enabled", &self.enabled)
            .field("actionable", &self.actionable)
            .field("has_callback", &self.activation_factory.is_some())
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("diagnostics", &self.common.diagnostics)
            .finish()
    }
}

impl<Action> Button<Action> {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            common: CommonNodeAuthoring::default(),
            enabled: true,
            activation_factory: None,
            actionable: false,
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
    #[must_use]
    pub fn on_activate(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.activation_factory = Some(Box::new(callback));
        self.actionable = true;
        self
    }
}

struct ButtonWidget<Action> {
    label: String,
    enabled: bool,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
    actionable: bool,
}

#[derive(Debug)]
struct ButtonWidgetState {
    label: String,
    enabled: bool,
    actionable: bool,
    activation_count: u64,
}

impl<Action> fmt::Debug for ButtonWidget<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ButtonWidget")
            .field("label", &self.label)
            .field("enabled", &self.enabled)
            .field("actionable", &self.actionable)
            .field("has_callback", &self.activation_factory.is_some())
            .finish()
    }
}

impl<Action> Widget<Action> for ButtonWidget<Action> {
    type State = ButtonWidgetState;
    fn create_state(&self) -> Self::State {
        ButtonWidgetState {
            label: self.label.clone(),
            enabled: self.enabled,
            actionable: self.actionable,
            activation_count: 0,
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
        if state.actionable != self.actionable {
            context.invalidate(
                WidgetInvalidation::INTERACTION
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS
                    | WidgetInvalidation::HIT_TEST,
            );
        }
        state.label.clone_from(&self.label);
        state.enabled = self.enabled;
        state.actionable = self.actionable;
    }
    fn activation(&self, _: &Self::State) -> WidgetActivation {
        if self.actionable {
            WidgetActivation::actionable(self.enabled)
        } else {
            WidgetActivation::NONE
        }
    }
    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        if self.enabled {
            state.activation_count = state.activation_count.saturating_add(1);
            context.invalidate(WidgetInvalidation::PAINT);
            self.activation_factory
                .as_mut()
                .map_or_else(WidgetActivationOutput::changed, |factory| {
                    WidgetActivationOutput::changed_with_action(factory())
                })
        } else {
            WidgetActivationOutput::none()
        }
    }
    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.label.clone(),
        }
    }
    fn hit_test(&self, state: &Self::State, context: HitContributionContext) -> HitContribution {
        if state.actionable {
            HitContribution::single_rect(local_rect(context.local_size()))
        } else {
            HitContribution::empty()
        }
    }
    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Button)
            .with_name(state.label.clone())
            .with_state(SemanticState::ENABLED.with_disabled(!state.enabled));
        if state.actionable {
            node = node.with_action(SemanticAction::Activate);
        }
        SemanticContribution::single(node)
    }
}

impl<Action: 'static> View<Action> for Button<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(crate::Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(ButtonWidget {
                label: self.label,
                enabled: self.enabled,
                activation_factory: self.activation_factory,
                actionable: self.actionable,
            })),
            Vec::new(),
            diagnostics,
        )
    }
}


pub struct Checkbox<Action> {
    label: String,
    checked: SemanticCheckedState,
    common: CommonNodeAuthoring,
    enabled: bool,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
    actionable: bool,
}

impl<Action> fmt::Debug for Checkbox<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Checkbox")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("layout", &self.common.layout)
            .field("enabled", &self.enabled)
            .field("actionable", &self.actionable)
            .field("has_callback", &self.activation_factory.is_some())
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("diagnostics", &self.common.diagnostics)
            .finish()
    }
}

impl<Action> Checkbox<Action> {
    #[must_use]
    pub fn new(label: impl Into<String>, checked: impl Into<SemanticCheckedState>) -> Self {
        Self {
            label: label.into(),
            checked: checked.into(),
            common: CommonNodeAuthoring::default(),
            enabled: true,
            activation_factory: None,
            actionable: false,
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
    #[must_use]
    pub fn on_activate(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.activation_factory = Some(Box::new(callback));
        self.actionable = true;
        self
    }
}

pub struct Switch<Action> {
    label: String,
    checked: bool,
    common: CommonNodeAuthoring,
    enabled: bool,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
    actionable: bool,
}

impl<Action> fmt::Debug for Switch<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Switch")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("layout", &self.common.layout)
            .field("enabled", &self.enabled)
            .field("actionable", &self.actionable)
            .field("has_callback", &self.activation_factory.is_some())
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("diagnostics", &self.common.diagnostics)
            .finish()
    }
}

impl<Action> Switch<Action> {
    #[must_use]
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        Self {
            label: label.into(),
            checked,
            common: CommonNodeAuthoring::default(),
            enabled: true,
            activation_factory: None,
            actionable: false,
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
    #[must_use]
    pub fn on_activate(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.activation_factory = Some(Box::new(callback));
        self.actionable = true;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BinaryControlKind {
    Checkbox,
    Switch,
}

impl BinaryControlKind {
    const fn role(self) -> SemanticRole {
        match self {
            Self::Checkbox => SemanticRole::Checkbox,
            Self::Switch => SemanticRole::Switch,
        }
    }
}

struct BinaryControlWidget<Action> {
    kind: BinaryControlKind,
    label: String,
    checked: SemanticCheckedState,
    enabled: bool,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
    actionable: bool,
}

#[derive(Debug)]
struct BinaryControlWidgetState {
    kind: BinaryControlKind,
    label: String,
    checked: SemanticCheckedState,
    enabled: bool,
    actionable: bool,
}

impl<Action> fmt::Debug for BinaryControlWidget<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BinaryControlWidget")
            .field("kind", &self.kind)
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("enabled", &self.enabled)
            .field("actionable", &self.actionable)
            .field("has_callback", &self.activation_factory.is_some())
            .finish()
    }
}

impl<Action> Widget<Action> for BinaryControlWidget<Action> {
    type State = BinaryControlWidgetState;

    fn create_state(&self) -> Self::State {
        BinaryControlWidgetState {
            kind: self.kind,
            label: self.label.clone(),
            checked: self.checked,
            enabled: self.enabled,
            actionable: self.actionable,
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.kind != self.kind {
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
        if state.label != self.label {
            context.invalidate(
                WidgetInvalidation::LAYOUT
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        }
        if state.checked != self.checked {
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
        if state.enabled != self.enabled {
            context.invalidate(
                WidgetInvalidation::INTERACTION
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        }
        if state.actionable != self.actionable {
            context.invalidate(
                WidgetInvalidation::INTERACTION
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS
                    | WidgetInvalidation::HIT_TEST,
            );
        }
        state.kind = self.kind;
        state.label.clone_from(&self.label);
        state.checked = self.checked;
        state.enabled = self.enabled;
        state.actionable = self.actionable;
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        if self.actionable {
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
        WidgetMeasure::Text {
            content: self.label.clone(),
        }
    }

    fn hit_test(&self, state: &Self::State, context: HitContributionContext) -> HitContribution {
        if state.actionable {
            HitContribution::single_rect(local_rect(context.local_size()))
        } else {
            HitContribution::empty()
        }
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(state.kind.role())
            .with_name(state.label.clone())
            .with_state(
                SemanticState::ENABLED
                    .with_disabled(!state.enabled)
                    .with_checked(state.checked),
            );
        if state.actionable {
            node = node.with_action(SemanticAction::Activate);
        }
        SemanticContribution::single(node)
    }
}

impl<Action: 'static> View<Action> for Checkbox<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(crate::Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(BinaryControlWidget {
                kind: BinaryControlKind::Checkbox,
                label: self.label,
                checked: self.checked,
                enabled: self.enabled,
                activation_factory: self.activation_factory,
                actionable: self.actionable,
            })),
            Vec::new(),
            diagnostics,
        )
    }
}

impl<Action: 'static> View<Action> for Switch<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(crate::Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(BinaryControlWidget {
                kind: BinaryControlKind::Switch,
                label: self.label,
                checked: self.checked.into(),
                enabled: self.enabled,
                activation_factory: self.activation_factory,
                actionable: self.actionable,
            })),
            Vec::new(),
            diagnostics,
        )
    }
}

pub struct Container<Action> {
    widget: Box<dyn ErasedWidget<Action>>,
    children: Vec<Element<Action>>,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for Container<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Container")
            .field("widget", &self.widget)
            .field("children", &self.children)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("layout", &self.common.layout)
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("diagnostics", &self.common.diagnostics)
            .finish()
    }
}

impl<Action> Container<Action> {
    #[must_use]
    pub fn new<Implementation>(widget: Implementation, children: impl Views<Action>) -> Self
    where
        Implementation: ChildBearingWidget<Action> + 'static,
    {
        Self {
            widget: Box::new(WidgetAdapter(widget)),
            children: children.into_elements(),
            common: CommonNodeAuthoring::default(),
        }
    }
    common_node_builder_methods!();
    #[must_use]
    pub fn gap(mut self, gap: impl Into<LogicalLength>) -> Self {
        self.common.layout = self.common.layout.with_gap(gap);
        self
    }
}

#[derive(Debug)]
struct GroupWidget;

impl<Action> Widget<Action> for GroupWidget {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Group);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl<Action> ChildBearingWidget<Action> for GroupWidget {}

impl<Action: 'static> View<Action> for Container<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(crate::Focusability::Automatic, None);
        Element::from_authored_parts(fields, self.widget, self.children, diagnostics)
    }
}

#[must_use]
pub fn checkbox<Action>(
    label: impl Into<String>,
    checked: impl Into<SemanticCheckedState>,
) -> Checkbox<Action> {
    Checkbox::new(label, checked)
}
#[must_use]
pub fn switch<Action>(label: impl Into<String>, checked: bool) -> Switch<Action> {
    Switch::new(label, checked)
}
#[must_use]
pub fn text(content: impl Into<String>) -> Text {
    Text::new(content)
}
#[must_use]
pub fn button<Action>(label: impl Into<String>) -> Button<Action> {
    Button::new(label)
}
#[must_use]
pub fn container<Action, Implementation>(
    widget: Implementation,
    children: impl Views<Action>,
) -> Container<Action>
where
    Implementation: ChildBearingWidget<Action> + 'static,
{
    Container::new(widget, children)
}
#[must_use]
pub fn column<Action>(children: impl Views<Action>) -> Container<Action> {
    Container::new(GroupWidget, children).with_layout(LayoutStyle::default().with_container(
        LayoutContainer::Flex(FlexContainerStyle::default().with_direction(FlexDirection::Column)),
    ))
}
#[must_use]
pub fn row<Action>(children: impl Views<Action>) -> Container<Action> {
    Container::new(GroupWidget, children).with_layout(LayoutStyle::default().with_container(
        LayoutContainer::Flex(FlexContainerStyle::default().with_direction(FlexDirection::Row)),
    ))
}

fn local_rect(size: LogicalSize) -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
        .unwrap_or_else(|_| unreachable!("validated local size yields a valid local rectangle"))
}
