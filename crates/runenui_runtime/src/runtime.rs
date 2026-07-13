//! Persistent mounted runtime ownership and reconciliation generations.

#![allow(clippy::redundant_pub_crate)]

use core::fmt;

use runenui_core::{Element, ElementKey};

use crate::{
    FocusState, MountedNodeId, RuntimeEvent, Trace, TraceTarget,
    mounted::{MountedTree, TargetStatus},
};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    ReconciliationGenerationExhausted,
    WidgetStatePayloadMismatch,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReconciliationGenerationExhausted => {
                f.write_str("reconciliation generation exhausted")
            }
            Self::WidgetStatePayloadMismatch => {
                f.write_str("mounted widget state payload mismatch")
            }
        }
    }
}
impl std::error::Error for RuntimeError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ReconciliationGeneration(u64);

impl ReconciliationGeneration {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationReport {
    generation: ReconciliationGeneration,
    live_node_count: usize,
    mounted_count: usize,
    updated_count: usize,
    unmounted_count: usize,
    moved_count: usize,
    retained_focus: bool,
    diagnostics: Vec<ReconciliationDiagnostic>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReconciliationDiagnostic {
    DuplicateSiblingKey {
        key: ElementKey,
        parent_path: String,
        old_occurrence_paths: Vec<String>,
        new_occurrence_paths: Vec<String>,
    },
    StatePayloadMismatch {
        path: String,
    },
}

impl fmt::Display for ReconciliationDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSiblingKey {
                key,
                parent_path,
                old_occurrence_paths,
                new_occurrence_paths,
            } => write!(
                f,
                "duplicate sibling key {:?} under {parent_path}; old=[{}], new=[{}]",
                key.as_str(),
                old_occurrence_paths.join(", "),
                new_occurrence_paths.join(", ")
            ),
            Self::StatePayloadMismatch { path } => {
                write!(f, "mounted widget state payload mismatch at {path}")
            }
        }
    }
}

impl ReconciliationReport {
    #[must_use]
    pub const fn generation(&self) -> ReconciliationGeneration {
        self.generation
    }
    #[must_use]
    pub const fn live_node_count(&self) -> usize {
        self.live_node_count
    }
    #[must_use]
    pub const fn mounted_count(&self) -> usize {
        self.mounted_count
    }
    #[must_use]
    pub const fn updated_count(&self) -> usize {
        self.updated_count
    }
    #[must_use]
    pub const fn unmounted_count(&self) -> usize {
        self.unmounted_count
    }
    #[must_use]
    pub const fn moved_count(&self) -> usize {
        self.moved_count
    }
    #[must_use]
    pub const fn retained_focus(&self) -> bool {
        self.retained_focus
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[ReconciliationDiagnostic] {
        &self.diagnostics
    }
}

pub(crate) struct Runtime<State, Action> {
    state: Option<State>,
    pub(crate) tree: MountedTree<Action>,
    trace: Trace,
    focus: FocusState,
    generation: u64,
    report: ReconciliationReport,
    shutdown: bool,
}

impl<State, Action> Runtime<State, Action> {
    pub(crate) fn mount(state: State, root: impl FnOnce(&State) -> Element<Action>) -> Self {
        let transient = root(&state);
        let (tree, reconcile_stats) = MountedTree::mount(transient);
        let mut trace = Trace::new();
        trace.record(RuntimeEvent::Mounted);
        let report = ReconciliationReport {
            generation: ReconciliationGeneration(1),
            live_node_count: tree.live_count(),
            mounted_count: reconcile_stats.mounted,
            updated_count: 0,
            unmounted_count: 0,
            moved_count: 0,
            retained_focus: false,
            diagnostics: reconcile_stats.diagnostics,
        };
        Self {
            state: Some(state),
            tree,
            trace,
            focus: FocusState::new(),
            generation: 1,
            report,
            shutdown: false,
        }
    }

    pub(crate) fn dispatch(
        &mut self,
        action: Action,
        update: impl FnOnce(&mut State, Action),
        root: impl FnOnce(&State) -> Element<Action>,
    ) -> Result<&ReconciliationReport, RuntimeError> {
        self.dispatch_with_target(action, update, root, None)
    }

