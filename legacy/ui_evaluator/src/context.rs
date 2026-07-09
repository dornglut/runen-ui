//! Host-provided evaluation inputs.

use serde::{Deserialize, Serialize};
use ui_binding::{BindingAuthorization, HostDataSnapshot};
use ui_program::UiEventPacket;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UiEvaluationContext {
    #[serde(default)]
    pub host_events: Vec<UiEventPacket>,
    #[serde(default)]
    pub host_data: Vec<HostDataSnapshot>,
    #[serde(default)]
    pub binding_authorizations: Vec<BindingAuthorization>,
}

impl UiEvaluationContext {
    pub fn with_host_event(mut self, event: UiEventPacket) -> Self {
        self.host_events.push(event);
        self
    }

    pub fn with_host_data(mut self, host_data: HostDataSnapshot) -> Self {
        self.host_data.push(host_data);
        self
    }

    pub fn with_binding_authorization(mut self, authorization: BindingAuthorization) -> Self {
        self.binding_authorizations.push(authorization);
        self
    }
}
