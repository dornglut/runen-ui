use runenui_core::{
    Brush, Color, EdgeInsets, Element, FlexBasis, FlexContainerStyle, FlexDirection, FlexItemStyle,
    FlexWrap, FontFamily, FontWeight, GenericFontFamily, ItemAlignment, LayoutBound,
    LayoutContainer, LayoutDimension, LayoutFactor, LayoutStyle, LogicalLength, Outline,
    OverflowPolicy, OverflowStyle, Radius, SemanticEditableMode, StrokeStyle, StyleEnvironment,
    StyleInteractionState, StyleProperties, StyleRecipe, StyleRecipeId, StyleTheme, StyleTokens,
    Typography, View, button, checkbox, children, column, row, text, text_field,
};

use crate::app::{GalleryAction, GalleryState, Page};

const BACKGROUND: Color = Color::rgb(16, 20, 29);
const SIDEBAR: Color = Color::rgb(24, 31, 43);
const PANEL: Color = Color::rgb(30, 39, 53);
const INSET: Color = Color::rgb(39, 50, 67);
const FOREGROUND: Color = Color::rgb(237, 242, 249);
const MUTED: Color = Color::rgb(172, 185, 203);
const ACCENT: Color = Color::rgb(75, 111, 183);
const ACCENT_HOVER: Color = Color::rgb(89, 129, 205);
const ACCENT_ACTIVE: Color = Color::rgb(61, 92, 152);

fn recipe_id(name: &'static str) -> StyleRecipeId {
    StyleRecipeId::from_static(name)
        .unwrap_or_else(|_| unreachable!("static Gallery style recipe is valid"))
}

fn focus_recipe_id() -> StyleRecipeId {
    recipe_id("gallery.focusable")
}

fn nav_recipe_id(selected: bool) -> StyleRecipeId {
    if selected {
        recipe_id("gallery.nav.selected")
    } else {
        recipe_id("gallery.nav.default")
    }
}

fn primary_recipe_id() -> StyleRecipeId {
    recipe_id("gallery.button.primary")
}

fn disabled_recipe_id() -> StyleRecipeId {
    recipe_id("gallery.button.disabled")
}

fn input_recipe_id() -> StyleRecipeId {
    recipe_id("gallery.input.text")
}

fn padding(value: u16) -> EdgeInsets {
    EdgeInsets::all(LogicalLength::from(value))
}

fn radius(value: u16) -> Radius {
    Radius::all(LogicalLength::from(value))
}

fn typography(size: u16, bold: bool) -> Typography {
    Typography::new(
        FontFamily::generic(GenericFontFamily::SansSerif),
        LogicalLength::from(size),
    )
    .with_weight(if bold {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    })
}

fn focus_outline() -> Outline {
    Outline::new(
        Brush::solid(FOREGROUND),
        StrokeStyle::new(LogicalLength::from(2_u16)),
    )
}

fn interactive_recipe(base: Color, hover: Color, active: Color, padding_size: u16) -> StyleRecipe {
    let mut recipe = StyleRecipe::new(
        StyleProperties::EMPTY
            .with_background(base)
            .with_foreground(FOREGROUND)
            .with_padding(padding(padding_size))
            .with_radius(radius(9))
            .with_typography(typography(14, true)),
    );
    recipe
        .define_interaction(
            StyleInteractionState::Hover,
            StyleProperties::EMPTY.with_background(hover),
        )
        .unwrap_or_else(|_| unreachable!("Gallery declares hover once per recipe"));
    recipe
        .define_interaction(
            StyleInteractionState::Active,
            StyleProperties::EMPTY.with_background(active),
        )
        .unwrap_or_else(|_| unreachable!("Gallery declares active once per recipe"));
    recipe
        .define_interaction(
            StyleInteractionState::FocusVisible,
            StyleProperties::EMPTY.with_outline(focus_outline()),
        )
        .unwrap_or_else(|_| unreachable!("Gallery declares focus once per recipe"));
    recipe
}

