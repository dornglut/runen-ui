#![forbid(unsafe_code)]

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const RUNTIME_MANIFEST: &str = "crates/runenui_runtime/Cargo.toml";
const RUNTIME_SOURCE: &str = "crates/runenui_runtime/src";
const LOCKFILE: &str = "Cargo.lock";
const EXACT_TAFFY_DECLARATION: &str = "taffy = { version = \"=0.14.0\", default-features = false, features = [\"flexbox\", \"grid\", \"block_layout\", \"content_size\"] }";
const EXPECTED_TAFFY_FEATURES: &[&str] =
    &["alloc", "block_layout", "content_size", "flexbox", "grid"];

#[test]
fn taffy_adoption_is_absent_or_exactly_bounded() -> Result<(), String> {
    let root = workspace_root()?;
    let runtime_manifest = read(&root.join(RUNTIME_MANIFEST))?;
    let lockfile = read(&root.join(LOCKFILE))?;

    let declarations = taffy_manifest_declarations(&root)?;
    let runtime_declaration = declarations
        .iter()
        .find(|declaration| declaration.path == Path::new(RUNTIME_MANIFEST));

    match runtime_declaration {
        None => {
            if !declarations.is_empty() {
                return Err(format!(
                    "M8C Taffy authority must enter only through {RUNTIME_MANIFEST}; found:\n{}",
                    format_declarations(&declarations)
                ));
            }
            if lockfile_contains_taffy(&lockfile) {
                return Err(
                    "Cargo.lock contains Taffy before the runtime owns the reviewed dependency"
                        .to_owned(),
                );
            }
            Ok(())
        }
        Some(declaration) => {
            if declarations.len() != 1 {
                return Err(format!(
                    "M8C permits exactly one direct Taffy dependency in {RUNTIME_MANIFEST}; found:\n{}",
                    format_declarations(&declarations)
                ));
            }
            if declaration.text != EXACT_TAFFY_DECLARATION {
                return Err(format!(
                    "M8C runtime Taffy dependency must be the exact reviewed declaration:\n{EXACT_TAFFY_DECLARATION}\nfound:\n{}",
                    declaration.text
                ));
            }
            if !runtime_manifest.contains(EXACT_TAFFY_DECLARATION) {
                return Err(
                    "exact reviewed Taffy declaration is not present in runtime manifest"
                        .to_owned(),
                );
            }
            audit_lockfile_taffy(&lockfile)?;
            audit_resolved_taffy_features(&root)
        }
    }
}

#[test]
fn runtime_source_cannot_retain_or_export_taffy_tree_authority() -> Result<(), String> {
    let root = workspace_root()?;
    let source_root = root.join(RUNTIME_SOURCE);
    let mut files = Vec::new();
    collect_files_with_extension(&source_root, "rs", &mut files)?;
    files.sort();

    if files.is_empty() {
        return Err(format!(
            "M8C source audit found no Rust files under {RUNTIME_SOURCE}"
        ));
    }

    let mut failures = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .map_err(|error| format!("failed to relativize {}: {error}", file.display()))?;
        let contents = read(&file)?;
        for forbidden in [
            "TaffyTree",
            "pub use taffy",
            "pub use ::taffy",
            "pub extern crate taffy",
        ] {
            if contents.contains(forbidden) {
                failures.push(format!(
                    "{} contains forbidden Taffy authority/export token `{forbidden}`",
                    relative.display()
                ));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "M8C Taffy source authority audit failed:\n{}",
            failures.join("\n")
        ))
    }
}

#[derive(Debug)]
struct ManifestDeclaration {
    path: PathBuf,
    text: String,
}

fn taffy_manifest_declarations(root: &Path) -> Result<Vec<ManifestDeclaration>, String> {
    let mut manifests = Vec::new();
    collect_files_with_extension(root, "toml", &mut manifests)?;
    manifests.retain(|path| {
        path.file_name()
            .is_some_and(|name| name == OsStr::new("Cargo.toml"))
    });
    manifests.sort();

    let mut declarations = Vec::new();
    for manifest in manifests {
        let relative = manifest
            .strip_prefix(root)
            .map_err(|error| format!("failed to relativize {}: {error}", manifest.display()))?
            .to_path_buf();
        let contents = read(&manifest)?;
        let mut dependency_table_is_taffy = false;

        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                dependency_table_is_taffy = matches!(
                    trimmed,
                    "[dependencies.taffy]"
                        | "[dev-dependencies.taffy]"
                        | "[build-dependencies.taffy]"
                        | "[workspace.dependencies.taffy]"
                );
                if dependency_table_is_taffy {
                    declarations.push(ManifestDeclaration {
                        path: relative.clone(),
                        text: trimmed.to_owned(),
                    });
                }
                continue;
            }

            if dependency_table_is_taffy && !trimmed.is_empty() && !trimmed.starts_with('#') {
                continue;
            }

            if is_taffy_dependency_line(trimmed) {
                declarations.push(ManifestDeclaration {
                    path: relative.clone(),
                    text: trimmed.to_owned(),
                });
            }
        }
    }

    Ok(declarations)
}

