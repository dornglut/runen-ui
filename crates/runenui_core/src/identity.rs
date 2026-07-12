//! Validated authored identity types.

use std::{
    cmp::Ordering,
    error::Error,
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
};

/// Reason an authored identifier was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentifierError {
    /// The identifier contained no characters.
    Empty,
    /// The identifier contained only whitespace.
    WhitespaceOnly,
    /// The identifier had leading or trailing whitespace.
    SurroundingWhitespace,
    /// The identifier contained a control character.
    ControlCharacter,
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("identifier must not be empty"),
            Self::WhitespaceOnly => formatter.write_str("identifier must not be whitespace-only"),
            Self::SurroundingWhitespace => {
                formatter.write_str("identifier must not have surrounding whitespace")
            }
            Self::ControlCharacter => {
                formatter.write_str("identifier must not contain control characters")
            }
        }
    }
}

impl Error for IdentifierError {}

/// Validates the canonical authored-identifier grammar.
///
/// Ordinary Unicode text is accepted. Empty or Unicode-whitespace-only text,
/// surrounding Unicode whitespace, and Unicode control characters are rejected.
pub const fn validate_identifier(value: &str) -> Result<(), IdentifierError> {
    if value.is_empty() {
        return Err(IdentifierError::Empty);
    }

    let bytes = value.as_bytes();
    let mut index = 0;
    let mut has_non_whitespace = false;
    let mut first_is_whitespace = false;
    let mut last_is_whitespace = false;
    while index < bytes.len() {
        let (code_point, next_index) = decode_utf8_code_point(bytes, index);
        if is_unicode_control(code_point) {
            return Err(IdentifierError::ControlCharacter);
        }
        let whitespace = match char::from_u32(code_point) {
            Some(character) => character.is_whitespace(),
            None => false,
        };
        if index == 0 {
            first_is_whitespace = whitespace;
        }
        last_is_whitespace = whitespace;
        if !whitespace {
            has_non_whitespace = true;
        }
        index = next_index;
    }

    if !has_non_whitespace {
        return Err(IdentifierError::WhitespaceOnly);
    }
    if first_is_whitespace || last_is_whitespace {
        return Err(IdentifierError::SurroundingWhitespace);
    }
    Ok(())
}

// `str::chars` and `char::is_control` are not const on the supported Rust
// toolchains. Decoding the already-valid UTF-8 from `str` here lets literal and
// dynamic construction share this validator. Unicode General Category Cc is
// exactly the C0 and C1 control ranges.
const fn decode_utf8_code_point(bytes: &[u8], index: usize) -> (u32, usize) {
    let first = bytes[index];
    if first < 0x80 {
        return (first as u32, index + 1);
    }
    if first < 0xe0 {
        return (
            ((first & 0x1f) as u32) << 6 | (bytes[index + 1] & 0x3f) as u32,
            index + 2,
        );
    }
    if first < 0xf0 {
        return (
            ((first & 0x0f) as u32) << 12
                | ((bytes[index + 1] & 0x3f) as u32) << 6
                | (bytes[index + 2] & 0x3f) as u32,
            index + 3,
        );
    }
    (
        ((first & 0x07) as u32) << 18
            | ((bytes[index + 1] & 0x3f) as u32) << 12
            | ((bytes[index + 2] & 0x3f) as u32) << 6
            | (bytes[index + 3] & 0x3f) as u32,
        index + 4,
    )
}

const fn is_unicode_control(code_point: u32) -> bool {
    code_point <= 0x1f || (code_point >= 0x7f && code_point <= 0x9f)
}

#[doc(hidden)]
#[must_use]
pub const fn is_valid_identifier_literal(value: &str) -> bool {
    validate_identifier(value).is_ok()
}