/// The same app-owned recipes are used for native and deterministic publications.
#[must_use]
pub fn style_environment() -> StyleEnvironment {
    let mut focus = StyleRecipe::new(StyleProperties::EMPTY);
    focus
        .define_interaction(
            StyleInteractionState::FocusVisible,
            StyleProperties::EMPTY.with_outline(focus_outline()),
        )
        .unwrap_or_else(|_| unreachable!("Gallery declares one focus-visible state"));

    let mut input = StyleRecipe::new(
        StyleProperties::EMPTY
            .with_background(INSET)
            .with_padding(padding(12))
            .with_radius(radius(8)),
    );
    input
        .define_interaction(
            StyleInteractionState::FocusVisible,
            StyleProperties::EMPTY.with_outline(focus_outline()),
        )
        .unwrap_or_else(|_| unreachable!("Gallery declares one input focus state"));

    let mut theme = StyleTheme::new(StyleTokens::new());
    for (id, recipe) in [
        (focus_recipe_id(), focus),
        (
            nav_recipe_id(false),
            interactive_recipe(SIDEBAR, INSET, INSET, 12),
        ),
        (
            nav_recipe_id(true),
            interactive_recipe(ACCENT_ACTIVE, ACCENT, ACCENT_ACTIVE, 12),
        ),
        (
            primary_recipe_id(),
            interactive_recipe(ACCENT, ACCENT_HOVER, ACCENT_ACTIVE, 12),
        ),
        (
            disabled_recipe_id(),
            StyleRecipe::new(
                StyleProperties::EMPTY
                    .with_background(Color::rgb(56, 65, 78))
                    .with_foreground(MUTED)
                    .with_padding(padding(12))
                    .with_radius(radius(9))
                    .with_typography(typography(14, true)),
            ),
        ),
        (input_recipe_id(), input),
    ] {
        theme
            .define_recipe(id, recipe)
            .unwrap_or_else(|_| unreachable!("Gallery declares distinct static recipes"));
    }
    StyleEnvironment::new(theme)
}

fn fixed_width(width: u16) -> LayoutStyle {
    LayoutStyle::default()
        .with_width(LayoutDimension::length(LogicalLength::from(width)))
        .with_flex_item(FlexItemStyle::default().with_shrink(LayoutFactor::ZERO))
}

fn expanding_width(minimum: u16) -> LayoutStyle {
    // A zero flex basis avoids treating Fill as an entire row when the
    // containing navigation/content pair has wrapping enabled.
    LayoutStyle::default()
        .with_min_width(LayoutBound::length(LogicalLength::from(minimum)))
        .with_flex_item(
            FlexItemStyle::default()
                .with_basis(FlexBasis::length(LogicalLength::ZERO))
                .with_grow(LayoutFactor::ONE),
        )
}

fn sidebar_layout() -> LayoutStyle {
    fixed_width(190).with_container(LayoutContainer::Flex(
        FlexContainerStyle::default().with_direction(FlexDirection::Column),
    ))
}

fn workspace_layout() -> LayoutStyle {
    expanding_width(252).with_container(LayoutContainer::Flex(
        FlexContainerStyle::default().with_direction(FlexDirection::Column),
    ))
}

fn preview_layout() -> LayoutStyle {
    expanding_width(252).with_container(LayoutContainer::Flex(
        FlexContainerStyle::default().with_direction(FlexDirection::Column),
    ))
}

fn inspector_layout() -> LayoutStyle {
    fixed_width(218).with_container(LayoutContainer::Flex(
        FlexContainerStyle::default().with_direction(FlexDirection::Column),
    ))
}

fn wrapping_row() -> LayoutStyle {
    LayoutStyle::default().with_container(LayoutContainer::Flex(
        FlexContainerStyle::default()
            .with_direction(FlexDirection::Row)
            .with_wrap(FlexWrap::Wrap)
            .with_align_items(ItemAlignment::Start),
    ))
}

fn navigation(state: &GalleryState) -> Element<GalleryAction> {
    column(children![
        column(children![
            text("RunenUI").typography(typography(22, true)),
            text("COMPONENT GALLERY")
                .typography(typography(11, true))
                .foreground(MUTED),
        ])
        .gap(5_u16)
        .padding(padding(6)),
        text("COMPONENTS")
            .typography(typography(11, true))
            .foreground(MUTED)
            .padding(padding(6)),
        button("Basics")
            .id("gallery.nav.basics")
            .key("gallery.nav.basics")
            .recipe(nav_recipe_id(state.page == Page::Basics))
            .with_layout(LayoutStyle::default().with_width(LayoutDimension::Fill))
            .on_activate(|| GalleryAction::Navigate(Page::Basics)),
        button("Inputs")
            .id("gallery.nav.inputs")
            .key("gallery.nav.inputs")
            .recipe(nav_recipe_id(state.page == Page::Inputs))
            .with_layout(LayoutStyle::default().with_width(LayoutDimension::Fill))
            .on_activate(|| GalleryAction::Navigate(Page::Inputs)),
        text("Built with the public RunenUI API")
            .typography(typography(12, false))
            .foreground(MUTED)
            .padding(padding(6)),
    ])
    .id("gallery.navigation")
    .key("gallery.navigation")
    .with_layout(sidebar_layout())
    .background(SIDEBAR)
    .radius(radius(12))
    .padding(padding(14))
    .gap(12_u16)
    .into_element()
}

