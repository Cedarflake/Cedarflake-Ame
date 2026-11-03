use crate::ports::media_inspection::GuardedMediaInspector;
use crate::ports::source_publication::{PublicationDirectorySource, PublicationNamespaceFactory};

pub(in crate::application) fn publication_namespaces()
-> impl PublicationNamespaceFactory<Namespace: PublicationDirectorySource> {
    crate::adapters::LocalPublicationNamespaces
}

pub(in crate::application) fn media_inspector() -> impl GuardedMediaInspector {
    crate::adapters::LocalMediaInspector::new()
}
