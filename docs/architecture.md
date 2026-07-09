# Design Doc Canvas — RunenUI Component/Action Target Surface + Extension Model

## Status

Draft target design.

This document defines the clean app-facing UI authoring surface for Runenwerk’s UI framework direction. It covers components, props, slots, typed actions, runtime output, extension packages, custom controls, theming, host adapters, renderer boundaries, and future feature growth.

This is not a full renderer design. It is not a complete standalone UI framework extraction plan. It is the target authoring/runtime boundary that future implementation should converge toward.

---

## 1. Problem

Runenwerk needs a UI authoring model that is:

* simple enough for app/product code
* strict enough for validation and deterministic tests
* renderer-neutral
* host-neutral
* extensible by users
* suitable for game UI, editor UI, tool panels, inspectors, graph editors, and future live authoring

The system must avoid these failure modes:

### 1.1 Inline mutation UI

Bad:

```rust id="f8nw9v"
if ui.button("Increment").clicked() {
    counter.count += 1;
}
```

Problem:

* UI directly mutates app/domain state.
* Hard to validate.
* Hard to test deterministically.
* Hard to replay as stories.
* Hard to separate engine/app/UI responsibilities.

### 1.2 Renderer-owned widgets

Bad:

```rust id="pg5boj"
renderer.draw_button(button);
```

Problem:

* Renderer becomes coupled to semantic UI controls.
* Custom render backends need to know app/widget logic.
* SDF/wgpu backend becomes polluted with UI concepts.
* Backend swapping becomes harder.

### 1.3 Premature mega-framework

Bad:

```text id="c4zo7b"
Start with:
  full DSL
  visual editor
  fine-grained reactivity
  SDF renderer
  plugin marketplace
  web/Godot/mobile adapters
  docking workbench
```

Problem:

* Too much abstraction before the core loop works.
* Counter/button proof becomes buried under framework machinery.
* Agents keep fixing structure instead of proving behavior.

---

## 2. Decision

Use a small retained/declarative Rust authoring surface first.

Core target:

```rust id="n0g006"
pub fn counter_app(props: CounterAppProps) -> Element
```

User-facing vocabulary:

| Concept            | Meaning                                                                   |
| ------------------ | ------------------------------------------------------------------------- |
| `Element`          | Public UI tree value returned by components                               |
| `Component`        | Pure function returning `Element`                                         |
| `Props`            | Typed component arguments                                                 |
| `Screen`           | Top-level component for a major app state branch                          |
| `Slot`             | Named child insertion point                                               |
| `Action`           | Typed app intent requested by UI                                          |
| `Route`            | Internal stable dispatch ID                                               |
| `Frame`            | Renderer-facing output                                                    |
| `Primitive`        | Renderer-facing draw command                                              |
| `Control`          | Registered interactive UI behavior                                        |
| `ExtensionPackage` | Static package that registers controls/themes/validators/stories/backends |

Core rule:

```text id="i49phf"
Components compose UI.
Actions request app intent.
Reducers mutate app/domain state.
Runtime derives interaction, layout, hit targets, accessibility, and render primitives.
Renderer draws primitives only.
```

---

## 3. Non-goals for the first implementation

The first implementation does not need:

* external `.runenui` DSL
* visual UI editor
* dynamic plugin loading
* hot reload
* full SDF renderer
* full text editor control
* docking workbench
* graph canvas
* fine-grained invalidation
* multi-host backend matrix
* OS accessibility bridge
* web/Godot/mobile adapters

These are future features. The first goal is a clean vertical slice:

```text id="bg0ea5"
props -> components -> Element
Element + input -> runtime output
runtime output -> action proposals + frame
app reducer -> state mutation
next props -> updated UI
```

---

## 4. Architecture overview

```text id="flsupj"
App/domain state
      |
      v
Props
      |
      v
Components
      |
      v
Element tree
      |
      v
Runtime update
      |
      +--> UiFrame
      +--> UiHitTargetMap
      +--> UiAccessibilityTree
      +--> UiOutputEvent::ActionRequested
      +--> UiRuntimeState
      +--> Diagnostics
      |
      v
Host/app consumes action proposals
      |
      v
Reducer mutates app/domain state
      |
      v
Next frame receives new props
```

Renderer boundary:

```text id="ap26i0"
Element/Button/Action
      |
      v
layout + interaction + accessibility + primitive extraction
      |
      v
UiFrame
      |
      v
Renderer backend
```

Renderer must not know:

* `Counter`
* `CounterAction`
* `ButtonElement`
* route validation
* app reducers
* domain state

---

## 5. Domain example

### 5.1 Domain state

```rust id="lrg12r"
#[derive(Clone, Debug)]
pub struct Counter {
    pub count: i64,
}

impl Counter {
    pub fn phase(&self, win_at: i64) -> CounterPhase {
        if self.count >= win_at {
            CounterPhase::Won
        } else {
            CounterPhase::Counting
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CounterPhase {
    Counting,
    Won,
}
```

### 5.2 Typed app actions

