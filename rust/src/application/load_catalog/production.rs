use crate::ports::source_state::SourceStateProbe;

pub(super) fn source_state() -> impl SourceStateProbe {
    crate::adapters::LocalSourceState
}
