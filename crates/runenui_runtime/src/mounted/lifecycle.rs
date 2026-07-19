use runenui_core::{WidgetUnmountContext, WidgetUnmountReason};

use crate::ReconciliationDiagnostic;

use super::{
    MountedNodeId,
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
        before_unmount: &mut dyn FnMut(&MountedNodeId),
    ) {
        let Some(children) = self.node(id).map(|node| node.children.clone()) else {
            return;
        };
        for (position, child) in children.into_iter().enumerate() {
            self.unmount_subtree(
                &child,
                reason,
                &format!("{path}/{position}"),
                stats,
                before_unmount,
            );
        }
        before_unmount(id);
        stats.unmounted_owners.push(id.clone());
        let (slot, generation) = self
            .runtime
            .__runtime_mounted_parts(id)
            .unwrap_or_else(|| unreachable!("unmount target belongs to mounted tree"));
        let mismatch = if let Some(node) = self.arena.get_mut(slot as usize, generation) {
            let mut context = WidgetUnmountContext::__runtime_new(reason);
            state_is_corrupted(node) || node.widget.unmount(&mut node.state, &mut context).is_err()
        } else {
            return;
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
    }

    pub(crate) fn shutdown(&mut self) -> ReconcileStats<Action> {
        let mut stats = ReconcileStats::default();
        if self.shutdown {
            return stats;
        }
        if let Some(root) = self.root.take() {
            let mut before_unmount = |_: &MountedNodeId| {};
            self.unmount_subtree(
                &root,
                WidgetUnmountReason::RuntimeShutdown,
                "root",
                &mut stats,
                &mut before_unmount,
            );
        }
        self.shutdown = true;
        stats
    }
}
