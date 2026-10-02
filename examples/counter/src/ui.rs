use std::time::Duration;

use runenui_core::{
    AnimationId, Brush, Color, EdgeInsets, Element, ExplicitTimeline, FlexBasis,
    FlexContainerStyle, FlexDirection, FlexItemStyle, FlexWrap, ItemAlignment, LayoutContainer,
    LayoutDimension, LayoutFactor, LayoutStyle, LogicalLength, MainAxisAlignment, MotionEasing,
    MotionKeyframe, MotionRepeat, MotionTarget, MotionValue, Outline, OverflowPolicy,
    OverflowStyle, Radius, SceneOpacity, StrokeStyle, StyleEnvironment, StyleInteractionState,
    StyleProperties, StyleRecipe, StyleRecipeId, StyleTheme, StyleTokens, TimelineSpec,
    TransitionSpec, UnitInterval, View, Widget, WidgetMeasure, WidgetMeasureInput, button,
    children, column, row, text,
};

use crate::app::{Counter, CounterAction};

const SCREEN_BACKGROUND: Color = Color::rgb(24, 28, 36);
const SCREEN_FOREGROUND: Color = Color::rgb(238, 240, 246);
const CONTROL_BACKGROUND: Color = Color::rgb(92, 106, 135);
const CONTROL_HOVER_BACKGROUND: Color = Color::rgb(78, 104, 160);
const CONTROL_ACTIVE_BACKGROUND: Color = Color::rgb(64, 80, 122);
const RESET_BACKGROUND: Color = Color::rgb(140, 92, 92);
const RESET_HOVER_BACKGROUND: Color = Color::rgb(166, 78, 86);
const RESET_ACTIVE_BACKGROUND: Color = Color::rgb(120, 60, 68);
const WIN_BACKGROUND: Color = Color::rgb(38, 82, 58);

fn padding(value: u16) -> EdgeInsets {
    EdgeInsets::all(LogicalLength::from(value))
}

fn control_radius() -> Radius {
    Radius::all(LogicalLength::from(6_u16))
}

fn focus_outline() -> Outline {
    Outline::new(
        Brush::solid(SCREEN_FOREGROUND),
        StrokeStyle::new(LogicalLength::from(2_u16)),
    )
}

fn fixed_control_item() -> FlexItemStyle {
    FlexItemStyle::default().with_shrink(LayoutFactor::ZERO)
}

fn control_item_layout() -> LayoutStyle {
    LayoutStyle::default().with_flex_item(fixed_control_item())
}

fn controls_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_container(LayoutContainer::Flex(
            FlexContainerStyle::default()
                .with_direction(FlexDirection::Row)
                .with_wrap(FlexWrap::Wrap)
                .with_justify_content(MainAxisAlignment::Center)
                .with_align_items(ItemAlignment::Center),
        ))
        .with_width(LayoutDimension::Fill)
}

fn content_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_container(LayoutContainer::Flex(
            FlexContainerStyle::default()
                .with_direction(FlexDirection::Column)
                .with_align_items(ItemAlignment::Center),
        ))
        .with_width(LayoutDimension::Fill)
        .with_flex_item(FlexItemStyle::default().with_shrink(LayoutFactor::ZERO))
}

fn stepper_group_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_container(LayoutContainer::Flex(
            FlexContainerStyle::default().with_direction(FlexDirection::Row),
        ))
        .with_flex_item(fixed_control_item())
}

#[derive(Debug)]
struct LayoutSpacer;

impl Widget<CounterAction> for LayoutSpacer {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::ZERO, LogicalLength::ZERO)
    }
}

fn vertical_spacer() -> Element<CounterAction> {
    Element::new(LayoutSpacer).with_layout(
        LayoutStyle::default()
            .with_height(LayoutDimension::length(LogicalLength::ZERO))
            .with_flex_item(
                FlexItemStyle::default()
                    .with_grow(LayoutFactor::ONE)
                    .with_shrink(LayoutFactor::ZERO)
                    .with_basis(FlexBasis::length(LogicalLength::ZERO)),
            ),
    )
}

fn screen_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_container(LayoutContainer::Flex(
            FlexContainerStyle::default().with_direction(FlexDirection::Column),
        ))
        .with_width(LayoutDimension::Fill)
        .with_height(LayoutDimension::Fill)
        .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll))
}

fn count_background(count: i32) -> Color {
    let step = u8::try_from(count.unsigned_abs().min(9))
        .unwrap_or_else(|_| unreachable!("clamped color step fits u8"));
    let offset = step * 12;
    if count < 0 {
        Color::rgb(40, 56, 104 + offset)
    } else {
        Color::rgb(40 + offset, 56, 104)
    }
}

fn count_transition() -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(200),
        Duration::ZERO,
        MotionEasing::Linear,
        None,
    )
    .unwrap_or_else(|_| unreachable!("Counter uses a bounded valid count transition"))
}

fn interaction_transition() -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(100),
        Duration::ZERO,
        MotionEasing::Linear,
        None,
    )
    .unwrap_or_else(|_| unreachable!("Counter uses a bounded valid interaction transition"))
}

fn screen_transition() -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(180),
        Duration::ZERO,
        MotionEasing::Linear,
        None,
    )
    .unwrap_or_else(|_| unreachable!("Counter uses a bounded valid screen transition"))
}

fn win_content_timeline() -> ExplicitTimeline {
    let start = SceneOpacity::TRANSPARENT;
    let spec = TimelineSpec::new(
        vec![
            MotionKeyframe::new(UnitInterval::ZERO, MotionValue::Opacity(start)),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Opacity(SceneOpacity::OPAQUE),
            ),
        ],
        vec![MotionEasing::Linear],
        Duration::from_millis(160),
        Duration::ZERO,
        MotionRepeat::ONCE,
        None,
    )
    .unwrap_or_else(|_| unreachable!("Counter win content timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("counter.win-content")
            .unwrap_or_else(|_| unreachable!("Counter win animation id is valid")),
        spec,
    )
}

