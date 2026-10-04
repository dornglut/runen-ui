use core::fmt;

use crate::{
    ApplicationCommand, ApplicationCommandDisposition, ApplicationCommandEvent, Axis, EventContext,
    EventPhase, FlexContainerStyle, FlexDirection, FocusGroup, FocusGroupActivationPolicy,
    FocusGroupBoundaryPolicy, Focusability, HitContribution, HitContributionContext, KeyboardPhase,
    LayoutContainer, LayoutStyle, LogicalKey, LogicalLength, LogicalPoint, LogicalRect,
    LogicalSize, OverflowStyle, PointerButton, PointerCaptureKind, PointerDeviceKind, PointerId,
    PointerPhase, ScrollBarLayout, ScrollBarPlacement, ScrollBarVisibility, ScrollChrome,
    ScrollControlBinding, ScrollControlRequest, ScrollNormalizedValue, SemanticAction,
    SemanticCheckedState, SemanticCommand, SemanticCommandEvent, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticNumber, SemanticRole,
    SemanticState, SemanticText, ShortcutBinding, StyleIntent, UiEvent, WidgetActivationContext,
    WidgetDiagnostic, WidgetEventOutput, WidgetInvalidation, WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, View, Views, common_node_builder_methods},
    widget_erasure::{ErasedWidget, WidgetAdapter},
    widget_protocol::{
        ChildBearingWidget, Widget, WidgetActivation, WidgetActivationOutput, WidgetMeasure,
        WidgetMeasureInput,
    },
};

pub struct CommandBinding<Action> {
    command: ApplicationCommand,
    action_factory: Box<dyn FnMut() -> Action>,
}
impl<Action> fmt::Debug for CommandBinding<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandBinding")
            .field("command", &self.command)
            .finish_non_exhaustive()
    }
}
impl<Action> CommandBinding<Action> {
    #[must_use]
    pub fn new(command: ApplicationCommand, action: impl FnMut() -> Action + 'static) -> Self {
        Self {
            command,
            action_factory: Box::new(action),
        }
    }
    #[must_use]
    pub const fn command(&self) -> &ApplicationCommand {
        &self.command
    }
}

pub struct CommandScope<Action> {
    bindings: Vec<CommandBinding<Action>>,
    children: Vec<Element<Action>>,
    common: CommonNodeAuthoring,
}
impl<Action> fmt::Debug for CommandScope<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandScope")
            .field("binding_count", &self.bindings.len())
            .field("children", &self.children)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .finish_non_exhaustive()
    }
}
impl<Action> CommandScope<Action> {
    #[must_use]
    pub fn new(
        bindings: impl IntoIterator<Item = CommandBinding<Action>>,
        children: impl Views<Action>,
    ) -> Self {
        Self {
            bindings: bindings.into_iter().collect(),
            children: children.into_elements(),
            common: CommonNodeAuthoring::default(),
        }
    }
    common_node_builder_methods!();
}
struct CommandScopeWidget<Action> {
    bindings: Vec<CommandBinding<Action>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CommandScopeWidgetState {
    has_duplicate_binding: bool,
}

impl<Action> CommandScopeWidget<Action> {
    fn has_duplicate_binding(&self) -> bool {
        self.bindings.iter().enumerate().any(|(index, binding)| {
            self.bindings[index + 1..]
                .iter()
                .any(|other| other.command().id() == binding.command().id())
        })
    }
}

impl<Action> fmt::Debug for CommandScopeWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandScopeWidget")
            .field("binding_count", &self.bindings.len())
            .finish()
    }
}
impl<Action> Widget<Action> for CommandScopeWidget<Action> {
    type State = CommandScopeWidgetState;

