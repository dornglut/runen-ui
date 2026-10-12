use runenui_core::{
    Color, EdgeInsets, Element, FlexContainerStyle, FlexDirection, FlexWrap, LayoutBound,
    LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength, OverflowPolicy, OverflowStyle,
    SemanticEditableMode, View, button, checkbox, children, column, row, text, text_field,
};

use crate::app::{GalleryAction, GalleryState, Page};

fn side_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_container(LayoutContainer::Flex(
            FlexContainerStyle::default().with_direction(FlexDirection::Column),
        ))
        .with_width(LayoutDimension::length(LogicalLength::from(140_u16)))
}

fn main_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_container(LayoutContainer::Flex(
            FlexContainerStyle::default().with_direction(FlexDirection::Column),
        ))
        .with_width(LayoutDimension::Fill)
        .with_min_width(LayoutBound::length(LogicalLength::from(220_u16)))
        .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll))
}

fn padding(amount: u16) -> EdgeInsets {
    EdgeInsets::all(LogicalLength::from(amount))
}

fn basics(state: &GalleryState) -> Element<GalleryAction> {
    column(children![
        text("Button").id("gallery.basics.heading"),
        button("Activate")
            .id("gallery.basics.activate")
            .key("gallery.basics.activate")
            .background(Color::rgb(72, 100, 146))
            .padding(padding(10))
            .on_activate(|| GalleryAction::Increment),
        text(format!("Activations: {}", state.presses)).id("gallery.basics.value"),
        button("Disabled")
            .disabled()
            .id("gallery.basics.disabled")
            .background(Color::rgb(65, 65, 75))
            .padding(padding(10)),
        text("Use Tab, Enter, Space, and pointer activation."),
    ])
    .key("gallery.page.basics")
    .gap(12_u16)
    .into_element()
}

fn inputs(state: &GalleryState) -> Element<GalleryAction> {
    let snapshot = state.document_snapshot();
    let selection = state
        .selection
        .bind(snapshot, &state.document)
        .unwrap_or_else(|_| unreachable!("gallery state retains a valid UTF-8 selection"));
    let field = text_field(
        snapshot,
        state.document.clone(),
        selection,
        SemanticEditableMode::SingleLine,
        |intent| GalleryAction::Edit(Box::new(intent)),
    )
    .unwrap_or_else(|_| unreachable!("gallery state retains valid single-line document"));
    column(children![
        text("Checkbox").id("gallery.inputs.heading"),
        checkbox("Enable preview", state.checked)
            .id("gallery.inputs.checkbox")
            .key("gallery.inputs.checkbox")
            .on_activate(|| GalleryAction::Toggle),
        text(format!("Checked: {}", state.checked)),
        text("TextField").id("gallery.inputs.textfield.label"),
        field
            .placeholder("Type a value")
            .labelled_by("gallery.inputs.textfield.label")
            .id("gallery.inputs.textfield")
            .key("gallery.inputs.textfield")
            .background(Color::rgb(50, 57, 73))
            .padding(padding(10)),
        text(format!("Application value: {}", state.document)),
        text("Editing is provided by the ordinary M10 transaction path."),
    ])
    .key("gallery.page.inputs")
    .gap(12_u16)
    .into_element()
}

fn inspector(state: &GalleryState) -> Element<GalleryAction> {
    // Read-only projection of the same application state: never a second
    // editor, selection store, or semantic observation authority.
    column(children![
        text("Live application state").id("gallery.inspector.title"),
        text(format!("Page: {}", state.page.title())).id("gallery.inspector.page"),
        text(format!("Activations: {}", state.presses)).id("gallery.inspector.presses"),
        text(format!("Checked: {}", state.checked)).id("gallery.inspector.checked"),
        text(format!("Text revision: {}", state.document_revision))
            .id("gallery.inspector.revision"),
        text(format!(
            "Selection bytes: {}..{}",
            state.selection.anchor(),
            state.selection.active()
        ))
        .id("gallery.inspector.selection"),
    ])
    .key("gallery.inspector")
    .background(Color::rgb(36, 42, 54))
    .padding(padding(12))
    .gap(8_u16)
    .into_element()
}

pub fn root(state: &GalleryState) -> Element<GalleryAction> {
    let page = match state.page {
        Page::Basics => basics(state),
        Page::Inputs => inputs(state),
    };
    row(children![
        column(children![
            text("RunenUI Gallery").id("gallery.title"),
            button("Basics")
                .id("gallery.nav.basics")
                .key("gallery.nav.basics")
                .on_activate(|| GalleryAction::Navigate(Page::Basics)),
            button("Inputs")
                .id("gallery.nav.inputs")
                .key("gallery.nav.inputs")
                .on_activate(|| GalleryAction::Navigate(Page::Inputs)),
        ])
        .key("gallery.navigation")
        .with_layout(side_layout())
        .gap(12_u16),
        column(children![
            text(state.page.title()).id("gallery.current.page"),
            page,
            inspector(state),
        ])
        .key("gallery.preview")
        .with_layout(main_layout())
        .gap(12_u16),
    ])
    .id("gallery.root")
    .key("gallery.root")
    .with_layout(
        LayoutStyle::default()
            .with_container(LayoutContainer::Flex(
                FlexContainerStyle::default()
                    .with_direction(FlexDirection::Row)
                    .with_wrap(FlexWrap::Wrap),
            ))
            .with_width(LayoutDimension::Fill)
            .with_height(LayoutDimension::Fill)
            .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
    )
    .foreground(Color::rgb(238, 240, 246))
    .background(Color::rgb(24, 28, 36))
    .padding(padding(16))
    .gap(16_u16)
    .into_element()
}