```rust id="0yi0k1"
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CounterAction {
    Increment,
    Decrement,
    Reset,
}

impl UiAction for CounterAction {
    fn route(&self) -> RouteId {
        match self {
            CounterAction::Increment => route!("counter.increment"),
            CounterAction::Decrement => route!("counter.decrement"),
            CounterAction::Reset => route!("counter.reset"),
        }
    }

    fn payload(&self) -> UiPayload {
        UiPayload::unit()
    }
}
```

### 5.3 App reducer

The app/domain owns mutation. UI only proposes actions.

```rust id="qa5jzr"
pub fn reduce_counter(counter: &mut Counter, action: CounterAction, win_at: i64) -> UiActionResult {
    match action {
        CounterAction::Increment if counter.count < win_at => {
            counter.count += 1;
            UiActionResult::accepted()
        }

        CounterAction::Decrement if counter.count > 0 && counter.count < win_at => {
            counter.count -= 1;
            UiActionResult::accepted()
        }

        CounterAction::Reset if counter.count > 0 => {
            counter.count = 0;
            UiActionResult::accepted()
        }

        _ => UiActionResult::rejected("disabled_or_invalid_counter_action"),
    }
}
```

Important rule:

```text id="ey7p8k"
UI availability is predictive.
Domain reducer validation is authoritative.
```

---

## 6. App-facing component model

### 6.1 Root component

```rust id="60xf1a"
#[derive(Clone, Debug)]
pub struct CounterAppProps {
    pub count: i64,
    pub win_at: i64,
}

impl CounterAppProps {
    pub fn from_counter(counter: &Counter) -> Self {
        Self {
            count: counter.count,
            win_at: 10,
        }
    }
}

pub fn counter_app(props: CounterAppProps) -> Element {
    let content = if props.count >= props.win_at {
        win_screen(WinScreenProps {
            count: props.count,
            reset_action: CounterAction::Reset,
        })
    } else {
        counting_screen(CountingScreenProps {
            count: props.count,
            win_at: props.win_at,
        })
    };

    app_shell(AppShellProps {
        title: "Counter".into(),
        content,
        footer: Some(status_bar(StatusBarProps {
            text: format!("Target: {}", props.win_at),
        })),
    })
}
```

### 6.2 App shell with explicit slots

Use explicit slot props first. Do not build a generalized slot registry until necessary.

```rust id="1gxl9e"
#[derive(Clone, Debug)]
pub struct AppShellProps {
    pub title: String,
    pub content: Element,
    pub footer: Option<Element>,
}

pub fn app_shell(props: AppShellProps) -> Element {
    let mut shell = ui::column("app_shell")
        .class("app-shell")
        .gap(12)
        .child(
            ui::header("app_header")
                .child(ui::label("app_title", props.title)),
        )
        .child(
            ui::slot("content")
                .child(props.content),
        );

    if let Some(footer) = props.footer {
        shell = shell.child(
            ui::slot("footer")
                .child(footer),
        );
    }

    shell.into_element()
}
```

### 6.3 Counting screen

```rust id="1vqo6d"
#[derive(Clone, Debug)]
pub struct CountingScreenProps {
    pub count: i64,
    pub win_at: i64,
}

pub fn counting_screen(props: CountingScreenProps) -> Element {
    ui::column("counting_screen")
        .class("screen counting-screen")
        .gap(8)
        .child(ui::label("title", "Counter").size(TextSize::Title))
        .child(ui::label("count", format!("Count: {}", props.count)).size(TextSize::BodyLarge))
        .child(counter_controls(CounterControlsProps {
            count: props.count,
            win_at: props.win_at,
        }))
        .into_element()
}
```

### 6.4 Win screen

```rust id="hxtunr"
#[derive(Clone, Debug)]
pub struct WinScreenProps<A> {
    pub count: i64,
    pub reset_action: A,
}

pub fn win_screen<A>(props: WinScreenProps<A>) -> Element
where
    A: UiAction + Clone + 'static,
{
    ui::column("win_screen")
        .class("screen win-screen")
        .gap(8)
        .child(ui::label("win_title", "You win!").size(TextSize::Title))
        .child(ui::label("final_count", format!("Final count: {}", props.count)))
        .child(action_button(
            ActionButtonProps::new("reset", "Reset", props.reset_action)
                .variant(ButtonVariant::Primary)
                .tooltip("Start over"),
        ))
        .into_element()
}
```

### 6.5 Counter controls

Name this `counter_controls`, not `counter_actions`.

Reason:

```text id="fzqzsn"
The component visually renders controls.
The actions themselves are typed app intents.
```

