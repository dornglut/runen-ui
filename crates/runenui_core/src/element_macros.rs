// Small `macro_rules!` layer over the typed builder and argument APIs.
// This deliberately does not introduce a parser, compiler, runtime AST, styling
// system, or procedural macro crate.

#[macro_export]
macro_rules! element {
    (text $content:literal $($rest:tt)*) => {{
        $crate::element!(@text_attrs [$crate::TextArgs::new($content)] $($rest)*)
    }};

    (text { $content:expr } $($rest:tt)*) => {{
        $crate::element!(@text_attrs [$crate::TextArgs::new($content)] $($rest)*)
    }};

    (button $label:literal $($rest:tt)*) => {{
        $crate::element!(@button_attrs [$crate::ButtonArgs::new($label)] $($rest)*)
    }};

    (column gap = $gap:literal { $($children:tt)* }) => {{
        $crate::element!(@container_vertical gap = $gap { $($children)* })
    }};

    (column { $($children:tt)* }) => {{
        $crate::element!(@container_vertical { $($children)* })
    }};

    (row gap = $gap:literal { $($children:tt)* }) => {{
        $crate::element!(@container_horizontal gap = $gap { $($children)* })
    }};

    (row { $($children:tt)* }) => {{
        $crate::element!(@container_horizontal { $($children)* })
    }};

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

    (@container_vertical gap = $gap:literal { $($children:tt)* }) => {{
        $crate::container_with(
            $crate::ContainerArgs::new(
                $crate::Axis::Vertical,
                $crate::element!(@children [] $($children)*),
            )
            .gap($gap),
        )
    }};

    (@container_vertical { $($children:tt)* }) => {{
        $crate::container_with($crate::ContainerArgs::new(
            $crate::Axis::Vertical,
            $crate::element!(@children [] $($children)*),
        ))
    }};

    (@container_horizontal gap = $gap:literal { $($children:tt)* }) => {{
        $crate::container_with(
            $crate::ContainerArgs::new(
                $crate::Axis::Horizontal,
                $crate::element!(@children [] $($children)*),
            )
            .gap($gap),
        )
    }};

    (@container_horizontal { $($children:tt)* }) => {{
        $crate::container_with($crate::ContainerArgs::new(
            $crate::Axis::Horizontal,
            $crate::element!(@children [] $($children)*),
        ))
    }};

    (@children [$($children:expr,)*]) => {{
        ($($children,)*)
    }};

    (@children [$($children:expr,)*] text $content:literal $($rest:tt)*) => {{
        $crate::element!(@children_text
            [$($children,)*]
            [$crate::TextArgs::new($content)]
            $($rest)*
        )
    }};

    (@children [$($children:expr,)*] text { $content:expr } $($rest:tt)*) => {{
        $crate::element!(@children_text
            [$($children,)*]
            [$crate::TextArgs::new($content)]
            $($rest)*
        )
    }};

    (@children [$($children:expr,)*] button $label:literal $($rest:tt)*) => {{
        $crate::element!(@children_button
            [$($children,)*]
            [$crate::ButtonArgs::new($label)]
            $($rest)*
        )
    }};

    (@children [$($children:expr,)*] column gap = $gap:literal { $($inner:tt)* } $($rest:tt)*) => {{
        $crate::element!(@children
            [$($children,)* $crate::element!(@container_vertical gap = $gap { $($inner)* }),]
            $($rest)*
        )
    }};

    (@children [$($children:expr,)*] column { $($inner:tt)* } $($rest:tt)*) => {{
        $crate::element!(@children
            [$($children,)* $crate::element!(@container_vertical { $($inner)* }),]
            $($rest)*
        )
    }};

    (@children [$($children:expr,)*] row gap = $gap:literal { $($inner:tt)* } $($rest:tt)*) => {{
        $crate::element!(@children
            [$($children,)* $crate::element!(@container_horizontal gap = $gap { $($inner)* }),]
            $($rest)*
        )
    }};

    (@children [$($children:expr,)*] row { $($inner:tt)* } $($rest:tt)*) => {{
        $crate::element!(@children
            [$($children,)* $crate::element!(@container_horizontal { $($inner)* }),]
            $($rest)*
        )
    }};

    (@children_text [$($children:expr,)*] [$args:expr] id = $id:literal $($rest:tt)*) => {{
        $crate::element!(@children_text [$($children,)*] [$args.id($id)] $($rest)*)
    }};

    (@children_text [$($children:expr,)*] [$args:expr] key = $key:literal $($rest:tt)*) => {{
        $crate::element!(@children_text [$($children,)*] [$args.key($key)] $($rest)*)
    }};

    (@children_text [$($children:expr,)*] [$args:expr] $($rest:tt)*) => {{
        $crate::element!(@children [$($children,)* $crate::text_with($args),] $($rest)*)
    }};

    (@children_button [$($children:expr,)*] [$args:expr] id = $id:literal $($rest:tt)*) => {{
        $crate::element!(@children_button [$($children,)*] [$args.id($id)] $($rest)*)
    }};

    (@children_button [$($children:expr,)*] [$args:expr] key = $key:literal $($rest:tt)*) => {{
        $crate::element!(@children_button [$($children,)*] [$args.key($key)] $($rest)*)
    }};

    (@children_button [$($children:expr,)*] [$args:expr] enabled = $enabled:literal $($rest:tt)*) => {{
        $crate::element!(@children_button [$($children,)*] [$args.enabled($enabled)] $($rest)*)
    }};

    (@children_button [$($children:expr,)*] [$args:expr] action = $action_head:ident $(:: $action_tail:ident)+ $($rest:tt)*) => {{
        $crate::element!(@children_button
            [$($children,)*]
            [$args.on_press($action_head $(:: $action_tail)+)]
            $($rest)*
        )
    }};

    (@children_button [$($children:expr,)*] [$args:expr] action = $action:ident $($rest:tt)*) => {{
        $crate::element!(@children_button [$($children,)*] [$args.on_press($action)] $($rest)*)
    }};

    (@children_button [$($children:expr,)*] [$args:expr] $($rest:tt)*) => {{
        $crate::element!(@children [$($children,)* $crate::button_with($args),] $($rest)*)
    }};

    (@text_attrs [$args:expr]) => {{
        $crate::text_with($args)
    }};

    (@text_attrs [$args:expr] id = $id:literal $($rest:tt)*) => {{
        $crate::element!(@text_attrs [$args.id($id)] $($rest)*)
    }};

    (@text_attrs [$args:expr] key = $key:literal $($rest:tt)*) => {{
        $crate::element!(@text_attrs [$args.key($key)] $($rest)*)
    }};

    (@button_attrs [$args:expr]) => {{
        $crate::button_with($args)
    }};

    (@button_attrs [$args:expr] id = $id:literal $($rest:tt)*) => {{
        $crate::element!(@button_attrs [$args.id($id)] $($rest)*)
    }};

    (@button_attrs [$args:expr] key = $key:literal $($rest:tt)*) => {{
        $crate::element!(@button_attrs [$args.key($key)] $($rest)*)
    }};

    (@button_attrs [$args:expr] enabled = $enabled:literal $($rest:tt)*) => {{
        $crate::element!(@button_attrs [$args.enabled($enabled)] $($rest)*)
    }};

    (@button_attrs [$args:expr] action = $action_head:ident $(:: $action_tail:ident)+) => {{
        $crate::element!(@button_attrs [$args.on_press($action_head $(:: $action_tail)+)])
    }};

    (@button_attrs [$args:expr] action = $action:ident) => {{
        $crate::element!(@button_attrs [$args.on_press($action)])
    }};
}
