use serde::{Deserialize, Serialize};

use crate::identity::UiStoryEvidenceProducerId;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStoryEvidenceProducer {
    pub producer_id: UiStoryEvidenceProducerId,
    pub label: String,
    pub owned_by_app: bool,
}

impl UiStoryEvidenceProducer {
    pub fn new(
        producer_id: UiStoryEvidenceProducerId,
        label: impl Into<String>,
        owned_by_app: bool,
    ) -> Self {
        Self {
            producer_id,
            label: label.into(),
            owned_by_app,
        }
    }

    pub fn domain_owned(producer_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(UiStoryEvidenceProducerId::new(producer_id), label, false)
    }

    pub fn app_owned(producer_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(UiStoryEvidenceProducerId::new(producer_id), label, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn producer_preserves_app_ownership() {
        let producer = UiStoryEvidenceProducer::app_owned(
            "runenwerk_editor.ui_gallery.source_loader",
            "Editor gallery source loader",
        );

        assert_eq!(
            producer.producer_id.as_str(),
            "runenwerk_editor.ui_gallery.source_loader"
        );
        assert_eq!(producer.label, "Editor gallery source loader");
        assert!(producer.owned_by_app);
    }
}
