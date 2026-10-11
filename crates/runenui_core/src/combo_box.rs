//! Select-only and editable `ComboBox` over one M4 focus proxy and M10 editing authority.
//! All committed values and popup visibility are authored by the application.
use crate::{
    ChildBearingWidget, Element, ElementId, ElementKey, EventContext, EventPhase, FocusEventKind,
    Focusability, HitContribution, HitContributionContext, KeyboardCompositionState, KeyboardPhase,
    LayoutBound, LayoutContainer, LayoutStyle, LogicalKey, LogicalLength, OptionItem, OverflowPolicy,
    OverflowStyle, Popover, PresentationFocusPolicy,
    PresentationOutsidePointerPolicy, SemanticAction, SemanticAutocomplete, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticNodeContribution, SemanticPopupKind,
    SemanticReference, SemanticRelationship, SemanticRelationshipKind, SemanticRole, SemanticState,
    SemanticValue, SurfacePresentation, SurfacePresentationAnchor, SurfacePresentationPlacement,
    SurfacePresentationSide, TextField, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetDiagnostic, WidgetEventOutput,
    WidgetInvalidation, WidgetMeasure, WidgetMeasureInput, WidgetUpdateContext,
    element::{CommonNodeAuthoring, common_node_builder_methods},
    list_box, scroll_container,
    widget_erasure::WidgetAdapter,
};
use core::fmt;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

type RequestHandler<Action> = Rc<RefCell<Box<dyn FnMut(ComboBoxRequest) -> Action>>>;
type OptionActivation<Action> = Rc<RefCell<Box<dyn FnMut() -> Action>>>;

/// An application request, not an automatic durable selection or popup mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComboBoxRequest {
    Open,
    Close,
}

/// A select-only `ComboBox` using standard `OptionItem` children.
///
/// One real M4 focus target owns a transient active `Option`. The application's
/// selected `Option` and opened/closed popup remain authoritative. An editable
/// variant must reuse M10 `TextField`, not instantiate a second editor.
pub struct ComboBox<Action> {
    label: String,
    popup_id: ElementId,
    options: Vec<OptionItem<Action>>,
    popup_max_height: LogicalLength,
    open: bool,
    enabled: bool,
    on_request: RequestHandler<Action>,
    common: CommonNodeAuthoring,
}
impl<Action> fmt::Debug for ComboBox<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComboBox")
            .field("label", &self.label)
            .field("popup_id", &self.popup_id)
            .field("options", &self.options.len())
            .field("open", &self.open)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}
