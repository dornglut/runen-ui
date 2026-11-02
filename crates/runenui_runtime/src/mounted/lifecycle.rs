use runenui_core::{WidgetUnmountContext, WidgetUnmountReason};

use crate::ReconciliationDiagnostic;

use super::{
    MountedNodeId,
    reconcile::ReconciliationApplyError,
    node::state_is_corrupted,
    tree::{MountedTree, ReconcileStats},
};

impl<Action> MountedTree<Action> {
    pub(super) fn unmount_subtree(
        &mut self,
        id: &MountedNodeId,
        reason: WidgetUnmountReason,
        path: &str,
        stats: &mut ReconcileStats<Action>,
        before_unmount: &mut dyn FnMut(&MountedNodeId) -> Result<(), ()>,
    ) -> Result<(), ReconciliationApplyError> {
        let Some(children) = self.node(id).map(|node| node.children.clone()) else {
            return Ok(());
        };
        for (position, child) in children.into_iter().enumerate() {
            self.unmount_subtree(
                &child,
                reason,
                &format!("{path}/{position}"),
                stats,
                before_unmount,
            )?;
        }

        let bindings = self
            .node_mut(id)
            .map(|node| core::mem::take(&mut node.semantic_bindings))
            .unwrap_or_default();
        let runtime = self.runtime.clone();
        if self
            .semantic_store
            .revoke_owner(&runtime, id, &bindings)
            .is_err()
        {
            debug_assert!(
                false,
                "semantic-store corruption fallback purged the unmounted owner"
            );
        }

        before_unmount(id).map_err(|()| ReconciliationApplyError)?;
        stats.unmounted_owners.push(id.clone());
        let (slot, generation) = self
            .runtime
            .__runtime_mounted_parts(id)
            .unwrap_or_else(|| unreachable!("unmount target belongs to mounted tree"));
        let mismatch = if let Some(node) = self.arena.get_mut(slot as usize, generation) {
            let mut context = WidgetUnmountContext::__runtime_new(reason);
            state_is_corrupted(node) || node.widget.unmount(&mut node.state, &mut context).is_err()
        } else {
            return Ok(());
        };
        if mismatch {
            stats
                .diagnostics
                .push(ReconciliationDiagnostic::StatePayloadMismatch {
                    path: path.to_owned(),
                });
        }
        if let Some(node) = self.arena.remove(slot as usize, generation) {
            drop(node);
            stats.unmounted += 1;
        }
        Ok(())
    }

    pub(crate) fn shutdown(&mut self) -> ReconcileStats<Action> {
        let mut stats = ReconcileStats::default();
        if self.shutdown {
            return stats;
        }
        if let Some(root) = self.root.take() {
            let mut before_unmount = |_: &MountedNodeId| Ok(());
            self.unmount_subtree(
                &root,
                WidgetUnmountReason::RuntimeShutdown,
                "root",
                &mut stats,
                &mut before_unmount,
            )
            .unwrap_or_else(|_| unreachable!("shutdown cannot refuse before-unmount cleanup"));
        }
        self.shutdown = true;
        stats
    }
}

// A mounted tree owns its hooks even when runtime construction is refused.
// Explicit runtime shutdown reaches this same idempotent authority first.
impl<Action> Drop for MountedTree<Action> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
