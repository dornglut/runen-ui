//! Standard same-surface presentation controls over the accepted public runtime contract.
//!
//! No standard control owns durable visibility, focus or presentation geometry.
//! An application mounts or removes the subtree in response to ordinary actions.

use core::{fmt, time::Duration};
use std::{cell::RefCell, collections::HashSet, rc::Rc};

use crate::{
    ElementId, EventContext, EventPhase, FocusBoundaryPolicy, FocusEventKind, FocusScope,
    FocusScopePolicy, Focusability, IntoElementId, LayoutContainer, LayoutStyle, PointerBoundaryKind,
    PointerId, PresentationDismissReason, PresentationFocusPolicy,
    PresentationOutsidePointerPolicy, SemanticCommand, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticRole, SemanticState,
    SurfacePresentation, SurfacePresentationAnchor, SurfacePresentationPlacement,
    SurfacePresentationSide, Text, TimerEffect, UiEvent, View, Views, Widget,
    WidgetEventOutput, WidgetInvalidation, WidgetUpdateContext, WorkFamily, WorkKey,
    element::{CommonNodeAuthoring, Element, common_node_builder_methods},
    widget_erasure::WidgetAdapter,
    widget_protocol::ChildBearingWidget,
};

type DismissCallback<Action> = Box<dyn FnMut(PresentationDismissReason) -> Action>;
type ActionFactory<Action> = Rc<RefCell<Box<dyn FnMut() -> Action>>>;

/// A nonmodal popover composed over one ordinary mounted presentation subtree.
///
/// Visibility is application-owned. Pass an existing SurfacePresentation to select
/// anchor, ordered fallback, outside-pointer, Escape and focus policies explicitly.
pub struct Popover<Action> {
    children: Vec<Element<Action>>,
    presentation: SurfacePresentation,
    on_dismiss: Option<DismissCallback<Action>>,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for Popover<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Popover")
            .field("presentation", &self.presentation)
            .field("children", &self.children)
            .field("has_dismiss_callback", &self.on_dismiss.is_some())
            .finish_non_exhaustive()
    }
}

impl<Action> Popover<Action> {
    #[must_use]
    pub fn new(children: impl Views<Action>, presentation: SurfacePresentation) -> Self {
        Self {
            children: children.into_elements(),
            presentation,
            on_dismiss: None,
            common: presentation_common(),
        }
    }

    common_node_builder_methods!();

    #[must_use]
    pub fn on_dismiss(
        mut self,
        callback: impl FnMut(PresentationDismissReason) -> Action + 'static,
    ) -> Self {
        self.on_dismiss = Some(Box::new(callback));
        self
    }
}

/// Same-surface dialog with typed Dialog semantics and the existing FocusScope.
///
/// Its default is modal, center-anchored, Escape-dismissable, outside-blocking
/// and EnterAndRestore-focused. Application state decides whether it is mounted.
pub struct Dialog<Action> {
    label: String,
    children: Vec<Element<Action>>,
    presentation: SurfacePresentation,
    on_dismiss: Option<DismissCallback<Action>>,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for Dialog<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dialog")
            .field("label", &self.label)
            .field("presentation", &self.presentation)
            .field("children", &self.children)
            .field("has_dismiss_callback", &self.on_dismiss.is_some())
            .finish_non_exhaustive()
    }
}

impl<Action> Dialog<Action> {
    #[must_use]
    pub fn new(label: impl Into<String>, children: impl Views<Action>) -> Self {
        Self {
            label: label.into(),
            children: children.into_elements(),
            presentation: SurfacePresentation::new(SurfacePresentationPlacement::new(
                SurfacePresentationSide::Center,
            ))
            .with_anchor(SurfacePresentationAnchor::SurfaceViewport)
            .with_outside_pointer(PresentationOutsidePointerPolicy::Block)
            .modal(true)
            .dismiss_on_cancel_or_back(true)
            .with_focus_policy(PresentationFocusPolicy::EnterAndRestore),
            on_dismiss: None,
            common: presentation_common(),
        }
    }

    common_node_builder_methods!();

    /// Explicitly replaces the complete accepted #341/#342 policy.
    /// Modal semantic state and trapping are always derived from this same fact.
    #[must_use]
    pub fn with_presentation(mut self, presentation: SurfacePresentation) -> Self {
        self.presentation = presentation;
        self
    }

    #[must_use]
    pub fn on_dismiss(
        mut self,
        callback: impl FnMut(PresentationDismissReason) -> Action + 'static,
    ) -> Self {
        self.on_dismiss = Some(Box::new(callback));
        self
    }
}

/// Nonfocusable descriptive presentation. App state owns visibility.
///
/// The described owner must author a standard DescribedBy relation to this
/// Tooltip's exact authored ID; no second semantic identity is generated.
pub struct Tooltip<Action> {
    label: String,
    presentation: SurfacePresentation,
    on_dismiss: Option<DismissCallback<Action>>,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for Tooltip<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tooltip")
            .field("label", &self.label)
            .field("presentation", &self.presentation)
            .finish_non_exhaustive()
    }
}