impl<Action> ComboBox<Action> {
    /// Options must have distinct explicit authored IDs. They remain the
    /// original public `OptionItem` contract, not a `ComboBox`-specific item type.
    #[must_use]
    pub fn new(
        label: impl Into<String>,
        popup_id: ElementId,
        options: impl IntoIterator<Item = OptionItem<Action>>,
        on_request: impl FnMut(ComboBoxRequest) -> Action + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            popup_id,
            options: options.into_iter().collect(),
            popup_max_height: LogicalLength::from(192_u16),
            open: false,
            enabled: true,
            on_request: Rc::new(RefCell::new(Box::new(on_request))),
            common: CommonNodeAuthoring {
                layout: LayoutStyle::default().with_container(LayoutContainer::Block),
                ..CommonNodeAuthoring::default()
            },
        }
    }
    common_node_builder_methods!();
    #[must_use]
    pub const fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Limits the ordinary scroll viewport of an opened popup without changing Option layout.
    #[must_use]
    pub const fn popup_max_height(mut self, height: LogicalLength) -> Self {
        self.popup_max_height = height;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Candidate {
    id: ElementId,
    key: Option<ElementKey>,
    selected: bool,
    enabled: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidComboBox {
    MissingOptionId,
    DuplicateOptionId,
    DuplicateOptionKey,
    MultipleSelected,
    PopupIdCollision,
    OwnerIdCollision,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComboState {
    pub(crate) label: String,
    pub(crate) popup_id: ElementId,
    options: Vec<Candidate>,
    selected_display: Option<String>,
    pub(crate) active: Option<ElementId>,
    pub(crate) open: bool,
    pub(crate) enabled: bool,
    pub(crate) invalid: Option<InvalidComboBox>,
}
pub struct ComboWidget<Action> {
    label: String,
    popup_id: ElementId,
    options: Vec<Candidate>,
    selected_display: Option<String>,
    open: bool,
    enabled: bool,
    pub(crate) invalid: Option<InvalidComboBox>,
    on_request: RequestHandler<Action>,
    option_activations: Vec<(ElementId, OptionActivation<Action>)>,
}
impl<Action> fmt::Debug for ComboWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComboWidget")
            .field("popup_id", &self.popup_id)
            .field("options", &self.options.len())
            .field("open", &self.open)
            .finish_non_exhaustive()
    }
}
impl<Action> ComboWidget<Action> {
    fn initial_active(&self) -> Option<ElementId> {
        self.options
            .iter()
            .find(|option| option.selected && option.enabled)
            .or_else(|| self.options.iter().find(|option| option.enabled))
            .map(|option| option.id.clone())
    }
    fn choose(&self, state: &mut ComboState, step: Navigation) {
        let eligible: Vec<_> = self.options.iter().filter(|o| o.enabled).collect();
        if eligible.is_empty() {
            state.active = None;
            return;
        }
        let current = state
            .active
            .as_ref()
            .and_then(|id| eligible.iter().position(|option| &option.id == id));
        let next = match step {
            Navigation::First => 0,
            Navigation::Last => eligible.len() - 1,
            Navigation::Next => current.map_or(0, |i| (i + 1).min(eligible.len() - 1)),
            Navigation::Previous => current.map_or(eligible.len() - 1, |i| i.saturating_sub(1)),
        };
        state.active = Some(eligible[next].id.clone());
    }
    pub(crate) fn accept_active(&self, state: &ComboState) -> Option<Action> {
        if !self.enabled || self.invalid.is_some() || !state.open {
            return None;
        }
        let id = state.active.as_ref()?;
        let (_, action) = self
            .option_activations
            .iter()
            .find(|(candidate, _)| candidate == id)?;
        Some((*action.borrow_mut())())
    }
    pub(crate) fn proxy_event(
        &self,
        state: &mut ComboState,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
        editable: bool,
    ) -> WidgetEventOutput {
        if !self.enabled
            || self.invalid.is_some()
            || context.phase() != EventPhase::Target
            || context.default_is_prevented()
        {
            return WidgetEventOutput::none();
        }
        // Losing the sole real focus owner dismisses an app-opened popup through
        // the same ordinary request queue. This is not a second focus registry.
        if state.open
            && event
                .as_focus()
                .is_some_and(|focus| focus.kind() == FocusEventKind::Out)
        {
            context.emit((*self.on_request.borrow_mut())(ComboBoxRequest::Close));
            return WidgetEventOutput::none();
        }
        let directive = if let Some(cmd) = event.as_semantic_command() {
            match cmd.command() {
                SemanticCommand::FocusUp => Some(Navigation::Previous),
                SemanticCommand::FocusDown => Some(Navigation::Next),
                SemanticCommand::FocusGroupFirst => Some(Navigation::First),
                SemanticCommand::FocusGroupLast => Some(Navigation::Last),
                SemanticCommand::Expand if !state.open => {
                    context.emit((*self.on_request.borrow_mut())(ComboBoxRequest::Open));
                    context.prevent_default();
                    context.stop_propagation();
                    return WidgetEventOutput::none();
                }
                SemanticCommand::Collapse if state.open => {
                    context.emit((*self.on_request.borrow_mut())(ComboBoxRequest::Close));
                    context.prevent_default();
                    context.stop_propagation();
                    return WidgetEventOutput::none();
                }
                _ => None,
            }
        } else if let Some(key) = event.as_keyboard() {
            if key.phase() != KeyboardPhase::Down
                || key.modifiers() != crate::KeyModifiers::NONE
                || key.composition_state() != KeyboardCompositionState::Inactive
            {
                return WidgetEventOutput::none();
            }
            match key.logical_key() {
                LogicalKey::ArrowUp => Some(Navigation::Previous),
                LogicalKey::ArrowDown => Some(Navigation::Next),
                LogicalKey::Home if !editable => Some(Navigation::First),
                LogicalKey::End if !editable => Some(Navigation::Last),
                LogicalKey::Enter if editable && state.open => {
                    if !key.is_repeat()
                        && let Some(action) = self.accept_active(state)
                    {
                        context.emit(action);
                    }
                    context.prevent_default();
                    context.stop_propagation();
                    return WidgetEventOutput::none();
                }
                LogicalKey::Escape if state.open => {
                    context.emit((*self.on_request.borrow_mut())(ComboBoxRequest::Close));
                    context.prevent_default();
                    context.stop_propagation();
                    return WidgetEventOutput::none();
                }
                _ => None,
            }
        } else {
            None
        };
        if let Some(directive) = directive {
            if state.open {
                let old = state.active.clone();
                self.choose(state, directive);
                if state.active != old {
                    context.invalidate(WidgetInvalidation::SEMANTICS);
                }
            } else {
                context.emit((*self.on_request.borrow_mut())(ComboBoxRequest::Open));
            }
            context.prevent_default();
            context.stop_propagation();
        }
        WidgetEventOutput::none()
    }
}
#[derive(Clone, Copy)]
enum Navigation {
    Previous,
    Next,
    First,
    Last,
}
impl<Action> Widget<Action> for ComboWidget<Action> {
    type State = ComboState;
    fn create_state(&self) -> Self::State {
        ComboState {
            label: self.label.clone(),
            popup_id: self.popup_id.clone(),
            options: self.options.clone(),
            selected_display: self.selected_display.clone(),
            active: if self.open && self.enabled && self.invalid.is_none() {
                self.initial_active()
            } else {
                None
            },
            open: self.open,
            enabled: self.enabled,
            invalid: self.invalid,
        }
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        let previous = state.clone();
        let mut next = self.create_state();
        if self.open
            && self.enabled
            && previous.open
            && previous.popup_id == self.popup_id
            && self.invalid.is_none()
        {
            // An authored ID is not a mounted-generation identity. An Option
            // with the same ID but a replaced reconciliation key must retire.
            // Keyed moves under this ListBox preserve identity; an unkeyed
            // candidate only retains its original reconciliation slot.
            next.active = previous.active.as_ref().and_then(|active| {
                previous
                    .options
                    .iter()
                    .enumerate()
                    .find(|(_, item)| &item.id == active)
                    .and_then(|(index, old)| {
                        let candidate = old.key.as_ref().map_or_else(
                            || {
                                self.options
                                    .get(index)
                                    .filter(|current| current.id == old.id && current.key.is_none())
                            },
                            |key| {
                                self.options.iter().find(|current| {
                                    current.id == old.id && current.key.as_ref() == Some(key)
                                })
                            },
                        );
                        candidate
                            .filter(|current| current.enabled)
                            .map(|_| active.clone())
                    })
            });
        }
        if previous != next {
            context.invalidate(WidgetInvalidation::SEMANTICS | WidgetInvalidation::INTERACTION);
            if previous.label != next.label || previous.open != next.open {
                context.invalidate(WidgetInvalidation::LAYOUT | WidgetInvalidation::PAINT);
            }
        }
        *state = next;
    }
    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.enabled && self.invalid.is_none())
    }
    fn activate(
        &mut self,
        state: &mut Self::State,
        _: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        if !self.enabled || self.invalid.is_some() {
            return WidgetActivationOutput::none();
        }
        if !state.open {
            return WidgetActivationOutput::action((*self.on_request.borrow_mut())(
                ComboBoxRequest::Open,
            ));
        }
        self.accept_active(state)
            .map_or_else(WidgetActivationOutput::none, WidgetActivationOutput::action)
    }
    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        self.proxy_event(state, event, context, false)
    }
    fn hit_test(&self, _: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = crate::LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("finite ComboBox geometry"));
        HitContribution::single_rect(rect)
    }
    fn semantics(
        &self,
        state: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        if state.invalid.is_some() {
            return SemanticContribution::empty();
        }
        let mut node = SemanticNodeContribution::primary(SemanticRole::ComboBox)
            .with_name(state.label.clone())
            .with_popup(SemanticPopupKind::ListBox)
            .with_state(
                SemanticState::ENABLED
                    .with_disabled(!state.enabled)
                    .with_expanded(state.open),
            )
            .with_action(SemanticAction::RequestFocus);
        if let Some(display) = state.selected_display.as_ref() {
            node = node.with_value(SemanticValue::Text(display.clone()));
        }
        if state.enabled {
            node = node
                .with_action(SemanticAction::Activate)
                .with_action(if state.open {
                    SemanticAction::Collapse
                } else {
                    SemanticAction::Expand
                });
        }
        if state.open {
            node = node.with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                SemanticReference::Authored {
                    element_id: state.popup_id.clone(),
                    semantic_key: None,
                },
            ));
            if let Some(id) = &state.active {
                node = node.with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::ActiveDescendant,
                    SemanticReference::Authored {
                        element_id: id.clone(),
                        semantic_key: None,
                    },
                ));
            }
        }
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
    fn diagnostics(&self, state: &Self::State) -> Vec<WidgetDiagnostic> {
        let Some(reason) = state.invalid else {
            return Vec::new();
        };
        let (code, text) = match reason {
            InvalidComboBox::MissingOptionId => (
                "runenui.control.combo.missing-option-id",
                "ComboBox popup Options need explicit authored IDs",
            ),
            InvalidComboBox::DuplicateOptionId => (
                "runenui.control.combo.duplicate-option-id",
                "ComboBox popup Option IDs must be unique",
            ),
            InvalidComboBox::DuplicateOptionKey => (
                "runenui.control.combo.duplicate-option-key",
                "ComboBox popup Option reconciliation keys must be unique",
            ),
            InvalidComboBox::MultipleSelected => (
                "runenui.control.combo.multiple-selected",
                "select-only ComboBox permits at most one selected Option",
            ),
            InvalidComboBox::PopupIdCollision => (
                "runenui.control.combo.popup-id-collision",
                "ComboBox popup ID must differ from all option IDs",
            ),
            InvalidComboBox::OwnerIdCollision => (
                "runenui.control.combo.owner-id-collision",
                "ComboBox owner ID must differ from the popup and all Option IDs",
            ),
        };
        vec![WidgetDiagnostic::new(code, text)]
    }
}
impl<Action> ChildBearingWidget<Action> for ComboWidget<Action> {}

