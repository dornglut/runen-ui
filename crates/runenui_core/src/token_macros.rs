/// Creates a compile-time-validated [`TokenId`](crate::TokenId) literal.
#[macro_export]
macro_rules! token_id {
    ($value:literal) => {{
        const _: () = if !$crate::is_valid_identifier_literal($value) {
            panic!("invalid token ID literal");
        };
        match $crate::TokenId::from_static($value) {
            Ok(value) => value,
            Err(_) => unreachable!("compile-time token validation disagreed with runtime validation"),
        }
    }};
}

/// Creates a compile-time-validated typed color-token reference.
#[macro_export]
macro_rules! color_token {
    ($value:literal) => {
        $crate::ColorToken::new($crate::token_id!($value))
    };
}

/// Creates a compile-time-validated typed brush-token reference.
#[macro_export]
macro_rules! brush_token {
    ($value:literal) => {
        $crate::BrushToken::new($crate::token_id!($value))
    };
}

/// Creates a compile-time-validated typed spacing-token reference.
#[macro_export]
macro_rules! spacing_token {
    ($value:literal) => {
        $crate::SpacingToken::new($crate::token_id!($value))
    };
}

/// Creates a compile-time-validated typed radius-token reference.
#[macro_export]
macro_rules! radius_token {
    ($value:literal) => {
        $crate::RadiusToken::new($crate::token_id!($value))
    };
}

/// Creates a compile-time-validated typed typography-token reference.
#[macro_export]
macro_rules! typography_token {
    ($value:literal) => {
        $crate::TypographyToken::new($crate::token_id!($value))
    };
}

/// Creates a compile-time-validated typed node-outline-token reference.
#[macro_export]
macro_rules! outline_token {
    ($value:literal) => {
        $crate::OutlineToken::new($crate::token_id!($value))
    };
}

/// Creates a compile-time-validated typed ordered-shadow-list-token reference.
#[macro_export]
macro_rules! shadow_token {
    ($value:literal) => {
        $crate::ShadowToken::new($crate::token_id!($value))
    };
}

/// Creates a compile-time-validated typed node-opacity-token reference.
#[macro_export]
macro_rules! opacity_token {
    ($value:literal) => {
        $crate::OpacityToken::new($crate::token_id!($value))
    };
}

/// Creates a compile-time-validated typed node-presentation-transform-token reference.
#[macro_export]
macro_rules! presentation_token {
    ($value:literal) => {
        $crate::PresentationToken::new($crate::token_id!($value))
    };
}