```rust id="ojhpa4"
#[derive(Clone, Debug)]
pub struct CounterControlsProps {
    pub count: i64,
    pub win_at: i64,
}

pub fn counter_controls(props: CounterControlsProps) -> Element {
    ui::row("counter_controls")
        .class("counter-controls")
        .gap(6)
        .child(action_button(
            ActionButtonProps::new("increment", "Increment", CounterAction::Increment)
                .variant(ButtonVariant::Primary)
                .enabled(props.count < props.win_at)
                .tooltip("Increase the counter"),
        ))
        .child(action_button(
            ActionButtonProps::new("decrement", "Decrement", CounterAction::Decrement)
                .variant(ButtonVariant::Secondary)
                .enabled(props.count > 0 && props.count < props.win_at)
                .tooltip("Decrease the counter"),
        ))
        .child(action_button(
            ActionButtonProps::new("reset", "Reset", CounterAction::Reset)
                .variant(ButtonVariant::Secondary)
                .enabled(props.count > 0)
                .tooltip("Reset to zero"),
        ))
        .into_element()
}
```

### 6.6 Reusable action button

```rust id="g5k99e"
#[derive(Clone, Debug)]
pub struct ActionButtonProps<A> {
    pub id: Id,
    pub label: String,
    pub action: A,
    pub variant: ButtonVariant,
    pub enabled: bool,
    pub icon: Option<IconId>,
    pub tooltip: Option<String>,
}

impl<A> ActionButtonProps<A> {
    pub fn new(id: impl Into<Id>, label: impl Into<String>, action: A) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            action,
            variant: ButtonVariant::Default,
            enabled: true,
            icon: None,
            tooltip: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn icon(mut self, icon: IconId) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

pub fn action_button<A>(props: ActionButtonProps<A>) -> Element
where
    A: UiAction + Clone + 'static,
{
    let accessible_label = props.label.clone();

    let mut button = ui::button(props.id)
        .variant(props.variant)
        .label(props.label)
        .accessible_label(accessible_label)
        .action(props.action)
        .enabled(props.enabled);

    if let Some(icon) = props.icon {
        button = button.icon(icon);
    }

    if let Some(tooltip) = props.tooltip {
        button = button.tooltip(tooltip);
    }

    button.into_element()
}
```

### 6.7 Status bar

```rust id="fz5zd6"
#[derive(Clone, Debug)]
pub struct StatusBarProps {
    pub text: String,
}

pub fn status_bar(props: StatusBarProps) -> Element {
    ui::row("status_bar")
        .class("status-bar")
        .child(ui::label("status_text", props.text).size(TextSize::Small))
        .into_element()
}
```

---

## 7. Minimal framework contracts

### 7.1 Prelude

```rust id="9xyqwx"
pub mod prelude {
    pub use crate::{
        action::{RouteId, UiAction, UiActionProposal, UiActionResult, UiOutputEvent, UiPayload},
        builder::ui,
        element::{Element, IntoElement},
        extension::{UiExtensionPackage, UiRegistry},
        id::{Id, id, route},
        input::UiInputEvent,
        renderer::UiRenderer,
        runtime::{UiFrameInput, UiFrameOutput, UiRuntime, UiRuntimeState},
        style::{ButtonVariant, IconId, TextSize, UiResolvedTheme},
        time::UiTime,
        viewport::UiViewport,
    };
}
```

### 7.2 Element

```rust id="dk0ct2"
#[derive(Clone, Debug)]
pub struct Element {
    pub id: Id,
    pub kind: ElementKind,
    pub classes: Vec<String>,
    pub children: Vec<Element>,
}

#[derive(Clone, Debug)]
pub enum ElementKind {
    Column(ColumnElement),
    Row(RowElement),
    Header(HeaderElement),
    Slot(SlotElement),
    Label(LabelElement),
    Button(ButtonElement),
    Custom(CustomElement),
}
```

### 7.3 Button element

```rust id="52uvri"
#[derive(Clone, Debug)]
pub struct ButtonElement {
    pub label: String,
    pub accessible_label: String,
    pub action: ActionDescriptor,
    pub variant: ButtonVariant,
    pub enabled: bool,
    pub icon: Option<IconId>,
    pub tooltip: Option<String>,
}
```

### 7.4 Custom element

Custom elements are for registered controls, not normal user components.

```rust id="ksbwzq"
#[derive(Clone, Debug)]
pub struct CustomElement {
    pub kind: ControlKindId,
    pub props: UiPropsValue,
}
```

Rule:

```text id="5kwrqm"
Components are free functions returning Element.
Controls are registered custom behavior.
```

### 7.5 Action descriptor

```rust id="x4fd31"
#[derive(Clone, Debug)]
pub struct ActionDescriptor {
    pub route: RouteId,
    pub payload: UiPayload,
}

pub trait UiAction {
    fn route(&self) -> RouteId;

    fn payload(&self) -> UiPayload {
        UiPayload::unit()
    }
}
```

### 7.6 Runtime input/output

```rust id="ccod5h"
pub struct UiFrameInput {
    pub root: Element,
    pub input_events: Vec<UiInputEvent>,
    pub previous_state: UiRuntimeState,
    pub viewport: UiViewport,
    pub theme: UiResolvedTheme,
    pub time: UiTime,
}

pub struct UiFrameOutput {
    pub next_state: UiRuntimeState,
    pub frame: UiFrame,
    pub hit_targets: UiHitTargetMap,
    pub accessibility: UiAccessibilityTree,
    pub events: Vec<UiOutputEvent>,
    pub diagnostics: Vec<UiDiagnostic>,
}

pub trait UiRuntime {
    fn update(&mut self, input: UiFrameInput) -> UiFrameOutput;
}
```