#[derive(Debug)]
struct ComboLabel {
    label: String,
}
impl<Action> Widget<Action> for ComboLabel {
    type State = String;
    fn create_state(&self) -> Self::State {
        self.label.clone()
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.label {
            context.invalidate(WidgetInvalidation::LAYOUT | WidgetInvalidation::PAINT);
        }
        state.clone_from(&self.label);
    }
    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text(crate::TextLeafMeasure::new(self.label.clone()))
    }
    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::empty()
    }
}
fn assemble_combo<Action: 'static>(
    label: String,
    popup_id: ElementId,
    options_source: Vec<OptionItem<Action>>,
    open: bool,
    owner_enabled: bool,
    owner_id: Option<&ElementId>,
    popup_max_height: LogicalLength,
    on_request: RequestHandler<Action>,
) -> (ComboWidget<Action>, Option<Element<Action>>) {
    let mut seen = BTreeSet::new();
    let mut seen_keys = BTreeSet::new();
    let mut options = Vec::new();
    let mut option_activations = Vec::new();
    let mut popup_options = Vec::new();
    let mut invalid = (owner_id == Some(&popup_id)).then_some(InvalidComboBox::OwnerIdCollision);
    let selected_display = options_source
        .iter()
        .find_map(OptionItem::combo_selected_display)
        .map(str::to_owned);
    for mut item in options_source {
        let Some((id, selected, option_enabled)) = item.combo_proxy_facts() else {
            invalid.get_or_insert(InvalidComboBox::MissingOptionId);
            continue;
        };
        if owner_id == Some(&id) {
            invalid.get_or_insert(InvalidComboBox::OwnerIdCollision);
        } else if id == popup_id {
            invalid.get_or_insert(InvalidComboBox::PopupIdCollision);
        } else if !seen.insert(id.as_str().to_owned()) {
            invalid.get_or_insert(InvalidComboBox::DuplicateOptionId);
        }
        if let Some(key) = item.combo_proxy_key()
            && !seen_keys.insert(key.clone())
        {
            invalid.get_or_insert(InvalidComboBox::DuplicateOptionKey);
        }
        if let Some(callback) = item.share_combo_activation() {
            option_activations.push((id.clone(), callback));
        }
        options.push(Candidate {
            id,
            key: item.combo_proxy_key().cloned(),
            selected,
            enabled: option_enabled,
        });
        popup_options.push(if owner_enabled {
            item.combo_box_proxy()
        } else {
            item.combo_box_proxy().disabled()
        });
    }
    if options.iter().filter(|item| item.selected).count() > 1 {
        invalid.get_or_insert(InvalidComboBox::MultipleSelected);
    }
    let popup = if open && invalid.is_none() {
        // M5 ActiveDescendant requires Controls to identify the ListBox,
        // not the surrounding Popover's semantic Group.
        let popup = list_box(popup_options).id(popup_id.clone()).into_element();
        let overflow = OverflowStyle::new(OverflowPolicy::Clip, OverflowPolicy::Scroll);
        let viewport = scroll_container(popup, overflow)
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_max_height(LayoutBound::Length(popup_max_height))
                    .with_overflow(overflow),
            )
            .into_element()
            // Overflow alone must not add another real M4 focus target.
            .with_focusability(Focusability::NotFocusable);
        let handler = Rc::clone(&on_request);
        let presentation = SurfacePresentation::new(SurfacePresentationPlacement::new(
            SurfacePresentationSide::Bottom,
        ))
        .with_anchor(SurfacePresentationAnchor::OwnerBounds)
        .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock)
        .with_focus_policy(PresentationFocusPolicy::Preserve)
        .dismiss_on_cancel_or_back(true);
        Some(
            Popover::new(vec![viewport], presentation)
                .on_dismiss(move |_| (*handler.borrow_mut())(ComboBoxRequest::Close))
                .into_element(),
        )
    } else {
        None
    };

    (
        ComboWidget {
            label,
            popup_id,
            options,
            selected_display,
            open,
            enabled: owner_enabled,
            invalid,
            on_request,
            option_activations,
        },
        popup,
    )
}

