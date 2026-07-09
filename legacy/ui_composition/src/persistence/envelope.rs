use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    AppProfileId, CompositionDefinitionId, CompositionDefinitionV1, DefinitionRevision,
    ExtensionProfileId,
};

use super::{
    CompositionDigest, CompositionPersistenceDiagnosticCode as Code,
    CompositionPersistenceDiagnosticStage as Stage,
    CompositionPersistenceDiagnosticSubject as Subject, CompositionPersistenceRejection,
};

macro_rules! schema_version {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(try_from = "u32", into = "u32")]
        pub struct $name(u32);

        impl $name {
            pub fn new(value: u32) -> Result<Self, CompositionPersistenceRejection> {
                if value == 0 {
                    return Err(super::diagnostic::rejection(
                        Code::InvalidVersion,
                        Stage::Compatibility,
                        Subject::General(stringify!($name).to_owned()),
                        "Use a non-zero schema version.",
                    ));
                }
                Ok(Self(value))
            }

            pub const fn get(self) -> u32 {
                self.0
            }
        }

        impl TryFrom<u32> for $name {
            type Error = CompositionPersistenceRejection;

            fn try_from(value: u32) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for u32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

schema_version!(AppSchemaVersion);
schema_version!(ExtensionSchemaVersion);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionCompatibility {
    app_profile: AppProfileId,
    minimum_app_schema_version: AppSchemaVersion,
    maximum_app_schema_version: AppSchemaVersion,
}

impl CompositionCompatibility {
    pub fn new(
        app_profile: AppProfileId,
        minimum_app_schema_version: AppSchemaVersion,
        maximum_app_schema_version: AppSchemaVersion,
    ) -> Result<Self, CompositionPersistenceRejection> {
        if minimum_app_schema_version > maximum_app_schema_version {
            return Err(super::diagnostic::rejection(
                Code::InvalidCompatibility,
                Stage::Compatibility,
                Subject::General(app_profile.to_string()),
                "Set the minimum app schema version at or below the maximum.",
            ));
        }
        Ok(Self {
            app_profile,
            minimum_app_schema_version,
            maximum_app_schema_version,
        })
    }

    pub fn app_profile(&self) -> &AppProfileId {
        &self.app_profile
    }

    pub const fn minimum_app_schema_version(&self) -> AppSchemaVersion {
        self.minimum_app_schema_version
    }

    pub const fn maximum_app_schema_version(&self) -> AppSchemaVersion {
        self.maximum_app_schema_version
    }

    pub fn accepts(&self, requirement: &CompositionCompatibilityRequirement) -> bool {
        self.app_profile == requirement.app_profile
            && requirement.app_schema_version >= self.minimum_app_schema_version
            && requirement.app_schema_version <= self.maximum_app_schema_version
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionCompatibilityRequirement {
    pub app_profile: AppProfileId,
    pub app_schema_version: AppSchemaVersion,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionSharedMetadataV1 {
    pub layout_id: CompositionDefinitionId,
    pub definition_revision: DefinitionRevision,
    pub core_schema_version: u16,
    pub compatibility: CompositionCompatibility,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionExtensionIdentity {
    pub profile: ExtensionProfileId,
    pub schema_version: ExtensionSchemaVersion,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionExtensionLinkV1 {
    pub identity: CompositionExtensionIdentity,
    pub core_payload_digest: CompositionDigest,
    pub extension_payload_digest: CompositionDigest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionCoreEnvelopeV1 {
    pub envelope_schema_version: u16,
    pub shared: CompositionSharedMetadataV1,
    pub core_payload_digest: CompositionDigest,
    pub extension_links: Vec<CompositionExtensionLinkV1>,
    pub definition: CompositionDefinitionV1,
}

impl CompositionCoreEnvelopeV1 {
    pub const ENVELOPE_SCHEMA_VERSION: u16 = 1;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionExtensionEnvelopeV1 {
    pub envelope_schema_version: u16,
    pub shared: CompositionSharedMetadataV1,
    pub link: CompositionExtensionLinkV1,
    pub payload_ron: String,
}

impl CompositionExtensionEnvelopeV1 {
    pub const ENVELOPE_SCHEMA_VERSION: u16 = 1;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalExtensionPayload {
    identity: CompositionExtensionIdentity,
    canonical_ron: String,
}

impl CanonicalExtensionPayload {
    pub fn new(
        profile: ExtensionProfileId,
        schema_version: ExtensionSchemaVersion,
        canonical_ron: impl Into<String>,
    ) -> Result<Self, CompositionPersistenceRejection> {
        let canonical_ron = canonical_ron.into();
        super::canonical::validate_extension_payload(&canonical_ron, &profile)?;
        Ok(Self {
            identity: CompositionExtensionIdentity {
                profile,
                schema_version,
            },
            canonical_ron,
        })
    }

    pub fn identity(&self) -> &CompositionExtensionIdentity {
        &self.identity
    }

    pub fn canonical_ron(&self) -> &str {
        &self.canonical_ron
    }
}

impl fmt::Display for CompositionExtensionIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.v{}", self.profile, self.schema_version.get())
    }
}
