use runenui_core::__runtime::RuntimeNamespace;

use super::MountedTree;

impl<Action> MountedTree<Action> {
    pub(crate) fn runtime_namespace(&self) -> RuntimeNamespace {
        self.runtime.clone()
    }
}
