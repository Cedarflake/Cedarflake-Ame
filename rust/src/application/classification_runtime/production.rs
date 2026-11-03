use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::ports::classification_vocabulary::ManagedClassificationVocabulary;

pub(super) fn vocabulary(
    cancelled: &AtomicBool,
) -> Result<impl ManagedClassificationVocabulary + use<>, ScanError> {
    crate::adapters::CuratedClassificationVocabulary::bundled(cancelled)
}