    fn create_state(&self) -> Self::State {
        CommandScopeWidgetState {
            has_duplicate_binding: self.has_duplicate_binding(),
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        let has_duplicate_binding = self.has_duplicate_binding();
        if state.has_duplicate_binding != has_duplicate_binding {
            context.invalidate(WidgetInvalidation::DIAGNOSTICS);
        }
        state.has_duplicate_binding = has_duplicate_binding;
    }

    fn event(
        &mut self,
        _: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Capture {
            return WidgetEventOutput::none();
        }
        let Some(command) = event
            .as_application_command()
            .map(ApplicationCommandEvent::command)
        else {
            return WidgetEventOutput::none();
        };
        let mut matching = None;
        let mut duplicate = false;
        for (index, binding) in self.bindings.iter().enumerate() {
            if binding.command().id() == command && matching.replace(index).is_some() {
                duplicate = true;
                break;
            }
        }
        let Some(index) = matching else {
            return WidgetEventOutput::none();
        };
        if duplicate {
            context.consume_application_command(ApplicationCommandDisposition::Ambiguous);
            return WidgetEventOutput::none();
        }
        if !self.bindings[index].command().enabled() {
            context.consume_application_command(ApplicationCommandDisposition::Disabled);
            return WidgetEventOutput::none();
        }
        context.consume_application_command(ApplicationCommandDisposition::Resolved);
        let action = (self.bindings[index].action_factory)();
        context.emit(action);
        WidgetEventOutput::none()
    }
    fn diagnostics(&self, state: &Self::State) -> Vec<WidgetDiagnostic> {
        if state.has_duplicate_binding {
            vec![WidgetDiagnostic::new(
                "runenui.command-scope.duplicate-command",
                "CommandScope contains duplicate bindings for one application command",
            )]
        } else {
            Vec::new()
        }
    }
}
impl<Action> ChildBearingWidget<Action> for CommandScopeWidget<Action> {}
impl<Action: 'static> View<Action> for CommandScope<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(crate::Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(CommandScopeWidget {
                bindings: self.bindings,
            })),
            self.children,
            diagnostics,
        )
    }
}
#[must_use]
pub fn command_binding<Action>(
    command: ApplicationCommand,
    action: impl FnMut() -> Action + 'static,
) -> CommandBinding<Action> {
    CommandBinding::new(command, action)
}
#[must_use]
pub fn command_scope<Action>(
    bindings: impl IntoIterator<Item = CommandBinding<Action>>,
    children: impl Views<Action>,
) -> CommandScope<Action> {
    CommandScope::new(bindings, children)
}

pub struct ShortcutScope<Action> {
    bindings: Vec<ShortcutBinding>,
    children: Vec<Element<Action>>,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for ShortcutScope<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ShortcutScope")
            .field("binding_count", &self.bindings.len())
            .field("children", &self.children)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .finish_non_exhaustive()
    }
}

impl<Action> ShortcutScope<Action> {
    #[must_use]
    pub fn new(
        bindings: impl IntoIterator<Item = ShortcutBinding>,
        children: impl Views<Action>,
    ) -> Self {
        Self {
            bindings: bindings.into_iter().collect(),
            children: children.into_elements(),
            common: CommonNodeAuthoring::default(),
        }
    }

    common_node_builder_methods!();
}

#[derive(Debug)]
struct ShortcutScopeWidget {
    bindings: Vec<ShortcutBinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ShortcutScopeWidgetState {
    has_duplicate_chord: bool,
}

impl ShortcutScopeWidget {
    fn has_duplicate_chord(&self) -> bool {
        self.bindings.iter().enumerate().any(|(index, binding)| {
            self.bindings[index + 1..]
                .iter()
                .any(|other| other.chord() == binding.chord())
        })
    }
}

impl<Action> Widget<Action> for ShortcutScopeWidget {
    type State = ShortcutScopeWidgetState;

    fn create_state(&self) -> Self::State {
        ShortcutScopeWidgetState {
            has_duplicate_chord: self.has_duplicate_chord(),
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        let has_duplicate_chord = self.has_duplicate_chord();
        if state.has_duplicate_chord != has_duplicate_chord {
            context.invalidate(WidgetInvalidation::DIAGNOSTICS);
        }
        state.has_duplicate_chord = has_duplicate_chord;
    }

    fn shortcuts(&self) -> &[ShortcutBinding] {
        self.bindings.as_slice()
    }

    fn diagnostics(&self, state: &Self::State) -> Vec<WidgetDiagnostic> {
        if state.has_duplicate_chord {
            vec![WidgetDiagnostic::new(
                "runenui.shortcut-scope.duplicate-chord",
                "ShortcutScope contains duplicate bindings for one shortcut chord",
            )]
        } else {
            Vec::new()
        }
    }
}

impl<Action> ChildBearingWidget<Action> for ShortcutScopeWidget {}

impl<Action: 'static> View<Action> for ShortcutScope<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(crate::Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(ShortcutScopeWidget {
                bindings: self.bindings,
            })),
            self.children,
            diagnostics,
        )
    }
}

