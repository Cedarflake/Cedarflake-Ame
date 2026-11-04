use std::sync::Arc;

use crate::domain::ScanError;

use super::PersistentIdentityJournalSource;
use super::journal_boundary::JournalBoundarySource;
use super::journal_volume::PersistentJournalVolumeSource;

pub(crate) trait JournalRuntimeSession: Clone + Send + Sync + 'static {
    type BoundarySource<'a>: JournalBoundarySource
    where
        Self: 'a;
    type VolumeSource: PersistentJournalVolumeSource;

    fn identity_source(&self) -> Option<Arc<dyn PersistentIdentityJournalSource>>;
    fn boundary_source(&self) -> Self::BoundarySource<'_>;
    fn volume_source(self) -> Self::VolumeSource;
}

pub(crate) trait JournalRuntimeConnection: Clone + Send + Sync + 'static {
    type Session: JournalRuntimeSession;

    fn unavailable() -> Self;
    fn protocol_version(&self) -> u16;
    fn is_connected(&self) -> bool;
    fn session(&self) -> Option<Self::Session>;
    fn close(&self) -> Result<(), ScanError>;
}

pub(crate) trait JournalRuntimeFactory: Send + Sync {
    type Connection: JournalRuntimeConnection;

    fn connect(&self) -> Self::Connection;
}
