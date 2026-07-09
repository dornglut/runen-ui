//! File: domain/ui/ui_surface/src/diagnostics/codes.rs
//! Purpose: Stable ui_surface diagnostic code declarations.

use diagnostics::{DiagnosticCode, DiagnosticDomain};

pub const UI_SURFACE_DIAGNOSTIC_DOMAIN: DiagnosticDomain =
    DiagnosticDomain::from_static_unchecked("ui_surface");

pub const MISSING_CAPABILITY_CODE: DiagnosticCode =
    DiagnosticCode::from_static_unchecked("ui_surface.ratification.missing_capability");

pub const DUPLICATE_MOUNTED_SURFACE_INSTANCE_CODE: DiagnosticCode =
    DiagnosticCode::from_static_unchecked("ui_surface.mount.duplicate_surface_instance");

pub const UNKNOWN_SURFACE_DEFINITION_CODE: DiagnosticCode =
    DiagnosticCode::from_static_unchecked("ui_surface.mount.unknown_definition");