### 7.7 Renderer boundary

```rust id="7k0pas"
pub trait UiRenderer {
    fn render(&mut self, frame: UiFrame);
}
```

Renderer consumes `UiFrame`, not `Element`, not `ButtonElement`, and not app actions.

---

## 8. Runtime flow

```rust id="s7im37"
pub fn run_counter_frame(
    ui_runtime: &mut dyn UiRuntime,
    renderer: &mut dyn UiRenderer,
    counter: &mut Counter,
    input_events: Vec<UiInputEvent>,
    previous_state: UiRuntimeState,
    viewport: UiViewport,
    theme: UiResolvedTheme,
) -> UiRuntimeState {
    let root = counter_app(CounterAppProps::from_counter(counter));

    let output = ui_runtime.update(UiFrameInput {
        root,
        input_events,
        previous_state,
        viewport,
        theme,
        time: UiTime::now(),
    });

    for event in output.events {
        match event {
            UiOutputEvent::ActionRequested(proposal) => {
                if let Some(action) = proposal.decode::<CounterAction>() {
                    reduce_counter(counter, action, 10);
                }
            }
        }
    }

    renderer.render(output.frame);

    output.next_state
}
```

---

## 9. Lowering model

User-facing code:

```rust id="ydr3jd"
action_button(
    ActionButtonProps::new("increment", "Increment", CounterAction::Increment)
        .enabled(count < win_at)
)
```

Lowers to internal element:

```rust id="zfrlxx"
ElementKind::Button(ButtonElement {
    label: "Increment".into(),
    accessible_label: "Increment".into(),
    action: ActionDescriptor {
        route: route!("counter.increment"),
        payload: UiPayload::unit(),
    },
    variant: ButtonVariant::Primary,
    enabled: count < win_at,
    icon: None,
    tooltip: Some("Increase the counter".into()),
})
```

Runtime produces:

```text id="iycjuq"
UiFrame:
  RectPrimitive button background
  BorderPrimitive button border
  GlyphRunPrimitive "Increment"

UiHitTarget:
  id = increment
  rect = computed layout rect
  route = counter.increment
  enabled = true

UiAccessibilityNode:
  role = button
  name = Increment

UiOutputEvent on click:
  ActionRequested(counter.increment)
```

---

## 10. Reactivity model

Initial implementation:

```text id="1s8bjy"
Full recompute per frame.
```

Inputs:

* root `Element`
* previous UI runtime state
* input events
* viewport
* theme
* time

Outputs:

* next UI runtime state
* render frame
* hit targets
* accessibility tree
* action proposals
* diagnostics

Later optimization can add dirty tracking. Do not start with fine-grained invalidation.

Reactive rule:

```text id="cy7pvh"
App state changes outside UI.
Next frame builds a new Element tree from new props.
Runtime derives layout, hit targets, accessibility, and render primitives from that tree.
```

Screen switching is declarative:

```rust id="ocd53s"
let content = if count >= win_at {
    win_screen(...)
} else {
    counting_screen(...)
};
```

Do not use imperative screen mutation:

```rust id="e1mbqg"
ui_runtime.set_screen("win_screen");
```

---

## 11. Input and action flow

Button click flow:

```text id="g8djun"
Pointer input
-> hit test
-> pressed/hover/focus runtime state update
-> released over same enabled hit target
-> ActionRequested(route)
-> app reducer validates and mutates domain state
-> next frame receives updated props
-> new Element tree
-> new UiFrame
```

The UI runtime does not mutate app/domain state.

---

## 12. ID strategy

Component-local IDs should be allowed:

```rust id="ahb3r6"
id!("increment")
id!("reset")
```

Runtime/compiler should namespace by component path:

```text id="6y42yg"
counter_app/counting_screen/counter_controls/increment
counter_app/win_screen/reset
```

This prevents conflicts when components are reused.

Initial implementation may use explicit full IDs if namespacing is not ready yet, but the target should support component-local IDs.

Rules:

```text id="i4yy1x"
1. IDs are stable across frames.
2. IDs are local by default.
3. Component path provides namespace.
4. Explicit global IDs are allowed only when needed.
5. Duplicate IDs in the same namespace produce diagnostics.
```

---

## 13. Accessibility requirements

Minimum requirements:

* text buttons derive accessible label from visible label
* icon-only buttons require explicit accessible label
* disabled controls remain represented as disabled accessibility nodes
* interactive controls expose role, bounds, label, enabled state, and action metadata

Minimum button accessibility output:

```text id="df2l83"
role = button
name = accessible_label
bounds = computed layout rect
enabled = button.enabled
action = press
```

Accessibility is part of runtime output, not renderer output.

