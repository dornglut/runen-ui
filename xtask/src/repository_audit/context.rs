use std::{collections::BTreeSet, fs, path::Path};

use super::{Finding, VolatilityPolicy, audit_volatility, path_text};

const EXPORTER_PATH: &str = "tools/context/export_repo_context.py";
const PROFILE_DIRECTORY: &str = "tools/context/profiles";
const EXPECTED_PROFILE_FILES: &[&str] = &[
    "full-audit.toml",
    "implementation-review.toml",
    "offline-review.toml",
];
const DEFAULT_PROFILE_PREFIX: &str = "DEFAULT_PROFILE =";
const DEFAULT_PROFILE_DECLARATION: &str = "DEFAULT_PROFILE = \"offline-review\"";

pub(super) fn audit(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    audit_profile_inventory(root, findings)?;
    audit_default_profile(root, findings)?;
    audit_profile_volatility(root, findings)
}

fn audit_profile_inventory(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let directory = root.join(PROFILE_DIRECTORY);
    let entries = fs::read_dir(&directory)
        .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?;
    let mut found = BTreeSet::new();

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an entry in {}: {error}",
                directory.display()
            )
        })?;
        if entry.path().is_file() {
            let name = entry.file_name().into_string().map_err(|_| {
                format!("non-UTF-8 profile file name below {}", directory.display())
            })?;
            found.insert(name);
        }
    }

    let expected = EXPECTED_PROFILE_FILES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    if found != expected {
        findings.push(Finding::fatal(
            "context.profile_inventory",
            Some(PROFILE_DIRECTORY.to_owned()),
            format!("expected context profile files {expected:?}, found {found:?}"),
        ));
    }

    Ok(())
}

fn audit_default_profile(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let contents = fs::read_to_string(root.join(EXPORTER_PATH))
        .map_err(|error| format!("failed to read {EXPORTER_PATH}: {error}"))?;
    let declarations = default_profile_declarations(&contents);
    if declarations != [DEFAULT_PROFILE_DECLARATION] {
        findings.push(Finding::fatal(
            "context.default_profile",
            Some(EXPORTER_PATH.to_owned()),
            format!(
                "context exporter must contain exactly one active {DEFAULT_PROFILE_DECLARATION:?} declaration; found {declarations:?}"
            ),
        ));
    }
    Ok(())
}

fn default_profile_declarations(contents: &str) -> Vec<&str> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with(DEFAULT_PROFILE_PREFIX))
        .collect()
}

fn audit_profile_volatility(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    for file_name in EXPECTED_PROFILE_FILES {
        let relative = Path::new(PROFILE_DIRECTORY).join(file_name);
        let contents = fs::read_to_string(root.join(&relative)).map_err(|error| {
            format!(
                "failed to read context profile {}: {error}",
                relative.display()
            )
        })?;
        audit_volatility(
            &path_text(&relative),
            &contents,
            VolatilityPolicy::StrictCurrent,
            findings,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_PROFILE_DECLARATION, default_profile_declarations};

    #[test]
    fn commented_expected_default_does_not_mask_active_drift() {
        let declarations = default_profile_declarations(
            "# DEFAULT_PROFILE = \"offline-review\"\nDEFAULT_PROFILE = \"ai-core\"\n",
        );
        assert_eq!(declarations, ["DEFAULT_PROFILE = \"ai-core\""]);
        assert_ne!(declarations, [DEFAULT_PROFILE_DECLARATION]);
    }
}