fn is_taffy_dependency_line(line: &str) -> bool {
    if line.is_empty() || line.starts_with('#') {
        return false;
    }

    let compact: String = line
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    compact.starts_with("taffy=") || compact.contains("package=\"taffy\"")
}

fn audit_lockfile_taffy(lockfile: &str) -> Result<(), String> {
    let blocks = package_blocks(lockfile);
    let taffy_blocks: Vec<&str> = blocks
        .into_iter()
        .filter(|block| block.lines().any(|line| line == "name = \"taffy\""))
        .collect();

    if taffy_blocks.len() != 1 {
        return Err(format!(
            "Cargo.lock must contain exactly one Taffy package after adoption; found {}",
            taffy_blocks.len()
        ));
    }

    let block = taffy_blocks[0];
    for expected in [
        "name = \"taffy\"",
        "version = \"0.14.0\"",
        "source = \"registry+https://github.com/rust-lang/crates.io-index\"",
        "checksum = \"639627c87f43b9181c811f40a6296409e093a17bc761214cba3c15df74f86b99\"",
    ] {
        if !block.lines().any(|line| line == expected) {
            return Err(format!(
                "Cargo.lock Taffy package is missing exact package identity line `{expected}`:\n{block}"
            ));
        }
    }

    Ok(())
}

fn audit_resolved_taffy_features(root: &Path) -> Result<(), String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsStr::new("cargo").to_os_string());
    let output = Command::new(cargo)
        .current_dir(root)
        .args([
            "tree",
            "--locked",
            "-p",
            "runenui_runtime",
            "-e",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p} {f}",
        ])
        .output()
        .map_err(|error| format!("failed to execute Cargo feature audit: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "Cargo production Taffy feature audit failed with {}:\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let matches: Vec<Vec<&str>> = stdout
        .lines()
        .filter_map(|line| {
            let (package, features) = line.rsplit_once(' ')?;
            (package == "taffy v0.14.0").then(|| {
                features
                    .strip_prefix('(')
                    .and_then(|value| value.strip_suffix(')'))
                    .unwrap_or(features)
                    .split(',')
                    .filter(|feature| !feature.is_empty())
                    .collect()
            })
        })
        .collect();

    if matches.len() != 1 {
        return Err(format!(
            "expected exactly one production Taffy feature line, found {}:\n{}",
            matches.len(),
            stdout
        ));
    }

    let mut features = matches[0].clone();
    features.sort_unstable();
    if features != EXPECTED_TAFFY_FEATURES {
        return Err(format!(
            "production Taffy feature set differs from the exact M8C boundary:\nexpected: {}\nfound: {}\nfull cargo tree output:\n{}",
            EXPECTED_TAFFY_FEATURES.join(","),
            features.join(","),
            stdout
        ));
    }

    Ok(())
}

fn lockfile_contains_taffy(lockfile: &str) -> bool {
    package_blocks(lockfile)
        .into_iter()
        .any(|block| block.lines().any(|line| line == "name = \"taffy\""))
}

fn package_blocks(lockfile: &str) -> Vec<&str> {
    lockfile
        .split("\n[[package]]\n")
        .skip(1)
        .map(|block| {
            let end = block.find("\n\n").unwrap_or(block.len());
            &block[..end]
        })
        .collect()
}

fn format_declarations(declarations: &[ManifestDeclaration]) -> String {
    declarations
        .iter()
        .map(|declaration| format!("{}: {}", declaration.path.display(), declaration.text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_files_with_extension(
    root: &Path,
    extension: &str,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in fs::read_dir(root)
        .map_err(|error| format!("failed to read directory {}: {error}", root.display()))?
    {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect directory entry under {}: {error}",
                root.display()
            )
        })?;
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if name != OsStr::new(".git") && name != OsStr::new("target") {
                collect_files_with_extension(&path, extension, files)?;
            }
        } else if path
            .extension()
            .is_some_and(|value| value == OsStr::new(extension))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest directory has no workspace parent".to_owned())
}
