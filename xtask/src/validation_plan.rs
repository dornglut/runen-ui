//! Repository-owned canonical validation plan and hosted execution partitions.

use std::{fs, path::Path};

pub(crate) const PARTITION_MANIFEST_PATH: &str = "validation-partitions.txt";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValidationPartition {
    WorkspaceTests,
    PublicContract,
    RepositoryQuality,
}

impl ValidationPartition {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::WorkspaceTests => "workspace-tests",
            Self::PublicContract => "public-contract",
            Self::RepositoryQuality => "repository-quality",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        PARTITIONS
            .iter()
            .copied()
            .find(|partition| partition.as_str() == value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValidationPhase {
    StableMetadata,
    StableFormatting,
    WorkspaceTests,
    StableClippy,
    PublicContract,
    LicensingAndPublishPolicy,
    DocumentationLinks,
    RepositoryAudit,
}

impl ValidationPhase {
    const fn partition(self) -> ValidationPartition {
        match self {
            Self::WorkspaceTests => ValidationPartition::WorkspaceTests,
            Self::PublicContract => ValidationPartition::PublicContract,
            Self::StableMetadata
            | Self::StableFormatting
            | Self::StableClippy
            | Self::LicensingAndPublishPolicy
            | Self::DocumentationLinks
            | Self::RepositoryAudit => ValidationPartition::RepositoryQuality,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValidationSelection {
    Complete,
    Partition(ValidationPartition),
}

impl ValidationSelection {
    fn includes(self, phase: ValidationPhase) -> bool {
        match self {
            Self::Complete => true,
            Self::Partition(partition) => phase.partition() == partition,
        }
    }
}

const PARTITIONS: &[ValidationPartition] = &[
    ValidationPartition::WorkspaceTests,
    ValidationPartition::PublicContract,
    ValidationPartition::RepositoryQuality,
];

const VALIDATION_PHASES: &[ValidationPhase] = &[
    ValidationPhase::StableMetadata,
    ValidationPhase::StableFormatting,
    ValidationPhase::WorkspaceTests,
    ValidationPhase::StableClippy,
    ValidationPhase::PublicContract,
    ValidationPhase::LicensingAndPublishPolicy,
    ValidationPhase::DocumentationLinks,
    ValidationPhase::RepositoryAudit,
];

pub(crate) fn parse_selection(
    arguments: impl Iterator<Item = String>,
) -> Result<ValidationSelection, String> {
    let arguments = arguments.collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => Ok(ValidationSelection::Complete),
        [flag, partition] if flag == "--partition" => ValidationPartition::from_str(partition)
            .map(ValidationSelection::Partition)
            .ok_or_else(|| format!("unknown validation partition {partition:?}; {}", usage())),
        _ => Err(usage()),
    }
}

pub(crate) fn phases(selection: ValidationSelection) -> impl Iterator<Item = ValidationPhase> {
    VALIDATION_PHASES
        .iter()
        .copied()
        .filter(move |phase| selection.includes(*phase))
}

pub(crate) fn usage() -> String {
    let partitions = PARTITIONS
        .iter()
        .map(|partition| partition.as_str())
        .collect::<Vec<_>>()
        .join("|");
    format!("usage: cargo validate [--partition <{partitions}>]")
}

pub(crate) fn validate_manifest(root: &Path) -> Result<(), String> {
    let path = root.join(PARTITION_MANIFEST_PATH);
    let actual = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    validate_manifest_contents(&actual)
}

pub(crate) fn expected_manifest() -> String {
    let mut manifest = PARTITIONS
        .iter()
        .map(|partition| partition.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    manifest.push('\n');
    manifest
}

fn validate_manifest_contents(actual: &str) -> Result<(), String> {
    let expected = expected_manifest();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{PARTITION_MANIFEST_PATH} must exactly match the repository-owned validation partition plan; expected {expected:?}, found {actual:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        VALIDATION_PHASES, ValidationPartition, ValidationPhase, ValidationSelection,
        expected_manifest, parse_selection, phases, validate_manifest_contents,
    };

    #[test]
    fn canonical_manifest_is_derived_from_the_partition_registry() {
        assert_eq!(
            expected_manifest(),
            "workspace-tests\npublic-contract\nrepository-quality\n"
        );
        assert!(validate_manifest_contents(&expected_manifest()).is_ok());
    }

    #[test]
    fn complete_selection_preserves_the_accepted_serial_phase_order() {
        assert_eq!(
            phases(ValidationSelection::Complete).collect::<Vec<_>>(),
            VALIDATION_PHASES
        );
    }

    #[test]
    fn repository_quality_membership_is_owned_by_the_phase_registry() {
        assert_eq!(
            phases(ValidationSelection::Partition(
                ValidationPartition::RepositoryQuality
            ))
            .collect::<Vec<_>>(),
            [
                ValidationPhase::StableMetadata,
                ValidationPhase::StableFormatting,
                ValidationPhase::StableClippy,
                ValidationPhase::LicensingAndPublishPolicy,
                ValidationPhase::DocumentationLinks,
                ValidationPhase::RepositoryAudit,
            ]
        );
    }

    #[test]
    fn manifest_omission_fails_closed() {
        assert!(validate_manifest_contents("workspace-tests\npublic-contract\n").is_err());
    }

    #[test]
    fn manifest_addition_fails_closed() {
        assert!(
            validate_manifest_contents(
                "workspace-tests\npublic-contract\nrepository-quality\nextra\n"
            )
            .is_err()
        );
    }

    #[test]
    fn manifest_reordering_fails_closed() {
        assert!(
            validate_manifest_contents("public-contract\nworkspace-tests\nrepository-quality\n")
                .is_err()
        );
    }

    #[test]
    fn known_partition_is_selected() {
        assert_eq!(
            parse_selection(
                ["--partition", "workspace-tests"]
                    .into_iter()
                    .map(str::to_owned)
            ),
            Ok(ValidationSelection::Partition(
                ValidationPartition::WorkspaceTests
            ))
        );
    }

    #[test]
    fn unknown_partition_fails_closed() {
        let result = parse_selection(["--partition", "missing"].into_iter().map(str::to_owned));
        assert!(matches!(result, Err(ref error) if error.contains("unknown validation partition")));
    }

    #[test]
    fn malformed_partition_arguments_fail_closed() {
        assert!(parse_selection(["--partition"].into_iter().map(str::to_owned)).is_err());
        assert!(
            parse_selection(
                ["--partition", "workspace-tests", "extra"]
                    .into_iter()
                    .map(str::to_owned)
            )
            .is_err()
        );
    }
}