impl<Action> Tooltip<Action> {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            presentation: SurfacePresentation::new(SurfacePresentationPlacement::new(
                SurfacePresentationSide::Bottom,
            )),
            on_dismiss: None,
            common: presentation_common(),
        }
    }

    common_node_builder_methods!();

    /// Accept placement/fallback and optional Escape dismissal. Tooltip
    /// nonmodality, pointer passthrough and focus preservation are invariant.
    #[must_use]
    pub fn with_presentation(mut self, presentation: SurfacePresentation) -> Self {
        self.presentation = presentation;
        self
    }

    #[must_use]
    pub fn on_dismiss(
        mut self,
        callback: impl FnMut(PresentationDismissReason) -> Action + 'static,
    ) -> Self {
        self.on_dismiss = Some(Box::new(callback));
        self
    }
}

fn presentation_common() -> CommonNodeAuthoring {
    CommonNodeAuthoring {
        layout: LayoutStyle::default().with_container(LayoutContainer::Block),
        ..CommonNodeAuthoring::default()
    }
}

struct PresentationSurfaceWidget<Action> {
    role: SemanticRole,
    name: Option<String>,
    modal: bool,
    on_dismiss: Option<DismissCallback<Action>>,
}

impl<Action> fmt::Debug for PresentationSurfaceWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PresentationSurfaceWidget")
            .field("role", &self.role)
            .field("name", &self.name)
            .field("modal", &self.modal)
            .field("has_dismiss_callback", &self.on_dismiss.is_some())
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PresentationSurfaceState {
    role: SemanticRole,
    name: Option<String>,
    modal: bool,
}

impl<Action> Widget<Action> for PresentationSurfaceWidget<Action> {
    type State = PresentationSurfaceState;

    fn create_state(&self) -> Self::State {
        PresentationSurfaceState {
            role: self.role,
            name: self.name.clone(),
            modal: self.modal,
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        let next = self.create_state();
        if *state != next {
            context.invalidate(WidgetInvalidation::SEMANTICS);
            *state = next;
        }
    }

    fn event(
        &mut self,
        _: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target {
            if let Some(command) = event.as_semantic_command() {
                if let SemanticCommand::PresentationDismiss(reason) = command.command() {
                    if let Some(callback) = self.on_dismiss.as_mut() {
                        context.emit(callback(reason));
                    }
                }
            }
        }
        WidgetEventOutput::none()
    }

    fn semantics(
        &self,
        state: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(state.role);
        if let Some(label) = &state.name {
            node = node.with_name(label.clone());
        }
        if state.role == SemanticRole::Dialog {
            node = node.with_state(SemanticState::ENABLED.with_modal(state.modal));
        }
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}
impl<Action> ChildBearingWidget<Action> for PresentationSurfaceWidget<Action> {}

fn presentation_element<Action: 'static>(
    common: CommonNodeAuthoring,
    presentation: SurfacePresentation,
    widget: PresentationSurfaceWidget<Action>,
    children: Vec<Element<Action>>,
    focus_scope: Option<FocusScope>,
    not_focusable: bool,
) -> Element<Action> {
    let (fields, diagnostics) = common.into_authored_fields(
        if not_focusable {
            Focusability::NotFocusable
        } else {
            Focusability::Automatic
        },
        focus_scope,
    );
    Element::from_authored_parts(fields, Box::new(WidgetAdapter(widget)), children, diagnostics)
        .surface_presentation(presentation)
}

impl<Action: 'static> View<Action> for Popover<Action> {
    fn into_element(self) -> Element<Action> {
        presentation_element(
            self.common,
            self.presentation,
            PresentationSurfaceWidget {
                role: SemanticRole::Group,
                name: None,
                modal: false,
                on_dismiss: self.on_dismiss,
            },
            self.children,
            None,
            false,
        )
    }
}

impl<Action: 'static> View<Action> for Dialog<Action> {
    fn into_element(self) -> Element<Action> {
        let modal = self.presentation.is_modal();
        let scope = modal.then(|| {
            FocusScope::new().with_policy(FocusScopePolicy::new(
                FocusBoundaryPolicy::Trap,
                FocusBoundaryPolicy::Trap,
            ))
        });
        presentation_element(
            self.common,
            self.presentation,
            PresentationSurfaceWidget {
                role: SemanticRole::Dialog,
                name: Some(self.label),
                modal,
                on_dismiss: self.on_dismiss,
            },
            self.children,
            scope,
            false,
        )
    }
}

impl<Action: 'static> View<Action> for Tooltip<Action> {
    fn into_element(self) -> Element<Action> {
        let visible_text = Text::new(self.label.clone()).into_element();
        let config = self
            .presentation
            .modal(false)
            .with_outside_pointer(PresentationOutsidePointerPolicy::Ignore)
            .with_focus_policy(PresentationFocusPolicy::Preserve);
        presentation_element(
            self.common,
            config,
            PresentationSurfaceWidget {
                role: SemanticRole::Tooltip,
                name: Some(self.label),
                modal: false,
                on_dismiss: self.on_dismiss,
            },
            vec![visible_text],
            None,
            true,
        )
    }
}