---

## 14. Renderer requirements

Renderer must consume only frame primitives.

Allowed renderer input:

```text id="q9d4q2"
UiFrame
UiSurface
UiLayer
UiPrimitive
```

Disallowed renderer input:

```text id="loau1q"
Element
ButtonElement
CounterAction
RouteId
UiActionProposal
domain state
```

Button rendering path:

```text id="12r31y"
ButtonElement
-> layout box
-> hit target
-> accessibility node
-> RectPrimitive + BorderPrimitive + GlyphRunPrimitive
-> renderer draws primitives
```

SDF backend later:

```text id="n97qg2"
RectPrimitive -> SDF box instance
BorderPrimitive -> SDF border/focus instance
GlyphRunPrimitive -> MSDF glyph instances
```

---

## 15. Extension model

The framework must support users adding new UI without editing core.

There are three extension levels:

```text id="k3rd1q"
Level 1: User components
Level 2: Registered controls
Level 3: Host/backend extensions
```

### 15.1 Level 1 — User components

User components are pure functions returning `Element`.

They require no registry.

Example:

```rust id="x85mhr"
#[derive(Clone, Debug)]
pub struct DangerButtonProps<A> {
    pub id: Id,
    pub label: String,
    pub action: A,
    pub enabled: bool,
}

pub fn danger_button<A>(props: DangerButtonProps<A>) -> Element
where
    A: UiAction + Clone + 'static,
{
    action_button(
        ActionButtonProps::new(props.id, props.label, props.action)
            .variant(ButtonVariant::Danger)
            .enabled(props.enabled),
    )
}
```

This is enough for:

```text id="f3cczx"
panels
cards
badges
status bars
toolbars
property rows
inspector groups
icon buttons assembled from existing controls
```

Rule:

```text id="oj2exk"
If it only composes existing elements, it is a component.
Components are free.
```

### 15.2 Level 2 — Registered controls

Registered controls add new runtime behavior.

Examples:

```text id="9x1pd4"
slider
text input
color picker
tree view
virtual list
node graph
timeline
curve editor
asset picker
material preview
```

Registered controls need framework support for:

* props validation
* state
* layout
* input handling
* accessibility output
* primitive extraction
* story fixtures

Rule:

```text id="hca1ml"
If it needs custom interaction/runtime behavior, it is a control.
Controls are registered.
```

### 15.3 Level 3 — Host/backend extensions

Host/backend extensions connect UI core to platforms and renderers.

Examples:

```text id="jszbvm"
Runenwerk engine adapter
winit adapter
headless host
Godot adapter later
SDF/wgpu renderer
headless renderer
accessibility bridge
font/text backend
asset resolver
```

Rule:

```text id="r9i2mi"
If it connects UI to a platform, engine, or renderer, it is an adapter/backend.
Adapters are outside core.
```

---

## 16. Extension package API

Use static Rust registration first.

Dynamic plugin loading is deferred.

```rust id="b4r6db"
pub trait UiExtensionPackage {
    fn package_id(&self) -> UiPackageId;
    fn version(&self) -> UiPackageVersion;

    fn register(&self, registry: &mut UiRegistry);
}
```

Usage:

```rust id="vp1yc5"
let mut registry = UiRegistry::new();

registry.register_package(CoreControlsPackage);
registry.register_package(EditorControlsPackage);
registry.register_package(GraphEditorPackage);
registry.register_package(MaterialPreviewPackage);
```

Registry shape:

```rust id="0cc55y"
pub struct UiRegistry {
    pub controls: ControlRegistry,
    pub themes: ThemeRegistry,
    pub validators: ValidatorRegistry,
    pub layout: LayoutRegistry,
    pub primitive_extractors: PrimitiveExtractorRegistry,
    pub stories: StoryRegistry,
}
```

Initial implementation can be smaller:

```rust id="aa2uad"
pub struct UiRegistry {
    pub controls: ControlRegistry,
}
```

---

## 17. Custom control contract

Full target contract:

```rust id="da1kb6"
pub trait UiControlDefinition {
    type Props: UiProps;
    type State: UiControlState;

    const KIND: ControlKindId;

    fn validate(props: &Self::Props, ctx: &mut ValidationContext);

    fn init_state(props: &Self::Props) -> Self::State;

    fn measure(
        props: &Self::Props,
        state: &Self::State,
        ctx: &mut MeasureContext,
    ) -> UiSize;

    fn arrange(
        props: &Self::Props,
        state: &mut Self::State,
        rect: UiRect,
        ctx: &mut ArrangeContext,
    );

    fn handle_input(
        props: &Self::Props,
        state: &mut Self::State,
        input: &UiInputEvent,
        ctx: &mut InteractionContext,
    );

    fn accessibility(
        props: &Self::Props,
        state: &Self::State,
        ctx: &mut AccessibilityContext,
    );

    fn render_primitives(
        props: &Self::Props,
        state: &Self::State,
        ctx: &mut PrimitiveContext,
    );
}
```

First implementation should use a reduced contract:

