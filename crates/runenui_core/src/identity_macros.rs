/// Creates a compile-time-validated [`ElementId`](crate::ElementId) literal.
#[macro_export]
macro_rules! element_id {
    ($value:literal) => {{
        const _: () = if !$crate::is_valid_identifier_literal($value) {
            panic!("invalid element ID literal");
        };
        match $crate::ElementId::new($value) {
            Ok(value) => value,
            Err(_) => unreachable!("compile-time ID validation disagreed with runtime validation"),
        }
    }};
}

/// Creates a compile-time-validated [`ElementKey`](crate::ElementKey) literal.
#[macro_export]
macro_rules! element_key {
    ($value:literal) => {{
        const _: () = if !$crate::is_valid_identifier_literal($value) {
            panic!("invalid element key literal");
        };
        match $crate::ElementKey::new($value) {
            Ok(value) => value,
            Err(_) => unreachable!("compile-time key validation disagreed with runtime validation"),
        }
    }};
}
