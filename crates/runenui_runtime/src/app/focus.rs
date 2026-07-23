use super::{AppRuntime, MountedTreeIndex, UiApp};

impl<App: UiApp> AppRuntime<App> {
    #[must_use]
    pub fn index(&mut self) -> MountedTreeIndex<'_, App::Action> {
        self.runtime.tree.index()
    }
}