#[must_use]
pub fn shortcut_scope<Action>(
    bindings: impl IntoIterator<Item = ShortcutBinding>,
    children: impl Views<Action>,
) -> ShortcutScope<Action> {
    ShortcutScope::new(bindings, children)
}

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

pub struct RadioButton<Action> {
    label: String,
    checked: bool,
    common: CommonNodeAuthoring,
    enabled: bool,
    focusability: Focusability,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
    actionable: bool,
}

impl<Action> fmt::Debug for RadioButton<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RadioButton")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("layout", &self.common.layout)
            .field("enabled", &self.enabled)
            .field("focusability", &self.focusability)
            .field("actionable", &self.actionable)
            .field("has_callback", &self.activation_factory.is_some())
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("diagnostics", &self.common.diagnostics)
            .finish()
    }
}

impl<Action> RadioButton<Action> {
    #[must_use]
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        Self {
            label: label.into(),
            checked,
            common: CommonNodeAuthoring::default(),
            enabled: true,
            focusability: Focusability::Automatic,
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
    /// Excludes this radio from focus selection while retaining its authored control state.
    #[must_use]
    pub const fn focus_hidden(mut self, hidden: bool) -> Self {
        self.focusability = if hidden {
            Focusability::Hidden
        } else {
            Focusability::Automatic
        };
        self
    }
    #[must_use]
    pub fn on_activate(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.activation_factory = Some(Box::new(callback));
        self.actionable = true;
        self
    }
    #[must_use]
    pub const fn checked(&self) -> bool {
        self.checked
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
    RadioButton,
    Switch,
}

impl BinaryControlKind {
    const fn role(self) -> SemanticRole {
        match self {
            Self::Checkbox => SemanticRole::Checkbox,
            Self::RadioButton => SemanticRole::RadioButton,
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

impl<Action: 'static> View<Action> for RadioButton<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self.common.into_authored_fields(self.focusability, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(BinaryControlWidget {
                kind: BinaryControlKind::RadioButton,
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

pub struct RadioGroup<Action> {
    children: Vec<RadioButton<Action>>,
    common: CommonNodeAuthoring,
    standalone_navigation: bool,
}

impl<Action> fmt::Debug for RadioGroup<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RadioGroup")
            .field("children", &self.children)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("layout", &self.common.layout)
            .field("standalone_navigation", &self.standalone_navigation)
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("diagnostics", &self.common.diagnostics)
            .finish()
    }
}

impl<Action> RadioGroup<Action> {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = RadioButton<Action>>) -> Self {
        Self {
            children: children.into_iter().collect(),
            common: CommonNodeAuthoring {
                layout: LayoutStyle::default().with_container(LayoutContainer::Flex(
                    FlexContainerStyle::default().with_direction(FlexDirection::Column),
                )),
                ..CommonNodeAuthoring::default()
            },
            standalone_navigation: true,
        }
    }
    common_node_builder_methods!();
    #[must_use]
    pub fn gap(mut self, gap: impl Into<LogicalLength>) -> Self {
        self.common.layout = self.common.layout.with_gap(gap);
        self
    }
    /// Enables or suppresses standalone arrow-key/controller remapping.
    ///
    /// Disable this when an enclosing composite such as a future toolbar owns
    /// directional navigation.
    #[must_use]
    pub const fn standalone_navigation(mut self, enabled: bool) -> Self {
        self.standalone_navigation = enabled;
        self
    }
}

#[derive(Debug)]
struct RadioGroupWidget {
    multiple_checked: bool,
    standalone_navigation: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RadioGroupWidgetState {
    multiple_checked: bool,
}

impl<Action> Widget<Action> for RadioGroupWidget {
    type State = RadioGroupWidgetState;

    fn create_state(&self) -> Self::State {
        RadioGroupWidgetState {
            multiple_checked: self.multiple_checked,
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.multiple_checked != self.multiple_checked {
            context.invalidate(WidgetInvalidation::SEMANTICS | WidgetInvalidation::DIAGNOSTICS);
        }
        state.multiple_checked = self.multiple_checked;
    }

    fn event(
        &mut self,
        _: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if !self.standalone_navigation
            || context.phase() != EventPhase::Bubble
            || context.default_is_prevented()
        {
            return WidgetEventOutput::none();
        }
        let Some(command) = event
            .as_semantic_command()
            .map(SemanticCommandEvent::command)
        else {
            return WidgetEventOutput::none();
        };
        let delegated = match command {
            SemanticCommand::FocusLeft | SemanticCommand::FocusUp => {
                Some(SemanticCommand::FocusGroupPrevious)
            }
            SemanticCommand::FocusRight | SemanticCommand::FocusDown => {
                Some(SemanticCommand::FocusGroupNext)
            }
            _ => None,
        };
        if let Some(delegated) = delegated {
            context.prevent_default();
            context.stop_propagation();
            context.emit_command(delegated);
        }
        WidgetEventOutput::none()
    }

    fn semantics(
        &self,
        state: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        if state.multiple_checked {
            return SemanticContribution::empty();
        }
        let mut node = SemanticNodeContribution::primary(SemanticRole::RadioGroup);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }

    fn diagnostics(&self, state: &Self::State) -> Vec<WidgetDiagnostic> {
        if state.multiple_checked {
            vec![WidgetDiagnostic::new(
                "runenui.control.radio-group.multiple-checked",
                "RadioGroup requires at most one checked RadioButton",
            )]
        } else {
            Vec::new()
        }
    }
}

impl<Action> ChildBearingWidget<Action> for RadioGroupWidget {}

impl<Action: 'static> View<Action> for RadioGroup<Action> {
    fn into_element(self) -> Element<Action> {
        let multiple_checked = self.children.iter().filter(|child| child.checked()).count() > 1;
        let children = self
            .children
            .into_iter()
            .map(|child| {
                let preferred = child.checked();
                child.into_element().focus_group_preferred(preferred)
            })
            .collect();
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(crate::Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(RadioGroupWidget {
                multiple_checked,
                standalone_navigation: self.standalone_navigation,
            })),
            children,
            diagnostics,
        )
        .focus_group(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                .with_activation(FocusGroupActivationPolicy::ActivateTarget),
        )
    }
}

/// Standard M11 scrollbar composed entirely from public scroll-control and chrome contracts.
///
/// The track is this view's ordinary node. The thumb is an ordinary child node
/// using the same generic binding and styling/hit/pointer protocols available to
/// downstream widgets. Neither node owns or mirrors logical scroll state.
pub struct ScrollBar {
    label: String,
    binding: ScrollControlBinding,
    layout: ScrollBarLayout,
    thumb_style: StyleIntent,
    common: CommonNodeAuthoring,
}

impl fmt::Debug for ScrollBar {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ScrollBar")
            .field("label", &self.label)
            .field("binding", &self.binding)
            .field("layout", &self.layout)
            .field("thumb_style", &self.thumb_style)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("diagnostics", &self.common.diagnostics)
            .finish()
    }
}

impl ScrollBar {
    #[must_use]
    pub fn new(
        label: impl Into<String>,
        binding: ScrollControlBinding,
        thickness: impl Into<LogicalLength>,
        minimum_thumb_extent: impl Into<LogicalLength>,
    ) -> Self {
        Self {
            label: label.into(),
            binding,
            layout: ScrollBarLayout::new(
                binding.axis(),
                thickness.into(),
                minimum_thumb_extent.into(),
            ),
            thumb_style: StyleIntent::EMPTY,
            common: CommonNodeAuthoring::default(),
        }
    }

