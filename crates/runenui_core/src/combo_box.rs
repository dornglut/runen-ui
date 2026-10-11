//! Select-only `ComboBox` over M4 real focus and M5 active-descendant semantics.
//! All committed values and popup visibility are authored by the application.
use crate::{
    ChildBearingWidget, Element, ElementId, EventContext, EventPhase, Focusability,
    HitContribution, HitContributionContext, KeyboardCompositionState, KeyboardPhase,
    LayoutContainer, LayoutStyle, LogicalKey, OptionItem, Popover, PresentationFocusPolicy,
    PresentationOutsidePointerPolicy, SemanticAction, SemanticCommand, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticPopupKind, SemanticReference,
    SemanticRelationship, SemanticRelationshipKind, SemanticRole, SemanticState,
    SurfacePresentation, SurfacePresentationAnchor, SurfacePresentationPlacement,
    SurfacePresentationSide, UiEvent, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetDiagnostic, WidgetEventOutput, WidgetInvalidation, WidgetMeasure,
    WidgetMeasureInput, WidgetUpdateContext,
    element::{CommonNodeAuthoring, common_node_builder_methods},
    list_box,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Candidate {
    id: ElementId,
    selected: bool,
    enabled: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InvalidComboBox {
    MissingOptionId,
    DuplicateOptionId,
    MultipleSelected,
    PopupIdCollision,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct ComboState {
    label: String,
    popup_id: ElementId,
    options: Vec<Candidate>,
    active: Option<ElementId>,
    open: bool,
    enabled: bool,
    invalid: Option<InvalidComboBox>,
}
struct ComboWidget<Action> {
    label: String,
    popup_id: ElementId,
    options: Vec<Candidate>,
    open: bool,
    enabled: bool,
    invalid: Option<InvalidComboBox>,
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
        if self.open && self.enabled && previous.open && self.invalid.is_none() {
            next.active = previous.active.clone().filter(|id| {
                self.options
                    .iter()
                    .any(|item| &item.id == id && item.enabled)
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
        let Some(active) = &state.active else {
            return WidgetActivationOutput::none();
        };
        let Some((_, callback)) = self.option_activations.iter().find(|(id, _)| id == active)
        else {
            return WidgetActivationOutput::none();
        };
        WidgetActivationOutput::action((*callback.borrow_mut())())
    }
    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if !self.enabled
            || self.invalid.is_some()
            || context.phase() != EventPhase::Target
            || context.default_is_prevented()
        {
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
                LogicalKey::Home => Some(Navigation::First),
                LogicalKey::End => Some(Navigation::Last),
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
        if state.enabled {
            node = node.with_action(SemanticAction::Activate);
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
            InvalidComboBox::MultipleSelected => (
                "runenui.control.combo.multiple-selected",
                "select-only ComboBox permits at most one selected Option",
            ),
            InvalidComboBox::PopupIdCollision => (
                "runenui.control.combo.popup-id-collision",
                "ComboBox popup ID must differ from all option IDs",
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
impl<Action: 'static> View<Action> for ComboBox<Action> {
    fn into_element(self) -> Element<Action> {
        let mut seen = BTreeSet::new();
        let mut options = Vec::new();
        let mut option_activations = Vec::new();
        let mut popup_options = Vec::new();
        let mut invalid = None;
        for mut item in self.options {
            let Some((id, selected, enabled)) = item.combo_proxy_facts() else {
                invalid.get_or_insert(InvalidComboBox::MissingOptionId);
                continue;
            };
            if id == self.popup_id {
                invalid.get_or_insert(InvalidComboBox::PopupIdCollision);
            } else if !seen.insert(id.as_str().to_owned()) {
                invalid.get_or_insert(InvalidComboBox::DuplicateOptionId);
            }
            if let Some(callback) = item.share_combo_activation() {
                option_activations.push((id.clone(), callback));
            }
            options.push(Candidate {
                id,
                selected,
                enabled,
            });
            popup_options.push(if self.enabled {
                item.combo_box_proxy()
            } else {
                item.combo_box_proxy().disabled()
            });
        }
        if options.iter().filter(|item| item.selected).count() > 1 {
            invalid.get_or_insert(InvalidComboBox::MultipleSelected);
        }
        let mut children = vec![
            Element::new(ComboLabel {
                label: self.label.clone(),
            })
            .with_focusability(Focusability::NotFocusable),
        ];
        if self.open && invalid.is_none() {
            // M5 ActiveDescendant requires Controls to identify the ListBox,
            // not the surrounding Popover's semantic Group.
            let popup = list_box(popup_options)
                .id(self.popup_id.clone())
                .into_element();
            let handler = Rc::clone(&self.on_request);
            let presentation = SurfacePresentation::new(SurfacePresentationPlacement::new(
                SurfacePresentationSide::Bottom,
            ))
            .with_anchor(SurfacePresentationAnchor::OwnerBounds)
            .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock)
            .with_focus_policy(PresentationFocusPolicy::Preserve)
            .dismiss_on_cancel_or_back(true);
            children.push(
                Popover::new(vec![popup], presentation)
                    .on_dismiss(move |_| (*handler.borrow_mut())(ComboBoxRequest::Close))
                    .into_element(),
            );
        }
        let (fields, diagnostics) = self.common.into_authored_fields(
            if self.enabled && invalid.is_none() {
                Focusability::Focusable
            } else {
                Focusability::NotFocusable
            },
            None,
        );
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(ComboWidget {
                label: self.label,
                popup_id: self.popup_id,
                options,
                open: self.open,
                enabled: self.enabled,
                invalid,
                on_request: self.on_request,
                option_activations,
            })),
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
