use crate::fs_walk::files_below;
use std::fs;
use std::path::Path;

pub fn validate_markdown_links() -> Result<(), String> {
    let root = std::env::current_dir()
        .map_err(|error| format!("failed to resolve repository root: {error}"))?;

    for markdown in files_below(&root)?
        .into_iter()
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("md"))
    {
        validate_file(&root, &markdown)?;
    }

    Ok(())
}

fn validate_file(root: &Path, markdown: &Path) -> Result<(), String> {
    let content = fs::read_to_string(markdown)
        .map_err(|error| format!("failed to read {}: {error}", markdown.display()))?;

    for (line_index, line) in content.lines().enumerate() {
        for target in link_targets(line) {
            if should_skip(target) {
                continue;
            }
            let relative_target = target
                .split('#')
                .next()
                .unwrap_or_default()
                .split('?')
                .next()
                .unwrap_or_default();
            if relative_target.is_empty() {
                continue;
            }
            let parent = markdown.parent().unwrap_or(root);
            let resolved = parent.join(relative_target);
            if !resolved.exists() {
                return Err(format!(
                    "broken Markdown link in {}:{}: {}",
                    display_relative(root, markdown),
                    line_index + 1,
                    target
                ));
            }
        }
    }

    Ok(())
}

fn link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut remainder = line;
    while let Some(open) = remainder.find("](") {
        let after_open = &remainder[open + 2..];
        let Some(close) = after_open.find(')') else {
            break;
        };
        let target = after_open[..close]
            .split_whitespace()
            .next()
            .unwrap_or_default();
        targets.push(target.trim_matches('<').trim_matches('>'));
        remainder = &after_open[close + 1..];
    }
    targets
}

fn should_skip(target: &str) -> bool {
    target.is_empty()
        || target.starts_with('#')
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