    common_node_builder_methods!();

    #[must_use]
    pub const fn visibility(mut self, visibility: ScrollBarVisibility) -> Self {
        self.layout = self.layout.with_visibility(visibility);
        self
    }

    #[must_use]
    pub const fn placement(mut self, placement: ScrollBarPlacement) -> Self {
        self.layout = self.layout.with_placement(placement);
        self
    }

    /// Replaces the ordinary authored style of the interactive thumb child.
    #[must_use]
    pub fn thumb_style(mut self, style: StyleIntent) -> Self {
        self.thumb_style = style;
        self
    }

    #[must_use]
    pub const fn binding(&self) -> ScrollControlBinding {
        self.binding
    }

    #[must_use]
    pub const fn scroll_bar_layout(&self) -> ScrollBarLayout {
        self.layout
    }
}

const fn scroll_bar_axis_coordinate(point: LogicalPoint, axis: Axis) -> f32 {
    match axis {
        Axis::Horizontal => point.x(),
        Axis::Vertical => point.y(),
    }
}

fn scroll_bar_pointer_down_is_primary(event: &crate::PointerEvent) -> bool {
    event.phase() == PointerPhase::Down
        && match event.device_kind() {
            PointerDeviceKind::Touch => true,
            _ => event.changed_button() == Some(PointerButton::Primary),
        }
}

fn emit_scroll_bar_request<Action>(
    context: &mut EventContext<'_, Action>,
    request: ScrollControlRequest,
) {
    context.emit_command(SemanticCommand::ScrollControl(request));
    context.prevent_default();
    context.stop_propagation();
}

fn scroll_bar_keyboard_request(
    event: &crate::KeyboardEvent,
    axis: Axis,
) -> Option<ScrollControlRequest> {
    if event.phase() != KeyboardPhase::Down {
        return None;
    }
    let modifiers = event.modifiers();
    match event.logical_key() {
        LogicalKey::ArrowLeft
            if axis == Axis::Horizontal && modifiers == crate::KeyModifiers::NONE =>
        {
            Some(ScrollControlRequest::SmallStepBackward)
        }
        LogicalKey::ArrowRight
            if axis == Axis::Horizontal && modifiers == crate::KeyModifiers::NONE =>
        {
            Some(ScrollControlRequest::SmallStepForward)
        }
        LogicalKey::ArrowUp if axis == Axis::Vertical && modifiers == crate::KeyModifiers::NONE => {
            Some(ScrollControlRequest::SmallStepBackward)
        }
        LogicalKey::ArrowDown
            if axis == Axis::Vertical && modifiers == crate::KeyModifiers::NONE =>
        {
            Some(ScrollControlRequest::SmallStepForward)
        }
        LogicalKey::PageUp if modifiers == crate::KeyModifiers::NONE => {
            Some(ScrollControlRequest::PageBackward)
        }
        LogicalKey::PageDown | LogicalKey::Space if modifiers == crate::KeyModifiers::NONE => {
            Some(ScrollControlRequest::PageForward)
        }
        LogicalKey::Home if modifiers == crate::KeyModifiers::NONE => {
            Some(ScrollControlRequest::ToStart)
        }
        LogicalKey::End if modifiers == crate::KeyModifiers::NONE => {
            Some(ScrollControlRequest::ToEnd)
        }
        LogicalKey::Space if modifiers == crate::KeyModifiers::SHIFT => {
            Some(ScrollControlRequest::PageBackward)
        }
        _ => None,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the semantic percentage is range-checked to finite [0, 100] before normalization into the accepted f32 scroll protocol"
)]
fn normalized_scroll_bar_percentage(value: SemanticNumber) -> Option<ScrollNormalizedValue> {
    let percentage = value.get();
    if !(0.0..=100.0).contains(&percentage) {
        return None;
    }
    ScrollNormalizedValue::new((percentage / 100.0) as f32).ok()
}

fn scroll_bar_semantic_request(command: SemanticCommand) -> Option<ScrollControlRequest> {
    match command {
        SemanticCommand::Increment => Some(ScrollControlRequest::SmallStepForward),
        SemanticCommand::Decrement => Some(ScrollControlRequest::SmallStepBackward),
        SemanticCommand::SetValue(value) => {
            normalized_scroll_bar_percentage(value).map(ScrollControlRequest::SetNormalized)
        }
        _ => None,
    }
}

#[derive(Debug)]
struct ScrollBarWidget {
    label: String,
    layout: ScrollBarLayout,
}

impl<Action> Widget<Action> for ScrollBarWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Target {
            return WidgetEventOutput::none();
        }

