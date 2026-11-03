use crate::ports::media_inspection::GuardedMediaInspector;
use crate::ports::source_publication::{PublicationFileSource, PublicationNamespaceFactory};

pub(in crate::application) fn publication_namespaces()
-> impl PublicationNamespaceFactory<Namespace: PublicationFileSource> {
    crate::adapters::LocalPublicationNamespaces
}

pub(super) fn media_inspector() -> impl GuardedMediaInspector {
    crate::adapters::LocalMediaInspector::new()
}
