//! Public-contract Gallery application and its initial interactive pages.
//! Native host and full M11 catalog acceptance are tracked separately in #457/#455.

mod app;
mod ui;

pub use app::{GalleryAction, GalleryApp, GalleryState, Page};

#[cfg(test)]
mod tests {
    use super::*;
    use runenui_core::{
        ElementId, LogicalLength, SemanticCommand, StyleEnvironment,
    };
    use runenui_runtime::{
        AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext,
    };

    fn settle(app: &mut AppRuntime<GalleryApp>) {
        app.pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|_| unreachable!("bounded gallery initial work must fit"));
    }

    fn id(value: &str) -> ElementId {
        ElementId::new(value).unwrap_or_else(|_| unreachable!("static gallery id is valid"))
    }

    #[test]
    fn gallery_basics_button_changes_application_owned_count() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.basics.activate"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("gallery button is mounted"));
        settle(&mut app);
        assert_eq!(app.state().presses, 1);
    }

    #[test]
    fn gallery_navigation_and_checkbox_use_standard_actions() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.nav.inputs"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("gallery inputs page navigation exists"));
        settle(&mut app);
        assert_eq!(app.state().page, Page::Inputs);
        app.submit_automation_command(id("gallery.inputs.checkbox"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("gallery checkbox is mounted"));
        settle(&mut app);
        assert!(app.state().checked);
    }

    #[test]
    fn gallery_publishes_one_public_semantic_tree() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        let style = StyleEnvironment::default();
        let context = SurfaceBuildContext::tight(
            &style,
            LogicalSize::new(LogicalLength::from(760_u16), LogicalLength::from(520_u16)),
        );
        let published = app.publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("gallery shell publication is valid"));
        let names: Vec<_> = published.semantic_publication().snapshot().nodes()
            .iter().filter_map(|node| node.name().map(str::to_owned)).collect();
        assert!(names.iter().any(|name| name == "Activate"));
        assert!(names.iter().any(|name| name == "Basics"));
    }
}
