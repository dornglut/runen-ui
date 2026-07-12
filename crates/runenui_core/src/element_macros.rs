/// Erases one typed view into [`Element`](crate::Element).
///
/// The macro intentionally accepts the same Rust builder expression used
/// without the macro; it has no parallel property grammar.
#[macro_export]
macro_rules! element {
    ($builder:expr $(,)?) => {
        $crate::View::into_element($builder)
    };
}

/// Collects any number of heterogeneous typed builders as erased children.
///
/// This expands directly to a `Vec<Element<_>>`, so it has no tuple arity
/// ceiling. Iterator-produced homogeneous children can be passed directly to
/// [`column`](crate::column) and [`row`](crate::row).
#[macro_export]
macro_rules! children {
    ($($builder:expr),* $(,)?) => {
        vec![$($crate::View::into_element($builder)),*]
    };
}
