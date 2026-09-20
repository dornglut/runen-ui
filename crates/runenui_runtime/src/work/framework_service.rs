//! Live framework-service requests and their opaque runtime-local tokens.

use std::sync::Arc;

use runenui_core::__runtime::{FrameworkServiceBinding, FrameworkServiceEffect};
use runenui_core::{FrameworkServiceRequest, FrameworkServiceResponseKind};

use super::WorkGeneration;

/// Opaque runtime-local token for one exact live framework-service request.
#[derive(Clone)]
pub struct FrameworkServiceToken {
    pub(crate) namespace: Arc<()>,
    pub(crate) generation: WorkGeneration,
}

impl core::fmt::Debug for FrameworkServiceToken {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("FrameworkServiceToken(..)")
    }
}

/// Read-only host integration view of a committed framework-service request.
pub struct FrameworkServiceRef<'a> {
    pub(crate) token: FrameworkServiceToken,
    pub(crate) request: &'a FrameworkServiceRequest,
    pub(crate) binding: &'a FrameworkServiceBinding,
}

impl FrameworkServiceRef<'_> {
    #[must_use]
    pub fn token(&self) -> FrameworkServiceToken {
        self.token.clone()
    }

    #[must_use]
    pub const fn request(&self) -> &FrameworkServiceRequest {
        self.request
    }

    #[must_use]
    pub const fn binding(&self) -> &FrameworkServiceBinding {
        self.binding
    }
}

pub(crate) struct LiveFrameworkService {
    pub(crate) generation: WorkGeneration,
    pub(crate) request: FrameworkServiceRequest,
    pub(crate) binding: FrameworkServiceBinding,
    pub(crate) expected: FrameworkServiceResponseKind,
}

impl LiveFrameworkService {
    pub(crate) fn new(generation: WorkGeneration, effect: FrameworkServiceEffect) -> Self {
        let expected = effect.request.response_kind();
        Self {
            generation,
            request: effect.request,
            binding: effect.binding,
            expected,
        }
    }
}
