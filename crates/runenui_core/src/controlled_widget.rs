//! EXPERIMENTAL #475: closed declarative widget authoring. Not an accepted API.
use crate::{
    ChildBearingWidget, EventPhase, Widget, WidgetEventOutput, WidgetInvalidation,
    WidgetUpdateContext,
};
use std::{any::Any, fmt, rc::Rc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlledNoticeKind {
    CompositionCancel,
    FocusOut,
}

pub struct ControlledNotice<Action> {
    pub kind: ControlledNoticeKind,
    pub phase: EventPhase,
    pub action: Action,
}

pub struct ControlledMountedPlan<Action> {
    pub revision: u64,
    pub update_actions: Vec<Action>,
    pub notices: Vec<ControlledNotice<Action>>,
}

/// The owned non-Clone resource remains in one authoritative mounted state.
/// Only the scalar revision is written by the controlled commit interpreter.
pub struct ControlledRetainedState {
    revision: u64,
    resource: Box<dyn Any>,
}
impl ControlledRetainedState {
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
    #[doc(hidden)]
    pub const fn __runtime_set_revision(&mut self, revision: u64) {
        self.revision = revision;
    }
    #[must_use]
    pub fn resource(&self) -> &dyn Any {
        self.resource.as_ref()
    }
}

/// Only this framework-owned widget type can supply interpreted controlled
/// operations. Authoring supplies concrete owned actions, not callback factories.
pub struct ControlledDeclarativeWidget<Action> {
    initial_revision: u64,
    next_revision: u64,
    make_resource: Rc<dyn Fn() -> Box<dyn Any>>,
    update_actions: Vec<Action>,
    notices: Vec<ControlledNotice<Action>>,
}
impl<Action> fmt::Debug for ControlledDeclarativeWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControlledDeclarativeWidget")
            .field("initial_revision", &self.initial_revision)
            .field("next_revision", &self.next_revision)
            .finish_non_exhaustive()
    }
}
impl<Action> ControlledDeclarativeWidget<Action> {
    pub fn new(revision: u64, make_resource: impl Fn() -> Box<dyn Any> + 'static) -> Self {
        Self {
            initial_revision: 0,
            next_revision: revision,
            make_resource: Rc::new(make_resource),
            update_actions: Vec::new(),
            notices: Vec::new(),
        }
    }
    #[must_use]
    pub fn on_update(mut self, action: Action) -> Self {
        self.update_actions.push(action);
        self
    }
    #[must_use]
    pub fn on_notice(
        mut self,
        kind: ControlledNoticeKind,
        phase: EventPhase,
        action: Action,
    ) -> Self {
        self.notices.push(ControlledNotice {
            kind,
            phase,
            action,
        });
        self
    }
}
impl<Action> Widget<Action> for ControlledDeclarativeWidget<Action> {
    type State = ControlledRetainedState;
    fn create_state(&self) -> Self::State {
        ControlledRetainedState {
            revision: self.initial_revision,
            resource: (self.make_resource)(),
        }
    }
    fn update(&self, _state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        // The opt-in transaction, rather than arbitrary widget code, owns
        // the controlled state transition and output publication.
        context.invalidate(WidgetInvalidation::ALL);
    }
    fn controlled_take_plan(&mut self) -> Option<ControlledMountedPlan<Action>> {
        Some(ControlledMountedPlan {
            revision: self.next_revision,
            update_actions: std::mem::take(&mut self.update_actions),
            notices: std::mem::take(&mut self.notices),
        })
    }
    fn event(
        &mut self,
        _state: &mut Self::State,
        _event: &crate::UiEvent,
        _context: &mut crate::EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        WidgetEventOutput::none()
    }
}
impl<Action> ChildBearingWidget<Action> for ControlledDeclarativeWidget<Action> {}