macro_rules! define_identifier {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(IdentifierText);

        impl $name {
            /// Validates and owns a dynamic identifier.
            ///
            /// # Errors
            ///
            /// Returns [`IdentifierError`] when the text is empty, whitespace-only,
            /// surrounded by whitespace, or contains control characters.
            pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(IdentifierText::owned(value)))
            }

            /// Validates a static identifier without allocation.
            ///
            /// # Errors
            ///
            /// Returns [`IdentifierError`] under the same rules as [`Self::new`].
            pub const fn from_static(value: &'static str) -> Result<Self, IdentifierError> {
                match validate_identifier(value) {
                    Ok(()) => Ok(Self(IdentifierText::from_static(value))),
                    Err(error) => Err(error),
                }
            }

            /// Returns the identifier text.
            #[must_use]
            pub const fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl FromStr for $name {
            type Err = IdentifierError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }
    };
}

#[derive(Clone)]
pub enum IdentifierText {
    Static(&'static str),
    Owned(Box<str>),
}

impl IdentifierText {
    pub(crate) fn owned(value: String) -> Self {
        Self::Owned(value.into_boxed_str())
    }

    pub(crate) const fn from_static(value: &'static str) -> Self {
        Self::Static(value)
    }

    pub(crate) const fn as_str(&self) -> &str {
        match self {
            Self::Static(value) => value,
            Self::Owned(value) => value,
        }
    }
}

impl fmt::Debug for IdentifierText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(formatter)
    }
}

impl PartialEq for IdentifierText {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for IdentifierText {}

impl PartialOrd for IdentifierText {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IdentifierText {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for IdentifierText {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

define_identifier!(
    ElementId,
    "Validated authored debug, test, automation, and integration identity."
);
define_identifier!(
    ElementKey,
    "Validated authored sibling identity reserved for future reconciliation."
);

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::ElementId {}
    impl Sealed for super::ElementKey {}
    impl Sealed for String {}
    impl Sealed for &str {}
}

/// Sealed conversion accepted by typed builders for authored element IDs.
///
/// Validated [`ElementId`] values retain their allocation-free static storage;
/// string inputs are validated and owned. Invalid strings remain available to
/// the builder's deterministic authoring diagnostics.
pub trait IntoElementId: sealed::Sealed {
    #[doc(hidden)]
    fn into_element_id(self) -> Result<ElementId, (String, IdentifierError)>;
}

/// Sealed conversion accepted by typed builders for authored sibling keys.
pub trait IntoElementKey: sealed::Sealed {
    #[doc(hidden)]
    fn into_element_key(self) -> Result<ElementKey, (String, IdentifierError)>;
}

macro_rules! impl_identifier_input {
    ($trait:ident, $method:ident, $identifier:ident) => {
        impl $trait for $identifier {
            fn $method(self) -> Result<$identifier, (String, IdentifierError)> {
                Ok(self)
            }
        }

        impl $trait for String {
            fn $method(self) -> Result<$identifier, (String, IdentifierError)> {
                match $identifier::new(self.clone()) {
                    Ok(value) => Ok(value),
                    Err(error) => Err((self, error)),
                }
            }
        }

        impl $trait for &str {
            fn $method(self) -> Result<$identifier, (String, IdentifierError)> {
                self.to_owned().$method()
            }
        }
    };
}

impl_identifier_input!(IntoElementId, into_element_id, ElementId);
impl_identifier_input!(IntoElementKey, into_element_key, ElementKey);

#[cfg(test)]
mod tests {
    use super::{ElementId, ElementKey, IdentifierError};

    #[test]
    fn identifiers_accept_meaningful_text_and_reject_invalid_forms() {
        assert_eq!(
            ElementId::new("counter.increment").map(|id| id.as_str().to_owned()),
            Ok("counter.increment".to_owned())
        );
        assert_eq!(
            ElementKey::new("item-42").map(|id| id.as_str().to_owned()),
            Ok("item-42".to_owned())
        );

        let cases = [
            ("", IdentifierError::Empty),
            ("   ", IdentifierError::WhitespaceOnly),
            (" leading", IdentifierError::SurroundingWhitespace),
            ("trailing ", IdentifierError::SurroundingWhitespace),
            ("line\nbreak", IdentifierError::ControlCharacter),
        ];
        for (value, expected) in cases {
            assert_eq!(ElementId::new(value), Err(expected));
            assert_eq!(ElementKey::new(value), Err(expected));
        }
    }
}
