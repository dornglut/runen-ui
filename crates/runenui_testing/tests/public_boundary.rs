const MANIFEST: &str = include_str!("../Cargo.toml");
const LIB: &str = include_str!("../src/lib.rs");
const HARNESS: &str = include_str!("../src/harness.rs");
const SEMANTIC: &str = include_str!("../src/semantic.rs");
const SETTLE: &str = include_str!("../src/settle.rs");
const SURFACE: &str = include_str!("../src/surface.rs");

#[test]
fn testing_crate_source_uses_no_private_runtime_bridge_or_wall_clock() {
    let sources = [
        ("lib.rs", LIB),
        ("harness.rs", HARNESS),
        ("semantic.rs", SEMANTIC),
        ("settle.rs", SETTLE),
        ("surface.rs", SURFACE),
    ];
    let forbidden = [
        "__runtime_",
        "std::thread::sleep",
        "thread::sleep",
        "Instant::now(",
        "SystemTime::now(",
    ];

    for (path, source) in sources {
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "runenui_testing source {path} must not contain private/wall-clock authority `{needle}`"
            );
        }
    }
}

#[test]
fn testing_crate_manifest_enables_no_internal_test_seam() {
    assert!(!MANIFEST.contains("internal-test-seams"));
}

#[test]
fn semantic_testing_layer_has_no_mounted_identity_dependency() {
    assert!(!SEMANTIC.contains("MountedNodeId"));
}