        if let Some(keyboard) = event.as_keyboard()
            && let Some(request) = scroll_bar_keyboard_request(keyboard, self.layout.axis())
        {
            if context.scroll_control_snapshot().is_some_and(|snapshot| {
                snapshot.axis() == self.layout.axis() && snapshot.maximum_offset().get() > 0.0
            }) {
                emit_scroll_bar_request(context, request);
            }
            return WidgetEventOutput::none();
        }

        if let Some(request) = event
            .as_semantic_command()
            .and_then(|event| scroll_bar_semantic_request(event.command()))
        {
            if context.scroll_control_snapshot().is_some_and(|snapshot| {
                snapshot.axis() == self.layout.axis() && snapshot.maximum_offset().get() > 0.0
            }) {
                emit_scroll_bar_request(context, request);
            }
            return WidgetEventOutput::none();
        }

        let Some(pointer) = event.as_pointer() else {
            return WidgetEventOutput::none();
        };
        if !scroll_bar_pointer_down_is_primary(pointer) {
            return WidgetEventOutput::none();
        }
        let (Some(snapshot), Some(local)) = (
            context.scroll_control_snapshot(),
            context.pointer_local_position(),
        ) else {
            return WidgetEventOutput::none();
        };
        if snapshot.maximum_offset().get() == 0.0 {
            return WidgetEventOutput::none();
        }
        let Some(geometry) = self
            .layout
            .thumb_geometry(snapshot, snapshot.viewport_extent())
        else {
            return WidgetEventOutput::none();
        };
        let coordinate = scroll_bar_axis_coordinate(local, self.layout.axis());
        let request = if coordinate < geometry.thumb_origin().get() {
            Some(ScrollControlRequest::PageBackward)
        } else if coordinate > geometry.thumb_origin().get() + geometry.thumb_extent().get() {
            Some(ScrollControlRequest::PageForward)
        } else {
            None
        };
        if let Some(request) = request {
            emit_scroll_bar_request(context, request);
        }
        WidgetEventOutput::none()
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        HitContribution::single_rect(local_rect(context.local_size()))
    }

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::ScrollBar)
            .with_name(self.label.clone())
            .with_action(SemanticAction::RequestFocus);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl<Action> ChildBearingWidget<Action> for ScrollBarWidget {}