    pub(crate) fn dispatch_with_target(
        &mut self,
        action: Action,
        update: impl FnOnce(&mut State, Action),
        root: impl FnOnce(&State) -> Element<Action>,
        target: Option<TraceTarget>,
    ) -> Result<&ReconciliationReport, RuntimeError> {
        let next = self.next_generation()?;
        self.trace
            .record_with_target(RuntimeEvent::ActionDispatched, target.clone());
        let app_state = self
            .state
            .as_mut()
            .unwrap_or_else(|| unreachable!("live runtime retains application state"));
        update(app_state, action);
        self.trace
            .record_with_target(RuntimeEvent::StateUpdated, target.clone());
        let transient = root(app_state);
        let previous_focus = self.focus.focused_node().cloned();
        let reconcile_stats = self.tree.reconcile(transient);
        self.generation = next;
        let retained_focus = previous_focus
            .as_ref()
            .is_some_and(|id| self.validate_focus(id));
        if !retained_focus && previous_focus.is_some() {
            self.focus.clear();
            self.trace.record(RuntimeEvent::FocusCleared);
        } else if retained_focus {
            self.trace.record(RuntimeEvent::FocusRetained);
        }
        self.tree.finish_focus_validation();
        self.trace
            .record_with_target(RuntimeEvent::TreeReconciled, target);
        self.report = ReconciliationReport {
            generation: ReconciliationGeneration(next),
            live_node_count: self.tree.live_count(),
            mounted_count: reconcile_stats.mounted,
            updated_count: reconcile_stats.updated,
            unmounted_count: reconcile_stats.unmounted,
            moved_count: reconcile_stats.moved,
            retained_focus,
            diagnostics: reconcile_stats.diagnostics,
        };
        Ok(&self.report)
    }

    pub(crate) fn preflight_reconciliation_generation(&self) -> Result<(), RuntimeError> {
        self.next_generation().map(drop)
    }

    fn next_generation(&self) -> Result<u64, RuntimeError> {
        self.generation
            .checked_add(1)
            .ok_or(RuntimeError::ReconciliationGenerationExhausted)
    }

    fn validate_focus(&mut self, id: &MountedNodeId) -> bool {
        self.tree.target_status(id) == TargetStatus::Live && {
            self.tree
                .activation(id)
                .is_ok_and(|activation| activation.enabled() && activation.is_actionable())
        }
    }

    #[must_use]
    pub(crate) const fn state(&self) -> &State {
        match &self.state {
            Some(state) => state,
            None => unreachable!(),
        }
    }
    #[must_use]
    pub(crate) const fn trace(&self) -> &Trace {
        &self.trace
    }
    #[must_use]
    pub(crate) const fn focus(&self) -> &FocusState {
        &self.focus
    }
    pub(crate) fn set_focus(&mut self, id: MountedNodeId) {
        self.focus.set(id);
    }
    pub(crate) fn clear_focus(&mut self) {
        self.focus.clear();
    }
    #[must_use]
    pub(crate) const fn report(&self) -> &ReconciliationReport {
        &self.report
    }

    pub(crate) fn shutdown(&mut self) {
        if self.shutdown {
            return;
        }
        self.tree.shutdown();
        self.focus.clear();
        self.trace.record(RuntimeEvent::RuntimeShutdown);
        self.shutdown = true;
    }

    pub(crate) fn into_state(mut self) -> State {
        self.shutdown();
        self.state
            .take()
            .unwrap_or_else(|| unreachable!("state is returned exactly once"))
    }

    #[cfg(any(test, feature = "internal-test-seams"))]
    pub(crate) const fn seed_generation_for_test(&mut self, generation: u64) {
        self.generation = generation;
    }
}

impl<State, Action> Drop for Runtime<State, Action> {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{Element, View, text};

    use super::{Runtime, RuntimeError};

    fn root(_: &String) -> Element<()> {
        text("root").key("root").into_element()
    }

    #[test]
    fn generation_exhaustion_aborts_before_application_or_tree_mutation() {
        let mut runtime = Runtime::mount(String::new(), root);
        let mounted = runtime.tree.index().nodes()[0].id().clone();
        runtime.generation = u64::MAX;
        let result = runtime.dispatch((), |state, ()| state.push('x'), root);
        assert_eq!(result, Err(RuntimeError::ReconciliationGenerationExhausted));
        assert_eq!(runtime.state(), "");
        assert_eq!(runtime.tree.index().nodes()[0].id(), &mounted);
    }
}