```rust id="o2z5se"
pub trait UiControlDefinition {
    type Props: UiProps;

    const KIND: ControlKindId;

    fn validate(props: &Self::Props, ctx: &mut ValidationContext);
    fn build_element(props: Self::Props) -> Element;
}
```

Then expand only when needed.

---

## 18. Example custom component: property row

A property row composes existing elements. It does not need registration.

```rust id="1a0jtg"
#[derive(Clone, Debug)]
pub struct PropertyRowProps {
    pub id: Id,
    pub label: String,
    pub value: Element,
}

pub fn property_row(props: PropertyRowProps) -> Element {
    ui::row(props.id)
        .class("property-row")
        .gap(8)
        .child(ui::label("label", props.label).size(TextSize::Small))
        .child(ui::slot("value").child(props.value))
        .into_element()
}
```

Usage:

```rust id="5rqzxs"
property_row(PropertyRowProps {
    id: id!("roughness_row"),
    label: "Roughness".into(),
    value: slider(SliderProps {
        id: id!("roughness"),
        value: material.roughness,
        min: 0.0,
        max: 1.0,
        action: MaterialAction::SetRoughness,
        enabled: true,
    }),
})
```

---

## 19. Example registered control: slider

A slider needs dragging behavior, payload generation, accessibility value metadata, and primitive extraction.

```rust id="5z2opz"
#[derive(Clone, Debug)]
pub struct SliderProps<A> {
    pub id: Id,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub action: A,
    pub enabled: bool,
}

pub struct SliderControl;

impl UiControlDefinition for SliderControl {
    type Props = SliderProps<ActionDescriptor>;
    type State = SliderState;

    const KIND: ControlKindId = control_kind!("core.slider");

    fn validate(props: &Self::Props, ctx: &mut ValidationContext) {
        if props.min >= props.max {
            ctx.error("slider_min_must_be_less_than_max");
        }
    }

    fn handle_input(
        props: &Self::Props,
        state: &mut Self::State,
        input: &UiInputEvent,
        ctx: &mut InteractionContext,
    ) {
        // pointer drag -> normalized value
        // normalized value -> payload
        // emit UiOutputEvent::ActionRequested
    }

    fn render_primitives(
        props: &Self::Props,
        state: &Self::State,
        ctx: &mut PrimitiveContext,
    ) {
        // track rect
        // filled rect
        // thumb shape
        // optional label
    }

    fn accessibility(
        props: &Self::Props,
        state: &Self::State,
        ctx: &mut AccessibilityContext,
    ) {
        // role = slider
        // min/max/current value
        // enabled state
    }
}
```

---

## 20. Slot model

Initial slot model:

```rust id="8hz0z6"
pub struct AppShellProps {
    pub content: Element,
    pub footer: Option<Element>,
}
```

Future slot specification:

```rust id="6nl95q"
pub struct SlotSpec {
    pub name: SlotName,
    pub required: bool,
    pub accepts: SlotAccepts,
    pub cardinality: SlotCardinality,
}
```

Example:

```rust id="yd3qx2"
SlotSpec {
    name: slot!("content"),
    required: true,
    accepts: SlotAccepts::AnyElement,
    cardinality: SlotCardinality::One,
}
```

Validation examples:

```text id="3khxj3"
AppShell.content missing -> validation error
Toolbar.left accepts only controls -> validation error
Menu.items accepts only menu items -> validation error
```

Do not implement strict slot specs before the simple `Element` slot model works.

---

## 21. Theme extensibility

Themes extend through semantic tokens, not raw colors inside every component.

Theme package API:

```rust id="ovwfd7"
pub trait ThemePackage {
    fn register_tokens(&self, registry: &mut ThemeRegistry);
}
```

Example tokens:

```text id="xn7i8r"
button.primary.background
button.primary.background.hovered
button.primary.background.pressed
button.primary.border
button.primary.border.focused
button.secondary.background
text.body.color
panel.background
surface.background
focus.ring.color
```

Button primitive extraction should use resolved theme tokens:

```rust id="me6xb6"
let background = theme.color(token!("button.primary.background"));
```

Rule:

```text id="d6mzt1"
Components request semantic tokens.
Themes resolve tokens.
Renderers receive resolved paints.
```

---

## 22. Action extensibility

App code defines typed actions.

Framework only requires:

```rust id="8e3qj9"
pub trait UiAction {
    fn route(&self) -> RouteId;

    fn payload(&self) -> UiPayload {
        UiPayload::unit()
    }
}
```

Example payload action:

```rust id="m9q1pu"
#[derive(Clone, Debug)]
pub enum MaterialAction {
    SetRoughness(f32),
    SetMetallic(f32),
    SelectTexture(TextureId),
}

impl UiAction for MaterialAction {
    fn route(&self) -> RouteId {
        match self {
            MaterialAction::SetRoughness(_) => route!("material.set_roughness"),
            MaterialAction::SetMetallic(_) => route!("material.set_metallic"),
            MaterialAction::SelectTexture(_) => route!("material.select_texture"),
        }
    }

    fn payload(&self) -> UiPayload {
        match self {
            MaterialAction::SetRoughness(value) => UiPayload::f32(*value),
            MaterialAction::SetMetallic(value) => UiPayload::f32(*value),
            MaterialAction::SelectTexture(id) => UiPayload::id(id),
        }
    }
}
```