impl<Action: 'static> View<Action> for ComboBox<Action> {
    fn into_element(self) -> Element<Action> {
        let (widget, popup) = assemble_combo(
            self.label.clone(),
            self.popup_id,
            self.options,
            self.open,
            self.enabled,
            self.common.id.as_ref(),
            self.popup_max_height,
            self.on_request,
        );
        let invalid = widget.invalid.is_some();
        let display = if invalid {
            self.label
        } else {
            widget.selected_display.clone().unwrap_or(self.label)
        };
        let mut children = vec![
            Element::new(ComboLabel { label: display })
                .with_focusability(Focusability::NotFocusable),
        ];
        children.extend(popup);
        let (fields, diagnostics) = self.common.into_authored_fields(
            if self.enabled && !invalid {
                Focusability::Focusable
            } else {
                Focusability::NotFocusable
            },
            None,
        );
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(widget)),
            children,
            diagnostics,
        )
    }
}

/// Authors one application-owned select-only `ComboBox`.
#[must_use]
pub fn combo_box<Action>(
    label: impl Into<String>,
    popup_id: ElementId,
    options: impl IntoIterator<Item = OptionItem<Action>>,
    on_request: impl FnMut(ComboBoxRequest) -> Action + 'static,
) -> ComboBox<Action> {
    ComboBox::new(label, popup_id, options, on_request)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditableComboBoxError {
    /// A composite editor must be a public single-line M10 `TextField`.
    IncompatibleEditor,
}
impl fmt::Display for EditableComboBoxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("editable ComboBox requires a public single-line non-composite TextField")
    }
}
impl std::error::Error for EditableComboBoxError {}