#[derive(Clone, Copy, Debug)]
struct ScrollBarDrag {
    pointer_id: PointerId,
    grab_offset: f32,
}

#[derive(Debug, Default)]
struct ScrollBarThumbState {
    drag: Option<ScrollBarDrag>,
}

#[derive(Debug)]
struct ScrollBarThumbWidget {
    layout: ScrollBarLayout,
}

impl ScrollBarThumbWidget {
    fn fail_closed_drag<Action>(
        state: &mut ScrollBarThumbState,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if state.drag.take().is_some() {
            context.release_pointer_capture();
            context.prevent_default();
            context.stop_propagation();
            WidgetEventOutput::changed()
        } else {
            WidgetEventOutput::none()
        }
    }
}

impl<Action> Widget<Action> for ScrollBarThumbWidget {
    type State = ScrollBarThumbState;

    fn create_state(&self) -> Self::State {
        ScrollBarThumbState::default()
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Target {
            return WidgetEventOutput::none();
        }

        if let Some(capture) = event.as_pointer_capture()
            && capture.kind() == PointerCaptureKind::Lost
            && state
                .drag
                .is_some_and(|drag| drag.pointer_id == capture.pointer_id())
        {
            state.drag = None;
            return WidgetEventOutput::changed();
        }

        let Some(pointer) = event.as_pointer() else {
            return WidgetEventOutput::none();
        };

        match pointer.phase() {
            PointerPhase::Down if scroll_bar_pointer_down_is_primary(pointer) => {
                if state.drag.is_some() {
                    return WidgetEventOutput::none();
                }
                let (Some(snapshot), Some(local)) = (
                    context.scroll_control_snapshot(),
                    context.pointer_local_position(),
                ) else {
                    return WidgetEventOutput::none();
                };
                let Some(geometry) = self
                    .layout
                    .thumb_geometry(snapshot, snapshot.viewport_extent())
                else {
                    return WidgetEventOutput::none();
                };
                if snapshot.maximum_offset().get() == 0.0 || geometry.travel().get() == 0.0 {
                    return WidgetEventOutput::none();
                }
                let coordinate = scroll_bar_axis_coordinate(local, self.layout.axis());
                state.drag = Some(ScrollBarDrag {
                    pointer_id: pointer.pointer_id(),
                    grab_offset: coordinate.clamp(0.0, geometry.thumb_extent().get()),
                });
                context.capture_pointer();
                context.prevent_default();
                context.stop_propagation();
                WidgetEventOutput::changed()
            }
            PointerPhase::Move
                if state
                    .drag
                    .is_some_and(|drag| drag.pointer_id == pointer.pointer_id()) =>
            {
                let Some(drag) = state.drag else {
                    return WidgetEventOutput::none();
                };
                let (Some(snapshot), Some(local)) = (
                    context.scroll_control_snapshot(),
                    context.pointer_local_position(),
                ) else {
                    return Self::fail_closed_drag(state, context);
                };
                let Some(geometry) = self
                    .layout
                    .thumb_geometry(snapshot, snapshot.viewport_extent())
                else {
                    return Self::fail_closed_drag(state, context);
                };
                if geometry.travel().get() == 0.0 || snapshot.maximum_offset().get() == 0.0 {
                    return Self::fail_closed_drag(state, context);
                }
                let coordinate = scroll_bar_axis_coordinate(local, self.layout.axis());
                let desired_origin = (geometry.thumb_origin().get() + coordinate
                    - drag.grab_offset)
                    .clamp(0.0, geometry.travel().get());
                let normalized =
                    ScrollNormalizedValue::new(desired_origin / geometry.travel().get())
                        .unwrap_or_else(|_| {
                            unreachable!("clamped thumb travel yields normalized value")
                        });
                emit_scroll_bar_request(context, ScrollControlRequest::SetNormalized(normalized));
                WidgetEventOutput::none()
            }
            PointerPhase::Up | PointerPhase::Cancel
                if state
                    .drag
                    .is_some_and(|drag| drag.pointer_id == pointer.pointer_id()) =>
            {
                state.drag = None;
                context.prevent_default();
                context.stop_propagation();
                WidgetEventOutput::changed()
            }
            _ => WidgetEventOutput::none(),
        }
    }

