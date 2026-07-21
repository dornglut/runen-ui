use std::fs;
use std::path::{Path, PathBuf};

pub fn files_below(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn visit(root: &Path, path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?
    {
        let entry = entry.map_err(|error| {
            format!("failed to read an entry below {}: {error}", path.display())
        })?;
        let entry_path = entry.path();
        let relative = entry_path
            .strip_prefix(root)
            .map_err(|error| format!("failed to relativize {}: {error}", entry_path.display()))?;

        if is_ignored(relative) {
            continue;
        }

        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", entry_path.display()))?;

        if file_type.is_symlink() {
            return Err(format!(
                "repository symlinks are not permitted: {}",
                relative.display()
            ));
        }

        if file_type.is_dir() {
            visit(root, &entry_path, files)?;
        } else if file_type.is_file() {
            files.push(entry_path);
        }
    }

    Ok(())
}

fn is_ignored(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str();
        value == ".git" || value == "target"
    })
}
