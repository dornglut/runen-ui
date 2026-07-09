# Vocabulary

RunenUI uses a small public vocabulary for authoring, runtime behavior, and renderer output.

## Core Terms

| Term | Meaning |
|---|---|
| State | Application-owned data used to derive UI. |
| Action | Application-owned intent emitted by elements. |
| update | Function that changes state in response to an action. |
| Element | UI description derived from state. |
| Runtime | System that receives input, dispatches actions, calls `update`, computes layout, and publishes frames. |

## Authoring Terms

| Term | Meaning |
|---|---|
| element! | Rust authoring macro for nested element trees. |
| root | Function that derives the root `Element` from application state. |
| on_press | Event binding for press activation. |
| on_change | Event binding for value changes. |
| id | Stable author-provided element identity for semantic anchors, testing, inspection, or accessibility. |

## Runtime Terms

| Term | Meaning |
|---|---|
| InputEvent | Host-provided input event consumed by the runtime. |
| LayoutBox | Computed layout result for an element. |
| Accessibility tree | Structured semantic UI data derived from elements and layout. |
| Trace | Inspectable record of runtime events such as input, actions, updates, layout, and frame publication. |

## Output Terms

| Term | Meaning |
|---|---|
| Primitive | Renderer-neutral output generated from elements and layout. |
| Surface | Named UI output target. |
| SurfaceFrame | Published frame for a surface. |
| Host | Application, engine, editor, or tool embedding RunenUI. |
| Renderer | Backend that consumes primitive output. |