    fn hit_test(&self, _: &Self::State, context: HitContributionContext) -> HitContribution {
        HitContribution::single_rect(local_rect(context.local_size()))
    }
}

impl<Action: 'static> View<Action> for ScrollBar {
    fn into_element(self) -> Element<Action> {
        let axis = self.binding.axis();
        let thumb_common = CommonNodeAuthoring {
            style: self.thumb_style,
            ..CommonNodeAuthoring::default()
        };
        let (thumb_fields, thumb_diagnostics) =
            thumb_common.into_authored_fields(Focusability::NotFocusable, None);
        let thumb = Element::from_authored_parts(
            thumb_fields,
            Box::new(WidgetAdapter(ScrollBarThumbWidget {
                layout: self.layout,
            })),
            Vec::new(),
            thumb_diagnostics,
        )
        .scroll_control(self.binding)
        .scroll_chrome(ScrollChrome::Thumb(axis));

        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(ScrollBarWidget {
                label: self.label,
                layout: self.layout,
            })),
            vec![thumb],
            diagnostics,
        )
        .scroll_control(self.binding)
        .scroll_chrome(ScrollChrome::Bar(self.layout))
    }
}

#[must_use]
pub fn scroll_bar(
    label: impl Into<String>,
    binding: ScrollControlBinding,
    thickness: impl Into<LogicalLength>,
    minimum_thumb_extent: impl Into<LogicalLength>,
) -> ScrollBar {
    ScrollBar::new(label, binding, thickness, minimum_thumb_extent)
}

/// Standard scroll owner that composes content with ordinary public scrollbar views.
///
/// Bars remain ordinary descendants bound through `ScrollControlBinding`. The
/// optional reserved corner is an ordinary noninteractive styled node. Runtime
/// derives all visibility, viewport geometry, ownership, and scroll state.
pub struct ScrollContainer<Action> {
    content: Element<Action>,
    scroll_bars: Vec<ScrollBar>,
    corner_style: StyleIntent,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for ScrollContainer<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ScrollContainer")
            .field("content", &self.content)
            .field("scroll_bars", &self.scroll_bars)
            .field("corner_style", &self.corner_style)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("layout", &self.common.layout)
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("diagnostics", &self.common.diagnostics)
            .finish()
    }
}

