use std::fmt;

use serde::{Deserialize, Serialize};

use super::diagnostic::{
    CompositionPersistenceDiagnosticCode as Code, CompositionPersistenceDiagnosticStage as Stage,
    CompositionPersistenceDiagnosticSubject as Subject, CompositionPersistenceRejection, rejection,
};

const PREFIX: &str = "blake3:";
const HEX_LENGTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CompositionDigest(String);

impl CompositionDigest {
    pub fn parse(value: impl Into<String>) -> Result<Self, CompositionPersistenceRejection> {
        let value = value.into();
        let Some(hex) = value.strip_prefix(PREFIX) else {
            return Err(invalid_digest(&value));
        };
        if hex.len() != HEX_LENGTH
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(invalid_digest(&value));
        }
        Ok(Self(value))
    }

    pub fn hash(bytes: &[u8]) -> Self {
        Self(format!("{PREFIX}{}", blake3::hash(bytes).to_hex()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn hex(&self) -> &str {
        &self.0[PREFIX.len()..]
    }
}

impl fmt::Display for CompositionDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for CompositionDigest {
    type Error = CompositionPersistenceRejection;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<CompositionDigest> for String {
    fn from(value: CompositionDigest) -> Self {
        value.0
    }
}

fn invalid_digest(value: &str) -> CompositionPersistenceRejection {
    rejection(
        Code::InvalidDigest,
        Stage::Digest,
        Subject::General(value.to_owned()),
        "Use a lowercase blake3:<64-hex> composition digest.",
    )
}
