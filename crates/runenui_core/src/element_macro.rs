// Declarative element construction macro prototype.
//
// This is intentionally a small `macro_rules!` layer over the existing typed
// builder and argument APIs. It does not introduce a parser, compiler, runtime
// AST, styling system, or procedural macro crate.

/// Builds a typed [`Element`](crate::Element) tree using compact nested syntax.
///
/// This prototype intentionally uses function-like node forms so it can stay as
/// a small `macro_rules!` wrapper over the existing builder APIs:
///
/// ```rust
/// # use runenui_core::{ElementKind, element};
/// # enum Action { Increment }
/// let ui = element! {
///     column(gap = 8_u16, [
///         text("Counter"),
///         button("+", id = "counter.increment", action = Action::Increment),
///     ])
/// };
///
/// let ElementKind::Container(_) = ui.kind() else {
///     panic!("expected container");
/// };
/// ```
#[macro_export]
macro_rules! element {
    (text($content:expr $(, id = $id:expr)? $(, key = $key:expr)? $(,)?)) => {{
        let args = $crate::TextArgs::new($content);
        $(let args = args.id($id);)?
        $(let args = args.key($key);)?
        $crate::text_with(args)
    }};

    (button($label:expr $(, id = $id:expr)? $(, key = $key:expr)? $(, action = $action:expr)? $(, enabled = $enabled:expr)? $(,)?)) => {{
        let args = $crate::ButtonArgs::new($label);
        $(let args = args.id($id);)?
        $(let args = args.key($key);)?
        $(let args = args.on_press($action);)?
        $(let args = args.enabled($enabled);)?
        $crate::button_with(args)
    }};

    (column([$($kind:ident $args:tt),* $(,)?])) => {{
        $crate::container_with($crate::ContainerArgs::new(
            $crate::Axis::Vertical,
            ($($crate::element!($kind $args),)*),
        ))
    }};

    (column(gap = $gap:expr, [$($kind:ident $args:tt),* $(,)?])) => {{
        $crate::container_with(
            $crate::ContainerArgs::new(
                $crate::Axis::Vertical,
                ($($crate::element!($kind $args),)*),
            )
            .gap($gap),
        )
    }};

    (row([$($kind:ident $args:tt),* $(,)?])) => {{
        $crate::container_with($crate::ContainerArgs::new(
            $crate::Axis::Horizontal,
            ($($crate::element!($kind $args),)*),
        ))
    }};

    (row(gap = $gap:expr, [$($kind:ident $args:tt),* $(,)?])) => {{
        $crate::container_with(
            $crate::ContainerArgs::new(
                $crate::Axis::Horizontal,
                ($($crate::element!($kind $args),)*),
            )
            .gap($gap),
        )
    }};
}
