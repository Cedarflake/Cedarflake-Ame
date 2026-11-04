use crate::ports::exact_duplicates::ExactContentReader;
use crate::ports::source_state::SourceStateProbe;

pub(super) fn content_reader() -> impl ExactContentReader {
    crate::application::background_analysis::AdmittedExactContentReader::new(
        crate::adapters::GuardedExactContentReader,
        crate::application::background_analysis::shared(),
    )
}

pub(super) fn source_state() -> impl SourceStateProbe {
    crate::adapters::LocalSourceState
}