fn recipe_id(value: &'static str) -> StyleRecipeId {
    StyleRecipeId::from_static(value)
        .unwrap_or_else(|_| unreachable!("Counter style recipe identifiers are valid"))
}

fn control_recipe_id() -> StyleRecipeId {
    recipe_id("counter.control")
}

fn reset_recipe_id() -> StyleRecipeId {
    recipe_id("counter.reset")
}

fn button_recipe(base: Color, hover: Color, active: Color) -> StyleRecipe {
    let mut recipe = StyleRecipe::new(
        StyleProperties::EMPTY
            .with_background(base)
            .with_padding(padding(6))
            .with_radius(control_radius())
            .with_transition(MotionTarget::Background, interaction_transition()),
    );
    recipe
        .define_interaction(
            StyleInteractionState::Hover,
            StyleProperties::EMPTY.with_background(hover),
        )
        .unwrap_or_else(|_| unreachable!("Counter defines hover once per button recipe"));
    recipe
        .define_interaction(
            StyleInteractionState::Focus,
            StyleProperties::EMPTY.with_outline(focus_outline()),
        )
        .unwrap_or_else(|_| unreachable!("Counter defines focus once per button recipe"));
    recipe
        .define_interaction(
            StyleInteractionState::Active,
            StyleProperties::EMPTY.with_background(active),
        )
        .unwrap_or_else(|_| unreachable!("Counter defines active once per button recipe"));
    recipe
}

/// Returns the complete application-owned style input used by every Counter publication path.
pub fn style_environment() -> StyleEnvironment {
    let mut theme = StyleTheme::new(StyleTokens::new());
    theme
        .define_recipe(
            control_recipe_id(),
            button_recipe(
                CONTROL_BACKGROUND,
                CONTROL_HOVER_BACKGROUND,
                CONTROL_ACTIVE_BACKGROUND,
            ),
        )
        .unwrap_or_else(|_| unreachable!("Counter defines its control recipe once"));
    theme
        .define_recipe(
            reset_recipe_id(),
            button_recipe(
                RESET_BACKGROUND,
                RESET_HOVER_BACKGROUND,
                RESET_ACTIVE_BACKGROUND,
            ),
        )
        .unwrap_or_else(|_| unreachable!("Counter defines its Reset recipe once"));
    StyleEnvironment::new(theme)
}

fn reset_button() -> impl View<CounterAction> {
    button("Reset")
        .id("counter.reset")
        .key("counter.reset")
        .with_layout(control_item_layout())
        .recipe(reset_recipe_id())
        .on_activate(|| CounterAction::Reset)
}

fn controls(children: impl runenui_core::Views<CounterAction>) -> impl View<CounterAction> {
    row(children)
        .key("counter.controls")
        .with_layout(controls_layout())
        .gap(12_u16)
}

fn content(
    children: impl runenui_core::Views<CounterAction>,
    win: bool,
) -> impl View<CounterAction> {
    let content = column(children)
        .id("counter.content")
        .key("counter.content")
        .with_layout(content_layout())
        .gap(8_u16);
    if win {
        content.timeline(win_content_timeline())
    } else {
        content
    }
}

fn screen(background: Color, content: impl View<CounterAction>) -> Element<CounterAction> {
    column(children![vertical_spacer(), content, vertical_spacer()])
        .key("counter.screen")
        .with_layout(screen_layout())
        .background(background)
        .transition(MotionTarget::Background, screen_transition())
        .foreground(SCREEN_FOREGROUND)
        .padding(padding(16))
        .into_element()
}

struct CounterScreen;

impl CounterScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        screen(
            SCREEN_BACKGROUND,
            content(
                children![
                    text("Counter").id("counter.title").key("counter.title"),
                    text(counter.count.to_string())
                        .id("counter.value")
                        .key("counter.value")
                        .background(count_background(counter.count))
                        .transition(MotionTarget::Background, count_transition())
                        .padding(padding(8))
                        .radius(control_radius()),
                    controls(children![
                        row(children![
                            button("−")
                                .id("counter.decrement")
                                .key("counter.decrement")
                                .with_layout(control_item_layout())
                                .recipe(control_recipe_id())
                                .on_activate(|| CounterAction::Decrement),
                            button("+")
                                .id("counter.increment")
                                .key("counter.increment")
                                .with_layout(control_item_layout())
                                .recipe(control_recipe_id())
                                .on_activate(|| CounterAction::Increment),
                        ])
                        .id("counter.stepper")
                        .key("counter.stepper")
                        .with_layout(stepper_group_layout())
                        .gap(6_u16),
                        reset_button(),
                    ]),
                ],
                false,
            ),
        )
    }
}

struct WinScreen;

impl WinScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        let count = counter.count;

        screen(
            WIN_BACKGROUND,
            content(
                children![
                    text("You win").id("counter.win.title").key("counter.title"),
                    text(format!("Count: {count}"))
                        .id("counter.value")
                        .key("counter.value")
                        .background(count_background(count))
                        .transition(MotionTarget::Background, count_transition())
                        .padding(padding(8))
                        .radius(control_radius()),
                    controls(children![reset_button()]),
                ],
                true,
            ),
        )
    }
}

pub fn root(counter: &Counter) -> Element<CounterAction> {
    if counter.has_won() {
        WinScreen::root(counter)
    } else {
        CounterScreen::root(counter)
    }
}
