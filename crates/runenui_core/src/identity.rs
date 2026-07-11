//! Validated authored identity types.

use std::{error::Error, fmt, str::FromStr};

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

pub const fn validate_identifier(value: &str) -> Result<(), IdentifierError> {
    if value.is_empty() {
        return Err(IdentifierError::Empty);
    }

    let bytes = value.as_bytes();
    let mut index = 0;
    let mut has_non_whitespace = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte < b' ' || byte == 0x7f {
            return Err(IdentifierError::ControlCharacter);
        }
        if !matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
            has_non_whitespace = true;
        }
        index += 1;
    }

    if !has_non_whitespace {
        return Err(IdentifierError::WhitespaceOnly);
    }
    if matches!(bytes[0], b' ' | b'\t' | b'\n' | b'\r')
        || matches!(bytes[bytes.len() - 1], b' ' | b'\t' | b'\n' | b'\r')
    {
        return Err(IdentifierError::SurroundingWhitespace);
    }
    Ok(())
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
                Ok(Self(IdentifierText::Owned(value.into_boxed_str())))
            }

            /// Validates a static identifier without allocation.
            ///
            /// # Errors
            ///
            /// Returns [`IdentifierError`] under the same rules as [`Self::new`].
            pub const fn from_static(value: &'static str) -> Result<Self, IdentifierError> {
                match validate_identifier(value) {
                    Ok(()) => Ok(Self(IdentifierText::Static(value))),
                    Err(error) => Err(error),
                }
            }

            /// Returns the identifier text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                match &self.0 {
                    IdentifierText::Static(value) => value,
                    IdentifierText::Owned(value) => value,
                }
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

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum IdentifierText {
    Static(&'static str),
    Owned(Box<str>),
}

define_identifier!(
    ElementId,
    "Validated authored debug, test, automation, and integration identity."
);
define_identifier!(
    ElementKey,
    "Validated authored sibling identity reserved for future reconciliation."
);

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
