#![forbid(unsafe_code)]

use std::{fs, path::Path};

const CORE_EDITING: &str = "crates/runenui_core/src/editing.rs";
const RUNTIME_EDITING: &str = "crates/runenui_runtime/src/editing.rs";
const RUNTIME_QUEUE: &str = "crates/runenui_runtime/src/queue.rs";
const EXTERNAL_PROOF: &str = "tests/external_widget/tests/m10c_transactional_editing.rs";

#[test]
fn core_exposes_values_and_protocol_without_live_or_native_authority() -> Result<(), String> {
    let root = workspace_root()?;
    let source = read(&root.join(CORE_EDITING))?;
    for required in [
        "pub struct EditingSessionGeneration",
        "pub struct EditRequestId",
        "pub struct EditIntent",
        "pub struct EditResolution",
        "pub struct EditableContribution<Action>",
        "pub struct UpdateOutput<Action, Protocol: HostProtocol>",
    ] {
        require(&source, required, CORE_EDITING)?;
    }
    for forbidden in [
        "accesskit",
        "winit",
        "arboard",
        "PlainEditor",
        "EditingRegistry",
        "HashMap",
    ] {
        forbid(&source, forbidden, CORE_EDITING)?;
    }
    Ok(())
}

#[test]
fn runtime_has_one_private_session_and_action_origin_authority() -> Result<(), String> {
    let root = workspace_root()?;
    let editing = read(&root.join(RUNTIME_EDITING))?;
    let queue = read(&root.join(RUNTIME_QUEUE))?;
    for required in [
        "pub(crate) struct EditingRegistry<Action>",
        "active: HashMap<MountedNodeId, EditingSession<Action>>",
        "draining: Vec<DrainingSession>",
        "next_session: Option<NonZeroU64>",
        "next_request: Option<NonZeroU64>",
        "fn rebase_pending_suffix(",
    ] {
        require(&editing, required, RUNTIME_EDITING)?;
    }
    for required in [
        "pub(crate) enum ApplicationActionOrigin",
        "Ordinary",
        "Edit(crate::editing::EditActionOrigin)",
    ] {
        require(&queue, required, RUNTIME_QUEUE)?;
    }
    for forbidden in ["accesskit", "winit", "arboard", "PlainEditor"] {
        forbid(&editing, forbidden, RUNTIME_EDITING)?;
    }

    let production_roots = [
        "crates/runenui_core/src",
        "crates/runenui_runtime/src",
        "crates/runenui_text/src",
        "crates/runenui_render_wgpu/src",
        "crates/runenui_winit/src",
    ];
    let mut owners = Vec::new();
    for relative in production_roots {
        collect_rust_files_containing(&root.join(relative), "struct EditingRegistry", &mut owners)?;
    }
    let expected = root.join(RUNTIME_EDITING);
    if owners != [expected] {
        return Err(format!(
            "M10C requires exactly one runtime editing registry owner; found {owners:?}"
        ));
    }
    Ok(())
}

#[test]
fn downstream_proof_uses_only_public_contracts_and_no_friend_seam() -> Result<(), String> {
    let root = workspace_root()?;
    let source = read(&root.join(EXTERNAL_PROOF))?;
    for required in [
        "EditableContribution::new(",
        "UpdateOutput::edit(",
        "submit_text(",
        "runtime.state().text",
    ] {
        require(&source, required, EXTERNAL_PROOF)?;
    }
    for forbidden in [
        "runenui_core::__runtime",
        "internal-test-seams",
        "__editing_",
        "EditingRegistry",
        "ApplicationActionOrigin",
    ] {
        forbid(&source, forbidden, EXTERNAL_PROOF)?;
    }
    Ok(())
}

fn collect_rust_files_containing(
    directory: &Path,
    needle: &str,
    found: &mut Vec<std::path::PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            collect_rust_files_containing(&path, needle, found)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && read(&path)?.contains(needle)
        {
            found.push(path);
        }
    }
    Ok(())
}

fn require(source: &str, required: &str, owner: &str) -> Result<(), String> {
    if source.contains(required) {
        Ok(())
    } else {
        Err(format!(
            "M10C authority lost required seam `{required}` in {owner}"
        ))
    }
}

fn forbid(source: &str, forbidden: &str, owner: &str) -> Result<(), String> {
    if source.contains(forbidden) {
        Err(format!(
            "M10C authority {owner} acquired forbidden competing/native seam `{forbidden}`"
        ))
    } else {
        Ok(())
    }
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn workspace_root() -> Result<std::path::PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest directory has no workspace parent".to_owned())
}