fn basics(state: &GalleryState) -> Element<GalleryAction> {
    column(children![
        text("Buttons")
            .id("gallery.basics.heading")
            .typography(typography(20, true)),
        text("Activation, focus and disabled behavior")
            .typography(typography(13, false))
            .foreground(MUTED),
        row(children![
            button("Activate")
                .id("gallery.basics.activate")
                .key("gallery.basics.activate")
                .recipe(primary_recipe_id())
                .on_activate(|| GalleryAction::Increment),
            button("Disabled")
                .disabled()
                .id("gallery.basics.disabled")
                .recipe(disabled_recipe_id()),
        ])
        .with_layout(wrapping_row())
        .gap(10_u16),
        column(children![
            text("ACTIVATION COUNT")
                .typography(typography(11, true))
                .foreground(MUTED),
            text(format!("Activations: {}", state.presses))
                .id("gallery.basics.value")
                .typography(typography(17, true)),
        ])
        .background(INSET)
        .radius(radius(8))
        .padding(padding(14))
        .gap(6_u16),
        text("Try clicking Activate, or focus it with Tab and press Enter or Space.")
            .typography(typography(13, false))
            .foreground(MUTED),
    ])
    .id("gallery.card.basics")
    .key("gallery.page.basics")
    .with_layout(preview_layout())
    .background(PANEL)
    .radius(radius(12))
    .padding(padding(20))
    .gap(14_u16)
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
        text("Form inputs").typography(typography(20, true)),
        text("Selection, editing and native text input")
            .typography(typography(13, false))
            .foreground(MUTED),
        column(children![
            text("Checkbox")
                .id("gallery.inputs.heading")
                .typography(typography(16, true)),
            checkbox("Enable preview", state.checked)
                .id("gallery.inputs.checkbox")
                .key("gallery.inputs.checkbox")
                .recipe(focus_recipe_id())
                .on_activate(|| GalleryAction::Toggle),
            text(format!("Checked: {}", state.checked))
                .typography(typography(13, false))
                .foreground(MUTED),
        ])
        .background(INSET)
        .radius(radius(8))
        .padding(padding(14))
        .gap(10_u16),
        column(children![
            text("TextField")
                .id("gallery.inputs.textfield.label")
                .typography(typography(16, true)),
            field
                .placeholder("Type a value")
                .labelled_by("gallery.inputs.textfield.label")
                .id("gallery.inputs.textfield")
                .key("gallery.inputs.textfield")
                .recipe(input_recipe_id())
                .with_layout(LayoutStyle::default().with_width(LayoutDimension::Fill)),
            text(format!("Application value: {}", state.document))
                .typography(typography(13, false))
                .foreground(MUTED),
        ])
        .background(INSET)
        .radius(radius(8))
        .padding(padding(14))
        .gap(10_u16),
        text("Supports Unicode, native IME and M10 undo/redo transactions.")
            .typography(typography(13, false))
            .foreground(MUTED),
    ])
    .id("gallery.card.inputs")
    .key("gallery.page.inputs")
    .with_layout(preview_layout())
    .background(PANEL)
    .radius(radius(12))
    .padding(padding(20))
    .gap(14_u16)
    .into_element()
}

fn inspector(state: &GalleryState) -> Element<GalleryAction> {
    // This is a read-only projection, not a second editor or state authority.
    column(children![
        text("LIVE STATE")
            .typography(typography(11, true))
            .foreground(MUTED),
        text("Live application state")
            .id("gallery.inspector.title")
            .typography(typography(16, true)),
        text("Application-owned values update as controls are used.")
            .typography(typography(12, false))
            .foreground(MUTED),
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
    .id("gallery.inspector")
    .key("gallery.inspector")
    .with_layout(inspector_layout())
    .background(SIDEBAR)
    .radius(radius(12))
    .padding(padding(16))
    .gap(12_u16)
    .into_element()
}

pub fn root(state: &GalleryState) -> Element<GalleryAction> {
    let page = match state.page {
        Page::Basics => basics(state),
        Page::Inputs => inputs(state),
    };

    row(children![
        navigation(state),
        column(children![
            column(children![
                text(state.page.title())
                    .id("gallery.current.page")
                    .typography(typography(24, true)),
                text("Explore and exercise public components in a native window.")
                    .typography(typography(13, false))
                    .foreground(MUTED),
            ])
            .gap(6_u16),
            row(children![page, inspector(state)])
                .id("gallery.content")
                .key("gallery.content")
                .with_layout(wrapping_row())
                .gap(16_u16),
        ])
        .id("gallery.preview")
        .key("gallery.preview")
        .with_layout(workspace_layout())
        .gap(20_u16),
    ])
    .id("gallery.root")
    .key("gallery.root")
    .with_layout(
        wrapping_row()
            .with_width(LayoutDimension::Fill)
            .with_height(LayoutDimension::Fill)
            .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
    )
    .foreground(FOREGROUND)
    .background(BACKGROUND)
    .padding(padding(18))
    .gap(18_u16)
    .into_element()
}
