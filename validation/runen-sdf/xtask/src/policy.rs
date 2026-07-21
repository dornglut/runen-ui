use crate::fs_walk::files_below;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REQUIRED_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "SECURITY.md",
    "README.md",
    "docs/architecture.md",
    "docs/provenance/runenwerk-extraction.md",
    "docs/roadmap.md",
    "docs/status-map.md",
    "docs/tooling/validation.md",
    "docs/work-tracking.md",
];

const ALLOWED_MANIFESTS: &[&str] = &[
    "Cargo.toml",
    "conformance/downstream/Cargo.toml",
    "xtask/Cargo.toml",
];

pub fn validate_repository() -> Result<(), String> {
    let root = repository_root()?;
    validate_required_files(&root)?;
    validate_manifest_inventory(&root)?;
    validate_root_manifest(&root)?;
    validate_path_dependencies(&root)?;
    validate_source_independence(&root)?;
    validate_no_gitlinks(&root)?;
    validate_provenance(&root)
}

fn repository_root() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|error| format!("failed to resolve repository root: {error}"))
}

fn validate_required_files(root: &Path) -> Result<(), String> {
    for required in REQUIRED_FILES {
        let path = root.join(required);
        if !path.is_file() {
            return Err(format!("required repository file is missing: {required}"));
        }
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if metadata.len() == 0 {
            return Err(format!("required repository file is empty: {required}"));
        }
    }
    Ok(())
}

fn validate_manifest_inventory(root: &Path) -> Result<(), String> {
    let allowed = ALLOWED_MANIFESTS
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    let mut found = BTreeSet::new();

    for path in files_below(root)? {
        if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml") {
            found.insert(normalized_relative(root, &path)?);
        }
    }

    if found == allowed {
        Ok(())
    } else {
        Err(format!(
            "unexpected Cargo manifest inventory; expected {allowed:?}, found {found:?}"
        ))
    }
}

fn validate_root_manifest(root: &Path) -> Result<(), String> {
    let manifest = read(root.join("Cargo.toml"))?;
    for required in [
        "name = \"runen-sdf\"",
        "rust-version = \"1.93.0\"",
        "license = \"MIT OR Apache-2.0\"",
        "publish = false",
        "[lints]",
        "workspace = true",
    ] {
        if !manifest.contains(required) {
            return Err(format!("root manifest is missing required declaration: {required}"));
        }
    }
    Ok(())
}

fn validate_path_dependencies(root: &Path) -> Result<(), String> {
    for manifest in ALLOWED_MANIFESTS {
        let manifest_path = root.join(manifest);
        let content = read(&manifest_path)?;
        for line in content.lines() {
            let Some(path_value) = quoted_assignment(line, "path") else {
                continue;
            };
            let parent = manifest_path.parent().ok_or_else(|| {
                format!("manifest has no parent directory: {}", manifest_path.display())
            })?;
            let joined = parent.join(path_value);
            let canonical = joined.canonicalize().map_err(|error| {
                format!("invalid path dependency {}: {error}", joined.display())
            })?;
            if !canonical.starts_with(root) {
                return Err(format!(
                    "path dependency escapes repository: {}",
                    canonical.display()
                ));
            }
        }
    }
    Ok(())
}

fn validate_source_independence(root: &Path) -> Result<(), String> {
    for manifest in ALLOWED_MANIFESTS {
        let path = root.join(manifest);
        let content = read(&path)?;
        reject_tokens(
            root,
            &path,
            &content,
            &["name = \"sdf\"", "package = \"sdf\"", "runenwerk"],
        )?;
    }

    for path in files_below(root)? {
        let relative = normalized_relative(root, &path)?;
        let is_public_rust = relative.starts_with("src/")
            || relative.starts_with("tests/")
            || relative.starts_with("conformance/downstream/src/");
        if !is_public_rust || path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }

        let content = read(&path)?;
        reject_tokens(
            root,
            &path,
            &content,
            &[
                "use sdf::",
                "extern crate sdf",
                "include!",
                "#[path",
                "runenwerk",
            ],
        )?;
    }

    for forbidden_directory in ["crates", "domain"] {
        if root.join(forbidden_directory).exists() {
            return Err(format!(
                "forbidden repository directory exists: {forbidden_directory}"
            ));
        }
    }

    let lockfile = read(root.join("Cargo.lock"))?;
    if lockfile.to_ascii_lowercase().contains("name = \"runenwerk\"") {
        return Err("Cargo.lock contains a Runenwerk package".to_owned());
    }

    Ok(())
}

fn reject_tokens(
    root: &Path,
    path: &Path,
    content: &str,
    forbidden: &[&str],
) -> Result<(), String> {
    let lowercase = content.to_ascii_lowercase();
    for token in forbidden {
        if lowercase.contains(&token.to_ascii_lowercase()) {
            return Err(format!(
                "forbidden token {token:?} in {}",
                normalized_relative(root, path)?
            ));
        }
    }
    Ok(())
}

fn validate_no_gitlinks(root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "-s"])
        .output()
        .map_err(|error| format!("failed to inspect git index: {error}"))?;
    if !output.status.success() {
        return Err("git ls-files -s failed".to_owned());
    }
    let index = String::from_utf8_lossy(&output.stdout);
    if index.lines().any(|line| line.starts_with("160000 ")) {
        Err("git submodules are forbidden".to_owned())
    } else {
        Ok(())
    }
}

fn validate_provenance(root: &Path) -> Result<(), String> {
    let provenance = read(root.join("docs/provenance/runenwerk-extraction.md"))?;
    for required in [
        "8de096259eab30f8d67672010df9190970d0bfc4",
        "domain/sdf",
        "PT-RUNENSDF-003",
    ] {
        if !provenance.contains(required) {
            return Err(format!("provenance is missing required authority: {required}"));
        }
    }
    Ok(())
}

fn quoted_assignment<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!("{key} = \"");
    let start = line.find(&marker)? + marker.len();
    let remainder = &line[start..];
    let end = remainder.find('"')?;
    Some(&remainder[..end])
}

fn read(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn normalized_relative(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|error| format!("failed to relativize {}: {error}", path.display()))
}
