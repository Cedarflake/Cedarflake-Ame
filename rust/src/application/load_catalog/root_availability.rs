use crate::domain::LibraryRootView;
use crate::ports::source_state::SourceStateProbe;

pub(super) fn refresh(probe: &impl SourceStateProbe, roots: &mut [LibraryRootView]) {
    for root in roots {
        let evidence = probe.inspect_root(&root.path);
        root.availability = evidence.availability;
        root.availability_message = evidence.message;
    }
}

#[cfg(test)]
mod tests;
