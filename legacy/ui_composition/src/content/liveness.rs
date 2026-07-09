use serde::{Deserialize, Serialize};

use crate::MountedUnitId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContentLiveness {
    Resolved,
    Missing,
    Loading,
    Suspended,
    Denied,
    UnsupportedProfile,
    Crashed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContentLivenessObservation {
    pub mounted_unit: MountedUnitId,
    pub state: ContentLiveness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentProjectionFallback {
    ResolvedContent,
    AppProvidedUnavailable,
    NeutralDiagnosticPlaceholder,
    Hidden,
}

pub const fn select_content_projection_fallback(
    liveness: ContentLiveness,
    app_projection_available: bool,
    neutral_placeholder_available: bool,
    unavailable_policy: crate::UnavailableContentPolicy,
    host_accepts_hide: bool,
) -> Option<ContentProjectionFallback> {
    if matches!(liveness, ContentLiveness::Resolved) {
        return Some(ContentProjectionFallback::ResolvedContent);
    }
    if app_projection_available {
        return Some(ContentProjectionFallback::AppProvidedUnavailable);
    }
    if neutral_placeholder_available {
        return Some(ContentProjectionFallback::NeutralDiagnosticPlaceholder);
    }
    if unavailable_policy.permits_hide() && host_accepts_hide {
        return Some(ContentProjectionFallback::Hidden);
    }
    None
}
