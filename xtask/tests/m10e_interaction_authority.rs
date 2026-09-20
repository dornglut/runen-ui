#![forbid(unsafe_code)]

use std::{fs, path::Path};

const CORE_POINTER: &str = "crates/runenui_core/src/pointer.rs";
const MOUNTED_INTERACTION: &str = "crates/runenui_runtime/src/mounted/interaction.rs";
const SURFACE_INTERACTION: &str = "crates/runenui_runtime/src/surface/interaction.rs";
const CONTROLLER_INPUT: &str = "crates/runenui_winit/src/controller_input.rs";

#[test]
fn scroll_has_one_mounted_owner_and_surface_geometry_is_only_a_projection() -> Result<(), String> {
    let root = workspace_root()?;
    let mounted = read(&root.join(MOUNTED_INTERACTION))?;
    let surface = read(&root.join(SURFACE_INTERACTION))?;
    let tree = read(&root.join("crates/runenui_runtime/src/mounted/tree.rs"))?;
    let resolve = read(&root.join("crates/runenui_runtime/src/surface/resolve.rs"))?;

    require(
        &mounted,
        "pub(crate) scroll_offset: (f32, f32)",
        MOUNTED_INTERACTION,
    )?;
    require(
        &surface,
        "This is a cache-compatibility snapshot only",
        SURFACE_INTERACTION,
    )?;
    require(&tree, "commit_scroll_offset(", "mounted/tree.rs")?;
    require(
        &resolve,
        "normalize_scroll_projection(",
        "surface/resolve.rs",
    )?;

    let production_roots = [
        "crates/runenui_core/src",
        "crates/runenui_runtime/src",
        "crates/runenui_text/src",
        "crates/runenui_render_wgpu/src",
        "crates/runenui_winit/src",
    ];
    let mut owners = Vec::new();
    for relative in production_roots {
        collect_rust_files_containing(
            &root.join(relative),
            "pub(crate) scroll_offset: (f32, f32)",
            &mut owners,
        )?;
    }
    let expected = root.join(MOUNTED_INTERACTION);
    if owners != [expected] {
        return Err(format!(
            "M10E requires one mounted scroll-offset owner; found {owners:?}"
        ));
    }
    Ok(())
}

#[test]
fn core_thresholds_are_values_and_controller_normalization_stays_host_owned() -> Result<(), String>
{
    let root = workspace_root()?;
    let core = read(&root.join(CORE_POINTER))?;
    let controller = read(&root.join(CONTROLLER_INPUT))?;
    for required in [
        "pub struct TouchGestureThresholds",
        "pub const fn new(",
        "pub const fn scroll_movement(",
        "pub const fn selection_movement(",
    ] {
        require(&core, required, CORE_POINTER)?;
    }
    for forbidden in ["TouchGestureState", "Gamepad", "gilrs", "winit::"] {
        forbid(&core, forbidden, CORE_POINTER)?;
    }
    for required in [
        "pub enum ControllerInputProfile",
        "pub enum ControllerTransition",
        "CommandOrigin::controller()",
        "SemanticCommand::Activate",
        "SemanticCommand::FocusDown",
        "UnsupportedProfileControl",
    ] {
        require(&controller, required, CONTROLLER_INPUT)?;
    }
    for forbidden in ["gilrs::", "winit::", "GamepadId", "AxisId", "fn poll("] {
        forbid(&controller, forbidden, CONTROLLER_INPUT)?;
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
            "M10E authority lost required contract `{required}` in {owner}"
        ))
    }
}

fn forbid(source: &str, forbidden: &str, owner: &str) -> Result<(), String> {
    if source.contains(forbidden) {
        Err(format!(
            "M10E authority {owner} acquired forbidden competing/native seam `{forbidden}`"
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
