use super::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArtifactCacheKey(String);

impl ArtifactCacheKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("artifact cache keys must not be empty")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, ArtifactContractError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ArtifactContractError::EmptyCacheKey);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactContractError {
    EmptyCacheKey,
}

impl fmt::Display for ArtifactContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCacheKey => write!(formatter, "artifact cache key must not be empty"),
        }
    }
}

impl std::error::Error for ArtifactContractError {}