impl<Action> ScrollContainer<Action> {
    #[must_use]
    pub fn new(content: impl View<Action>, overflow: OverflowStyle) -> Self {
        Self {
            content: content.into_element(),
            scroll_bars: Vec::new(),
            corner_style: StyleIntent::EMPTY,
            common: CommonNodeAuthoring {
                layout: LayoutStyle::default().with_overflow(overflow),
                ..CommonNodeAuthoring::default()
            },
        }
    }

    common_node_builder_methods!();

    /// Appends one standard bar. Axis ownership is taken from the bar's public
    /// binding; duplicate same-axis bars remain explicit authored structure and
    /// are rejected by the generic runtime chrome validation rather than being
    /// silently overwritten here.
    #[must_use]
    pub fn scroll_bar(mut self, scroll_bar: ScrollBar) -> Self {
        self.scroll_bars.push(scroll_bar);
        self
    }

    /// Replaces the ordinary authored style used by the reserved two-axis corner.
    #[must_use]
    pub fn corner_style(mut self, style: StyleIntent) -> Self {
        self.corner_style = style;
        self
    }
}

#[derive(Debug)]
struct ScrollCornerWidget;

impl<Action> Widget<Action> for ScrollCornerWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}
}

impl<Action: 'static> View<Action> for ScrollContainer<Action> {
    fn into_element(self) -> Element<Action> {
        let has_horizontal = self
            .scroll_bars
            .iter()
            .any(|bar| bar.binding().axis() == Axis::Horizontal);
        let has_vertical = self
            .scroll_bars
            .iter()
            .any(|bar| bar.binding().axis() == Axis::Vertical);

        let mut children = Vec::with_capacity(
            1 + self.scroll_bars.len() + usize::from(has_horizontal && has_vertical),
        );
        children.push(self.content);
        children.extend(self.scroll_bars.into_iter().map(View::into_element));

        if has_horizontal && has_vertical {
            let corner_common = CommonNodeAuthoring {
                style: self.corner_style,
                ..CommonNodeAuthoring::default()
            };
            let (corner_fields, corner_diagnostics) =
                corner_common.into_authored_fields(Focusability::NotFocusable, None);
            children.push(
                Element::from_authored_parts(
                    corner_fields,
                    Box::new(WidgetAdapter(ScrollCornerWidget)),
                    Vec::new(),
                    corner_diagnostics,
                )
                .scroll_chrome(ScrollChrome::Corner),
            );
        }

        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(ScrollViewportWidget)),
            children,
            diagnostics,
        )
    }
}

#[must_use]
pub fn scroll_container<Action>(
    content: impl View<Action>,
    overflow: OverflowStyle,
) -> ScrollContainer<Action> {
    ScrollContainer::new(content, overflow)
}

pub struct ScrollViewport<Action> {
    content: Element<Action>,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for ScrollViewport<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ScrollViewport")
            .field("content", &self.content)
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("layout", &self.common.layout)
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("diagnostics", &self.common.diagnostics)
            .finish_non_exhaustive()
    }
}

impl<Action> ScrollViewport<Action> {
    #[must_use]
    pub fn new(content: impl View<Action>, overflow: OverflowStyle) -> Self {
        Self {
            content: content.into_element(),
            common: CommonNodeAuthoring {
                layout: LayoutStyle::default().with_overflow(overflow),
                ..CommonNodeAuthoring::default()
            },
        }
    }

    common_node_builder_methods!();
}

#[derive(Debug)]
struct ScrollViewportWidget;

impl<Action> Widget<Action> for ScrollViewportWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        HitContribution::single_rect(local_rect(context.local_size()))
    }

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

impl<Action> ChildBearingWidget<Action> for ScrollViewportWidget {}

impl<Action: 'static> View<Action> for ScrollViewport<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(crate::Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(ScrollViewportWidget)),
            vec![self.content],
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
pub fn radio_button<Action>(label: impl Into<String>, checked: bool) -> RadioButton<Action> {
    RadioButton::new(label, checked)
}
#[must_use]
pub fn radio_group<Action>(
    children: impl IntoIterator<Item = RadioButton<Action>>,
) -> RadioGroup<Action> {
    RadioGroup::new(children)
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
pub fn scroll_viewport<Action>(
    content: impl View<Action>,
    overflow: OverflowStyle,
) -> ScrollViewport<Action> {
    ScrollViewport::new(content, overflow)
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