/// One focused M10 `TextField` with `ComboBox` semantics and an `OptionItem` popup.
pub struct EditableComboBox<Action> {
    editor: TextField<Action>,
    label: String,
    popup_id: ElementId,
    options: Vec<OptionItem<Action>>,
    on_request: RequestHandler<Action>,
    popup_max_height: LogicalLength,
    open: bool,
    autocomplete: SemanticAutocomplete,
}
impl<Action> fmt::Debug for EditableComboBox<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EditableComboBox")
            .field("popup_id", &self.popup_id)
            .field("option_count", &self.options.len())
            .field("open", &self.open)
            .field("autocomplete", &self.autocomplete)
            .finish_non_exhaustive()
    }
}
impl<Action> EditableComboBox<Action> {
    #[must_use]
    pub const fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
    /// Limits the existing shared popup's standard scroll viewport.
    #[must_use]
    pub const fn popup_max_height(mut self, height: LogicalLength) -> Self {
        self.popup_max_height = height;
        self
    }
    #[must_use]
    pub const fn autocomplete(mut self, policy: SemanticAutocomplete) -> Self {
        self.autocomplete = policy;
        self
    }
}
impl<Action: 'static> View<Action> for EditableComboBox<Action> {
    fn into_element(self) -> Element<Action> {
        let owner = self.editor.combo_owner_id().cloned();
        let enabled = self.editor.combo_editor_enabled();
        let (widget, popup) = assemble_combo(
            self.label,
            self.popup_id,
            self.options,
            self.open,
            enabled,
            owner.as_ref(),
            self.popup_max_height,
            self.on_request,
        );
        self.editor
            .with_combo_box(widget, popup, self.autocomplete)
            .into_element()
    }
}

/// Composes an existing public single-line M10 `TextField` with standard `OptionItem` values.
///
/// # Errors
/// Rejects multiline, secret or already-composite editor configurations.
pub fn editable_combo_box<Action>(
    editor: TextField<Action>,
    label: impl Into<String>,
    popup_id: ElementId,
    options: impl IntoIterator<Item = OptionItem<Action>>,
    on_request: impl FnMut(ComboBoxRequest) -> Action + 'static,
) -> Result<EditableComboBox<Action>, EditableComboBoxError> {
    if !editor.permits_combo_box() {
        return Err(EditableComboBoxError::IncompatibleEditor);
    }
    Ok(EditableComboBox {
        editor,
        label: label.into(),
        popup_id,
        options: options.into_iter().collect(),
        on_request: Rc::new(RefCell::new(Box::new(on_request))),
        popup_max_height: LogicalLength::from(192_u16),
        open: false,
        autocomplete: SemanticAutocomplete::List,
    })
}
