//! Domain-owned UI story proof contracts.

pub mod cli_v2;
pub mod diagnostic;
pub mod evidence;
pub mod fixtures_v2;
pub mod identity;
pub mod manifest_v2;
pub mod mount_v2;
pub mod registry_v2;
pub mod report_v2;
pub mod run_v2;
pub mod workflow;

pub use cli_v2::*;
pub use diagnostic::*;
pub use evidence::*;
pub use fixtures_v2::*;
pub use identity::*;
pub use manifest_v2::*;
pub use mount_v2::*;
pub use registry_v2::*;
pub use report_v2::*;
pub use run_v2::*;
pub use workflow::*;

#[cfg(test)]
mod tests {
    #[test]
    fn root_exports_do_not_reintroduce_flat_stage_api() {
        let source = include_str!("lib.rs");
        let forbidden_symbols = [
            concat!("UiStory", "StageKind"),
            concat!("UiStory", "StageReport"),
            concat!("UiStory", "RunReport"),
            concat!("UiStory", "MountEligibility"),
            concat!("required", "_stages"),
            concat!("run_story", "_with_stage_reports"),
        ];
        let forbidden_modules = [
            concat!("pub mod ", "gallery"),
            concat!("pub mod ", "runner"),
            concat!("pub mod ", "report;"),
            concat!("pub mod ", "mount;"),
            concat!("pub mod ", "proof"),
            concat!("pub mod ", "registry;"),
            concat!("pub mod ", "manifest;"),
            concat!("pub mod ", "cli;"),
        ];

        for forbidden in forbidden_symbols.into_iter().chain(forbidden_modules) {
            assert!(
                !source.contains(forbidden),
                "ui_story root must not re-export old flat-stage authority `{forbidden}`"
            );
        }

        for required in [
            "pub mod cli_v2;",
            "pub mod manifest_v2;",
            "pub mod registry_v2;",
            "pub mod run_v2;",
            "pub mod report_v2;",
            "pub mod mount_v2;",
            "pub mod fixtures_v2;",
            "pub mod diagnostic;",
            "pub mod evidence;",
            "pub mod identity;",
            "pub mod workflow;",
        ] {
            assert!(
                source.contains(required),
                "ui_story root must keep current V2 module export `{required}`"
            );
        }
    }
}