Future validation:

```text id="guk2tw"
material.set_roughness expects f32 in [0, 1]
material.set_metallic expects f32 in [0, 1]
material.select_texture expects TextureId
```

---

## 23. Render extensibility

There are two cases.

### 23.1 New visual components

Most visual changes should be composed from existing primitives.

Examples:

```text id="n0zgg1"
Badge = rounded rect + label
Card = panel + header + slot
IconButton = button + icon
PropertyRow = row + label + slot
```

No renderer extension required.

### 23.2 New render primitives

New primitives are rare. Add them only when existing primitives cannot represent the control.

Possible later primitives:

```text id="k62wcs"
PathPrimitive
BoxShadowPrimitive
GraphCanvasPrimitive
ProductSurfacePrimitive
ViewportSurfaceEmbed
SdfShapePrimitive
```

Rule:

```text id="ldlv7z"
New controls should not automatically require new primitives.
Only add a primitive when the renderer truly needs a new drawing contract.
```

For Runenwerk:

```text id="mjkw3u"
Material preview should use ProductSurface or SurfaceEmbed.
Graph editor can start with rects, strokes, and glyphs.
SDF-specific details stay in backend.
```

---

## 24. Host/backend extensibility

Host adapter:

```rust id="cshajn"
pub trait UiHostAdapter {
    fn collect_input(&mut self) -> Vec<UiInputEvent>;
    fn viewport(&self) -> UiViewport;
    fn theme(&self) -> UiResolvedTheme;
    fn handle_request(&mut self, request: UiHostRequest);
    fn handle_event(&mut self, event: UiOutputEvent);
}
```

Renderer backend:

```rust id="tsewfx"
pub trait UiRendererBackend {
    fn prepare(&mut self, frame: &UiFrame);
    fn render(&mut self, target: &mut dyn UiRenderTarget);
}
```

Examples:

```text id="fbm7le"
Hosts:
  RunenwerkHostAdapter
  HeadlessHostAdapter
  WinitHostAdapter
  GodotHostAdapter later

Renderers:
  HeadlessRenderer
  SdfWgpuRenderer later
  TinySkiaRenderer maybe
```

Host/backend code must remain outside UI core.

---

## 25. Validation rules

Minimum validation:

* duplicate sibling IDs produce diagnostics
* button must have label or accessible label
* icon-only button must have explicit accessible label
* disabled button must not emit action events
* button action must lower to a route
* unknown action route must be rejected by app/host
* reducer must guard domain constraints even if UI says enabled
* required slot content must be present
* custom control kind must be registered
* custom control props must validate

Later validation:

* route schema checking
* payload schema checking
* capability checking
* source maps
* story run reports
* theme token validation
* accessibility completeness reports
* extension package version compatibility

---

## 26. Future features

### 26.1 P0 — Framework correctness

Needed before calling this a real framework slice:

| Feature                          | Reason                                |
| -------------------------------- | ------------------------------------- |
| Component-local ID namespacing   | Reusable components cannot collide    |
| Typed props                      | Components stay explicit and testable |
| Typed action proposals           | UI does not mutate app state          |
| Control registry                 | Users can add real widgets            |
| Validation reports               | Bad UI fails before runtime           |
| Basic accessibility metadata     | Must not be bolted on last            |
| Headless runtime tests           | Prevents fake visual-only progress    |
| Primitive-only renderer boundary | Prevents renderer coupling            |
| Full recompute runtime           | Simple deterministic starting point   |

### 26.2 P1 — Productive UI system

Needed for serious tools/editor UI:

| Feature               | Reason                          |
| --------------------- | ------------------------------- |
| Text input            | Any serious UI needs editing    |
| Focus traversal       | Keyboard usability              |
| Keyboard shortcuts    | Editor workflow                 |
| Menus/context menus   | Tool UX                         |
| Popovers/overlays     | Dropdowns/tooltips/modals       |
| Scroll containers     | Panels/lists                    |
| Virtual lists/tables  | Asset browsers/logs             |
| Tree view             | Scene/entity hierarchy          |
| Inspector forms       | Material/procgen/entity editing |
| Drag/drop             | Assets/docking/graphs           |
| Undo/redo integration | Editor-grade action handling    |
| Theme tokens          | Scalable styling                |
| Story runner          | Deterministic UI proofs         |

### 26.3 P2 — Advanced platform/editor features

Useful later:

| Feature                   | Reason                        |
| ------------------------- | ----------------------------- |
| Docking layout            | Full workbench                |
| Graph canvas              | Material/procgen/shader tools |
| Timeline/curve editor     | Animation/procedural tooling  |
| Live preview              | UI authoring workflow         |
| Hot reload                | Productivity                  |
| External `.runenui` DSL   | Non-Rust authoring            |
| Visual UI editor          | Later authoring surface       |
| Fine-grained invalidation | Performance optimization      |
| Frame diffing             | Renderer optimization         |
| SDF/wgpu backend          | Custom high-quality rendering |
| Web/Godot adapters        | Multi-host reach              |

---

## 27. Acceptance tests

### 27.1 Component composition

Given:

```rust id="o5rb96"
counter_app(CounterAppProps { count: 3, win_at: 10 })
```

Expect:

* root contains `app_shell`
* content slot contains `counting_screen`
* `win_screen` is absent

Given:

```rust id="b5j6nv"
counter_app(CounterAppProps { count: 10, win_at: 10 })
```

Expect:

* content slot contains `win_screen`
* `counting_screen` is absent

### 27.2 Button lowering

Given:

```rust id="27zc1p"
ActionButtonProps::new("increment", "Increment", CounterAction::Increment)
```

Expect lowered button:

* label = `Increment`
* accessible label = `Increment`
* route = `counter.increment`
* enabled = `true`

### 27.3 Disabled button

Given decrement button at count `0`:

* button exists
* enabled = `false`
* hit target exists or is marked disabled
* click does not emit action event

### 27.4 Action proposal

Given enabled increment button and pointer press/release inside its hit target:

Expect:

```text id="wgy59j"
UiOutputEvent::ActionRequested(counter.increment)
```

### 27.5 Domain reducer guard

Given count `10` and `CounterAction::Increment`:

Expect:

```text id="pglxse"
UiActionResult::Rejected
counter.count remains 10
```

### 27.6 Renderer isolation

Renderer test must prove renderer receives only `UiFrame`.

Renderer must not import:

* `CounterAction`
* `ButtonElement`
* app domain state
* action proposal types

### 27.7 Custom component extension

Given user component `danger_button`:

* no registry required
* output is an `Element`
* composed button lowers normally

### 27.8 Custom control registration

Given custom `SliderControl`:

* unregistered use emits diagnostic
* registered use validates props
* invalid min/max emits validation error
* enabled drag emits action proposal
* disabled drag emits no action proposal
* accessibility exposes slider role/value

---

## 28. Recommended implementation slices

### Slice 1 — App-facing syntax

Implement:

* `Element`
* `ElementKind`
* `ui::column`
* `ui::row`
* `ui::label`
* `ui::button`
* `ui::slot`
* `UiAction`
* `ActionDescriptor`
* `ActionButtonProps`
* counter example

No renderer work required beyond headless checks.

### Slice 2 — Runtime proof

Implement:

* layout boxes for row/column/label/button
* hit target map
* pointer moved/pressed/released
* hover/pressed/focus state
* `ActionRequested`
* full recompute per frame

### Slice 3 — Render primitive output

Implement lowering:

* label -> text/glyph primitive
* button -> `RectPrimitive`, `BorderPrimitive`, text/glyph primitive
* row/column backgrounds if styled

Renderer remains primitive-only.

### Slice 4 — Extension registry minimum

Implement:

* `UiRegistry`
* `UiExtensionPackage`
* `ControlRegistry`
* custom control kind lookup
* registered/unregistered diagnostics
* one example custom control or stub slider

### Slice 5 — Runenwerk adapter

Bridge:

* engine input -> `UiInputEvent`
* app/domain snapshot -> props
* `UiFrame` -> Runenwerk render submission
* `UiOutputEvent` -> app/domain reducer
* host requests -> cursor/clipboard/IME/window handling

### Slice 6 — Hardening

Add:

* validation reports
* source maps
* basic accessibility tree
* story/replay test
* component-local ID namespacing
* disabled/rejected action diagnostics

---

## 29. Final target statement

The target UI surface is:

```rust id="mazzlr"
pub fn counter_app(props: CounterAppProps) -> Element
```

with components:

```rust id="arzos1"
pub fn app_shell(props: AppShellProps) -> Element
pub fn counting_screen(props: CountingScreenProps) -> Element
pub fn win_screen<A: UiAction + Clone + 'static>(props: WinScreenProps<A>) -> Element
pub fn counter_controls(props: CounterControlsProps) -> Element
pub fn action_button<A: UiAction + Clone + 'static>(props: ActionButtonProps<A>) -> Element
```

The extension model is:

```text id="u0wnuu"
Components are easy extension.
Controls are registered extension.
Hosts/backends are adapter extension.
Primitives are rare renderer-contract extension.
```

The target boundary is:

```text id="9znns9"
App/domain state -> props -> components -> Element
Element + input + runtime state -> UiFrameOutput
UiFrameOutput.events -> app reducer
UiFrameOutput.frame -> renderer
```

The target rule is:

```text id="5r6m5l"
Everything app-facing returns Element.
Everything interactive emits Action.
Everything renderer-facing is produced later as UiFrame primitives.
Everything app-mutating happens in reducers outside UI.
```
