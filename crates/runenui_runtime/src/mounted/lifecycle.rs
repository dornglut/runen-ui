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
        stats: &mut ReconcileStats,
    ) {
        let Some(children) = self.node(id).map(|node| node.children.clone()) else {
            return;
        };
        for (position, child) in children.into_iter().enumerate() {
            self.unmount_subtree(&child, reason, &format!("{path}/{position}"), stats);
        }
        let mismatch = if let Some(node) = self.arena.get_mut(id.slot, id.generation) {
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
        if let Some(node) = self.arena.remove(id.slot, id.generation) {
            drop(node);
            stats.unmounted += 1;
        }
    }

    pub(crate) fn shutdown(&mut self) -> ReconcileStats {
        let mut stats = ReconcileStats::default();
        if self.shutdown {
            return stats;
        }
        if let Some(root) = self.root.take() {
            self.unmount_subtree(
                &root,
                WidgetUnmountReason::RuntimeShutdown,
                "root",
                &mut stats,
            );
        }
        self.shutdown = true;
        stats
    }
}
