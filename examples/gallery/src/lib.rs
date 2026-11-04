//! Public-contract Gallery application and its initial interactive pages.
//! The headless shell is not a native host or a complete standard-control catalog.

mod app;
mod ui;

pub use app::{GalleryAction, GalleryApp, GalleryState, Page};
pub use ui::style_environment;

#[cfg(test)]
mod tests {
    use super::*;
    use runenui_core::{
        CommittedTextEvent, ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState,
        KeyboardEvent, KeyboardPhase, LogicalKey, LogicalLength, LogicalPoint, PhysicalKey,
        PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
        SemanticCommand, SemanticRelationshipKind, SemanticRole, StyleInteractionState,
        StyleRecipeId,
    };
    use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

    fn settle(app: &mut AppRuntime<GalleryApp>) {
        app.pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|_| unreachable!("bounded gallery initial work must fit"));
    }

    fn id(value: &str) -> ElementId {
        ElementId::new(value).unwrap_or_else(|_| unreachable!("static gallery id is valid"))
    }

    #[test]
    fn gallery_focus_style_is_keyboard_visible_without_permanent_outline() {
        let style = style_environment();
        let id = StyleRecipeId::from_static("gallery.focusable")
            .unwrap_or_else(|_| unreachable!("static Gallery recipe id is valid"));
        let recipe = style
            .theme()
            .recipe(&id)
            .unwrap_or_else(|| unreachable!("Gallery registers its focus recipe"));
        assert!(recipe.base().outline().is_none());
        assert!(
            recipe
                .interaction(StyleInteractionState::FocusVisible)
                .and_then(|properties| properties.outline())
                .is_some()
        );
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
    fn gallery_text_field_commits_utf8_through_standard_m10() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.nav.inputs"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("navigation is mounted"));
        settle(&mut app);
        app.submit_automation_command(
            id("gallery.inputs.textfield"),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("gallery TextField accepts focus"));
        settle(&mut app);
        app.submit_text(
            CommittedTextEvent::new("Hé", None)
                .unwrap_or_else(|_| unreachable!("fixture committed text is valid")),
        )
        .unwrap_or_else(|_| unreachable!("focused gallery TextField accepts committed text"));
        settle(&mut app);
        assert_eq!(app.state().document, "Hé");
        assert_eq!(app.state().document_revision, 2);
        assert_eq!(app.state().selection.active(), "Hé".len());
    }

    #[test]
    fn gallery_edit_queue_accepts_consecutive_commits_with_m10_predecessors() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.nav.inputs"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("gallery inputs navigation exists"));
        settle(&mut app);
        app.submit_automation_command(
            id("gallery.inputs.textfield"),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("gallery TextField accepts focus"));
        settle(&mut app);
        for value in ["x", "y"] {
            app.submit_text(
                CommittedTextEvent::new(value, None)
                    .unwrap_or_else(|_| unreachable!("fixture edit text is nonempty")),
            )
            .unwrap_or_else(|_| unreachable!("M10 accepts the bounded pending edit"));
        }
        settle(&mut app);
        assert_eq!(app.state().document, "xy");
        assert_eq!(app.state().document_revision, 3);
        assert_eq!(app.state().selection.active(), 2);
    }

    #[test]
    fn gallery_undo_redo_round_trip_preserves_utf8_document_and_selection() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.nav.inputs"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("inputs navigation exists"));
        settle(&mut app);
        app.submit_automation_command(
            id("gallery.inputs.textfield"),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("TextField is focusable"));
        settle(&mut app);
        app.submit_text(
            CommittedTextEvent::new("Hé", None)
                .unwrap_or_else(|_| unreachable!("UTF-8 input is valid")),
        )
        .unwrap_or_else(|_| unreachable!("TextField admits committed text"));
        settle(&mut app);
        assert_eq!(app.state().document, "Hé");

        app.submit_automation_command(id("gallery.inputs.textfield"), SemanticCommand::Undo)
            .unwrap_or_else(|_| unreachable!("TextField supports undo"));
        settle(&mut app);
        assert_eq!(app.state().document, "");
        assert_eq!(app.state().document_revision, 3);
        assert_eq!(app.state().selection.active(), 0);

        app.submit_automation_command(id("gallery.inputs.textfield"), SemanticCommand::Redo)
            .unwrap_or_else(|_| unreachable!("TextField supports redo"));
        settle(&mut app);
        assert_eq!(app.state().document, "Hé");
        assert_eq!(app.state().document_revision, 4);
        assert_eq!(app.state().selection.active(), "Hé".len());
    }

    #[test]
    fn gallery_new_edit_invalidates_redo_and_empty_history_is_noop() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.nav.inputs"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("inputs navigation exists"));
        settle(&mut app);
        app.submit_automation_command(
            id("gallery.inputs.textfield"),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("TextField is focusable"));
        settle(&mut app);
        app.submit_automation_command(id("gallery.inputs.textfield"), SemanticCommand::Undo)
            .unwrap_or_else(|_| unreachable!("empty undo is routed"));
        settle(&mut app);
        assert_eq!(app.state().document_revision, 1);

        app.submit_text(
            CommittedTextEvent::new("é", None)
                .unwrap_or_else(|_| unreachable!("fixture text is valid")),
        )
        .unwrap_or_else(|_| unreachable!("input is admitted"));
        settle(&mut app);
        app.submit_automation_command(id("gallery.inputs.textfield"), SemanticCommand::Undo)
            .unwrap_or_else(|_| unreachable!("undo is routed"));
        settle(&mut app);
        assert_eq!(app.state().document, "");
        app.submit_text(
            CommittedTextEvent::new("Q", None)
                .unwrap_or_else(|_| unreachable!("replacement text is valid")),
        )
        .unwrap_or_else(|_| unreachable!("new input is admitted"));
        settle(&mut app);
        let revision = app.state().document_revision;
        app.submit_automation_command(id("gallery.inputs.textfield"), SemanticCommand::Redo)
            .unwrap_or_else(|_| unreachable!("empty redo is routed"));
        settle(&mut app);
        assert_eq!(app.state().document, "Q");
        assert_eq!(app.state().document_revision, revision);
    }

    #[test]
    fn gallery_navigation_retires_text_input_capability_with_its_page() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.nav.inputs"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("gallery inputs navigation exists"));
        settle(&mut app);
        app.submit_automation_command(
            id("gallery.inputs.textfield"),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("gallery TextField accepts focus"));
        settle(&mut app);
        assert!(app.focused_text_input_capability().accepts_committed_text());
        app.submit_automation_command(id("gallery.nav.basics"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("gallery basics navigation exists"));
        settle(&mut app);
        assert_eq!(app.state().page, Page::Basics);
        assert!(!app.focused_text_input_capability().accepts_committed_text());
    }

    #[test]
    fn gallery_text_field_has_a_resolved_accessible_label() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.nav.inputs"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("gallery input navigation exists"));
        settle(&mut app);
        let style = style_environment();
        let context = SurfaceBuildContext::tight(
            &style,
            LogicalSize::new(LogicalLength::from(760_u16), LogicalLength::from(520_u16)),
        );
        let published = app
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("gallery input page publishes"));
        let nodes = published.semantic_publication().snapshot().nodes();
        let label = nodes
            .iter()
            .find(|node| node.role() == SemanticRole::Text && node.name() == Some("TextField"))
            .unwrap_or_else(|| unreachable!("the authored visible label is published"));
        let field = nodes
            .iter()
            .find(|node| node.role() == SemanticRole::EditableText)
            .unwrap_or_else(|| unreachable!("the public TextField is published"));
        assert!(field.relationships().iter().any(|relation| {
            relation.kind() == SemanticRelationshipKind::LabelledBy
                && relation.target() == label.id()
        }));
    }

    #[test]
    fn gallery_disabled_button_cannot_change_application_count() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        let _ =
            app.submit_automation_command(id("gallery.basics.disabled"), SemanticCommand::Activate);
        settle(&mut app);
        assert_eq!(app.state().presses, 0);
    }

    #[test]
    fn gallery_inspector_reads_current_application_state_after_actions() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.basics.activate"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("enabled Button is actionable"));
        settle(&mut app);
        app.submit_automation_command(id("gallery.nav.inputs"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("Inputs page can be selected"));
        settle(&mut app);
        app.submit_automation_command(id("gallery.inputs.checkbox"), SemanticCommand::Activate)
            .unwrap_or_else(|_| unreachable!("Checkbox is actionable"));
        settle(&mut app);
        app.submit_automation_command(
            id("gallery.inputs.textfield"),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("TextField can receive focus"));
        settle(&mut app);
        app.submit_text(
            CommittedTextEvent::new("é", None)
                .unwrap_or_else(|_| unreachable!("fixture is valid UTF-8")),
        )
        .unwrap_or_else(|_| unreachable!("TextField can commit UTF-8"));
        settle(&mut app);

        let style = style_environment();
        let context = SurfaceBuildContext::tight(
            &style,
            LogicalSize::new(LogicalLength::from(760_u16), LogicalLength::from(520_u16)),
        );
        let publication = app
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("the Gallery publishes the live inspector"));
        let names: Vec<_> = publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .filter_map(|node| node.name())
            .collect();
        for expected in [
            "Live application state",
            "Page: Inputs",
            "Activations: 1",
            "Checked: true",
            "Text revision: 2",
            "Selection bytes: 2..2",
        ] {
            assert!(
                names.contains(&expected),
                "missing live state readout: {expected}"
            );
        }
    }

    #[test]
    fn gallery_navigation_and_inspector_remain_mounted_on_narrow_surface() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        let style = style_environment();
        for (width, height) in [(760_u16, 520_u16), (320_u16, 480_u16)] {
            let context = SurfaceBuildContext::tight(
                &style,
                LogicalSize::new(LogicalLength::from(width), LogicalLength::from(height)),
            );
            let publication = app
                .publish_surface(&context)
                .unwrap_or_else(|_| unreachable!("responsive Gallery surface publishes"));
            let nodes = publication.frame().nodes();
            for authored in [
                "gallery.nav.basics",
                "gallery.nav.inputs",
                "gallery.basics.activate",
                "gallery.inspector.title",
            ] {
                assert!(
                    nodes
                        .iter()
                        .any(|node| node.authored_id() == Some(&id(authored))),
                    "missing responsive Gallery node: {authored} at {width}x{height}"
                );
            }
        }
    }

    #[test]
    fn gallery_navigation_has_distinct_active_and_idle_recipes() {
        let style = style_environment();
        let selected = StyleRecipeId::from_static("gallery.nav.selected")
            .unwrap_or_else(|_| unreachable!("static active navigation recipe is valid"));
        let idle = StyleRecipeId::from_static("gallery.nav.default")
            .unwrap_or_else(|_| unreachable!("static idle navigation recipe is valid"));
        let active_recipe = style
            .theme()
            .recipe(&selected)
            .unwrap_or_else(|| unreachable!("active navigation recipe is registered"));
        let idle_recipe = style
            .theme()
            .recipe(&idle)
            .unwrap_or_else(|| unreachable!("idle navigation recipe is registered"));
        assert_ne!(
            active_recipe.base().background(),
            idle_recipe.base().background(),
            "selected page must remain visually distinguishable"
        );
        for recipe in [active_recipe, idle_recipe] {
            assert!(
                recipe
                    .interaction(StyleInteractionState::FocusVisible)
                    .and_then(|properties| properties.outline())
                    .is_some(),
                "navigation focus must stay keyboard visible"
            );
        }
    }

    #[test]
    fn gallery_shell_reflows_navigation_and_inspector_without_discarding_controls() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        let style = style_environment();
        for (width, height) in [(1200_u16, 720_u16), (320_u16, 480_u16)] {
            let context = SurfaceBuildContext::tight(
                &style,
                LogicalSize::new(LogicalLength::from(width), LogicalLength::from(height)),
            );
            let publication = app
                .publish_surface(&context)
                .unwrap_or_else(|_| unreachable!("Gallery publishes at both viewport widths"));
            let bounds = |authored: &str| {
                publication
                    .frame()
                    .nodes()
                    .iter()
                    .find(|node| node.authored_id() == Some(&id(authored)))
                    .unwrap_or_else(|| unreachable!("missing authored Gallery node: {authored}"))
                    .bounds()
            };
            let nav = bounds("gallery.navigation");
            let workspace = bounds("gallery.preview");
            let sample = bounds("gallery.card.basics");
            let state = bounds("gallery.inspector");
            if width == 1200 {
                assert!(
                    workspace.x() > nav.x(),
                    "wide viewport should put navigation beside the workspace"
                );
                assert!(
                    state.x() > sample.x(),
                    "wide viewport should put the inspector beside the sample"
                );
            } else {
                assert!(
                    workspace.y() > nav.y(),
                    "narrow viewport should wrap the workspace below navigation"
                );
                assert!(
                    state.y() > sample.y(),
                    "narrow viewport should wrap the inspector below the sample"
                );
            }
        }
    }

    #[test]
    fn gallery_native_neutral_keyboard_enter_activates_focused_button() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        app.submit_automation_command(id("gallery.basics.activate"), SemanticCommand::RequestFocus)
            .unwrap_or_else(|_| unreachable!("gallery button admits programmatic focus"));
        settle(&mut app);
        app.submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Enter,
            LogicalKey::Enter,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("gallery focused button admits neutral Enter"));
        settle(&mut app);
        assert_eq!(app.state().presses, 1);
    }

    #[test]
    fn gallery_native_neutral_pointer_click_activates_published_button() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        let style = style_environment();
        let context = SurfaceBuildContext::tight(
            &style,
            LogicalSize::new(LogicalLength::from(760_u16), LogicalLength::from(520_u16)),
        );
        let published = app
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("gallery basics page publishes"));
        let node = published
            .frame()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&id("gallery.basics.activate")))
            .unwrap_or_else(|| unreachable!("button has a published visual node"));
        let bounds = node.bounds();
        // The interactive Button has rounded corners: sample its center, not a
        // corner pixel that may legitimately lie outside its hit shape.
        let point = LogicalPoint::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() / 2.0,
        )
        .unwrap_or_else(|_| unreachable!("published button center is finite"));
        assert_eq!(
            published.hit_test_scene().target_at(point),
            Some(node.id()),
            "the exact public visual Button must be hit-testable"
        );
        let pointer_id = PointerId::new(1)
            .unwrap_or_else(|| unreachable!("fixture pointer identity is non-zero"));
        let down = PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            point,
            published.input_context().clone(),
        )
        .with_buttons(PointerButtons::new([PointerButton::Primary]))
        .with_changed_button(PointerButton::Primary);
        app.submit_pointer(down)
            .unwrap_or_else(|_| unreachable!("primary pointer down enters runtime"));
        settle(&mut app);
        assert_eq!(app.state().presses, 0, "pointer-down alone never activates");

        let next = app
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("post-press pointer context publishes"));
        let up = PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            point,
            next.input_context().clone(),
        )
        .with_changed_button(PointerButton::Primary);
        app.submit_pointer(up)
            .unwrap_or_else(|_| unreachable!("primary pointer release enters runtime"));
        settle(&mut app);
        assert_eq!(app.state().presses, 1);
    }

    #[test]
    fn gallery_publishes_one_public_semantic_tree() {
        let mut app = AppRuntime::<GalleryApp>::mount(GalleryState::default());
        settle(&mut app);
        let style = style_environment();
        let context = SurfaceBuildContext::tight(
            &style,
            LogicalSize::new(LogicalLength::from(760_u16), LogicalLength::from(520_u16)),
        );
        let published = app
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("gallery shell publication is valid"));
        let names: Vec<_> = published
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .filter_map(|node| node.name().map(str::to_owned))
            .collect();
        assert!(names.iter().any(|name| name == "Activate"));
        assert!(names.iter().any(|name| name == "Basics"));
    }
}