/// Host-neutral hover/focus intent for a Tooltip; the application conditionally
/// authors Tooltip itself. Show delay uses a keyed owner-local TimerEffect;
/// pointer/focus lifetime and timer cancellation reuse ordinary runtime paths.
pub struct TooltipTrigger<Action> {
    owner: Element<Action>,
    show_delay: Duration,
    show: ActionFactory<Action>,
    hide: ActionFactory<Action>,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for TooltipTrigger<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TooltipTrigger")
            .field("owner", &self.owner)
            .field("show_delay", &self.show_delay)
            .finish_non_exhaustive()
    }
}

impl<Action> TooltipTrigger<Action> {
    #[must_use]
    pub fn new(
        owner: impl View<Action>,
        show_delay: Duration,
        on_show: impl FnMut() -> Action + 'static,
        on_hide: impl FnMut() -> Action + 'static,
    ) -> Self {
        Self {
            owner: owner.into_element(),
            show_delay,
            show: Rc::new(RefCell::new(Box::new(on_show))),
            hide: Rc::new(RefCell::new(Box::new(on_hide))),
            common: CommonNodeAuthoring::default(),
        }
    }

    common_node_builder_methods!();
}

struct TooltipTriggerWidget<Action> {
    delay: Duration,
    show: ActionFactory<Action>,
    hide: ActionFactory<Action>,
}

impl<Action> fmt::Debug for TooltipTriggerWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TooltipTriggerWidget")
            .field("delay", &self.delay)
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
struct TooltipTriggerState {
    hovering: HashSet<PointerId>,
    focused: bool,
}

impl<Action> TooltipTriggerWidget<Action> {
    fn update_intent(
        &mut self,
        was_active: bool,
        active: bool,
        context: &mut EventContext<'_, Action>,
    ) {
        if was_active == active {
            return;
        }
        let show_key = WorkKey::new("runenui.tooltip.show")
            .unwrap_or_else(|_| unreachable!("static work key is valid"));
        let hide_key = WorkKey::new("runenui.tooltip.hide")
            .unwrap_or_else(|_| unreachable!("static work key is valid"));
        if active {
            context.cancel(WorkFamily::Timer, hide_key);
            let show = Rc::clone(&self.show);
            context.timer(TimerEffect::once(self.delay, move || (show.borrow_mut())()).keyed(show_key));
        } else {
            context.cancel(WorkFamily::Timer, show_key);
            context.emit((self.hide.borrow_mut())());
        }
    }
}

impl<Action> Widget<Action> for TooltipTriggerWidget<Action> {
    type State = TooltipTriggerState;

    fn create_state(&self) -> Self::State {
        TooltipTriggerState::default()
    }

    fn update(&self, _: &mut Self::State, _: &mut WidgetUpdateContext<Action>) {}

    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        let was_active = !state.hovering.is_empty() || state.focused;
        if context.phase() == EventPhase::Target {
            if let Some(boundary) = event.as_pointer_boundary() {
                match boundary.kind() {
                    PointerBoundaryKind::Enter => {
                        state.hovering.insert(boundary.pointer_id());
                    }
                    PointerBoundaryKind::Leave => {
                        state.hovering.remove(&boundary.pointer_id());
                    }
                    _ => {}
                }
            }
        }
        if context.phase() == EventPhase::Capture {
            if let Some(focus) = event.as_focus() {
                match focus.kind() {
                    FocusEventKind::In => state.focused = true,
                    FocusEventKind::Out => state.focused = false,
                    _ => {}
                }
            }
        }
        self.update_intent(
            was_active,
            !state.hovering.is_empty() || state.focused,
            context,
        );
        WidgetEventOutput::none()
    }

    fn semantics(
        &self,
        _: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut group = SemanticNodeContribution::primary(SemanticRole::Group);
        if context.has_mounted_children() {
            group = group.with_mounted_children();
        }
        SemanticContribution::single(group)
    }
}
impl<Action> ChildBearingWidget<Action> for TooltipTriggerWidget<Action> {}

impl<Action: 'static> View<Action> for TooltipTrigger<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::NotFocusable, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(TooltipTriggerWidget {
                delay: self.show_delay,
                show: self.show,
                hide: self.hide,
            })),
            vec![self.owner],
            diagnostics,
        )
    }
}

/// Existing semantic DescribedBy relation from an exact authored owner to Tooltip.
#[must_use]
pub fn tooltip_description_reference(id: ElementId) -> crate::SemanticRelationship {
    crate::SemanticRelationship::new(
        crate::SemanticRelationshipKind::DescribedBy,
        crate::SemanticReference::Authored {
            element_id: id,
            semantic_key: None,
        },
    )
}
