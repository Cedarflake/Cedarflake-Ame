use super::{
    LibraryChangeIntent, LibraryRecoveryAuthorityReason, LibraryRecoveryOpeningBoundary,
    LibraryRootGeneration, ScanError,
};

pub const PERSISTENT_JOURNAL_CONTRACT_VERSION: u16 = 1;
pub const MAX_PERSISTENT_JOURNAL_BATCH_INTENTS: usize = 1_024;
pub const MAX_PERSISTENT_JOURNAL_HANDOFFS: usize = 1_024;
pub const MAX_PERSISTENT_JOURNAL_PENDING_RENAMES: usize = 1_024;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct JournalIdentifier(u64);

impl JournalIdentifier {
    pub fn new(value: u64) -> Result<Self, ScanError> {
        if value == 0 {
            return Err(invalid_value("journal identifier"));
        }
        Ok(Self(value))
    }

    pub fn parse_canonical(value: &str) -> Result<Self, ScanError> {
        let parsed = value
            .parse::<u64>()
            .map_err(|_| invalid_value("journal identifier"))?;
        let identifier = Self::new(parsed)?;
        if identifier.to_canonical_text() != value {
            return Err(invalid_value("journal identifier"));
        }
        Ok(identifier)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub fn to_canonical_text(self) -> String {
        self.0.to_string()
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct JournalUsn(i64);

impl JournalUsn {
    pub fn new(value: i64) -> Result<Self, ScanError> {
        if value < 0 {
            return Err(invalid_value("journal USN"));
        }
        Ok(Self(value))
    }

    pub fn parse_canonical(value: &str) -> Result<Self, ScanError> {
        let parsed = value
            .parse::<i64>()
            .map_err(|_| invalid_value("journal USN"))?;
        let usn = Self::new(parsed)?;
        if usn.to_canonical_text() != value {
            return Err(invalid_value("journal USN"));
        }
        Ok(usn)
    }

    pub const fn value(self) -> i64 {
        self.0
    }

    pub fn to_canonical_text(self) -> String {
        self.0.to_string()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum JournalFileReference {
    V2([u8; 8]),
    V3([u8; 16]),
}

impl JournalFileReference {
    pub fn from_bytes(value: &[u8]) -> Result<Self, ScanError> {
        match value.len() {
            8 => {
                let mut reference = [0_u8; 8];
                reference.copy_from_slice(value);
                Ok(Self::V2(reference))
            }
            16 => {
                let mut reference = [0_u8; 16];
                reference.copy_from_slice(value);
                Ok(Self::V3(reference))
            }
            _ => Err(invalid_value("journal file reference")),
        }
    }

    pub fn parse_canonical(value: &str) -> Result<Self, ScanError> {
        let (version, encoded) = value
            .split_once(':')
            .ok_or_else(|| invalid_value("journal file reference"))?;
        if encoded
            .bytes()
            .any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
        {
            return Err(invalid_value("journal file reference"));
        }
        let bytes = decode_hex(encoded)?;
        match version {
            "v2" if bytes.len() == 8 => Self::from_bytes(&bytes),
            "v3" if bytes.len() == 16 => Self::from_bytes(&bytes),
            _ => Err(invalid_value("journal file reference")),
        }
    }

    pub fn to_canonical_text(&self) -> String {
        let (version, bytes) = match self {
            Self::V2(bytes) => ("v2", bytes.as_slice()),
            Self::V3(bytes) => ("v3", bytes.as_slice()),
        };
        let mut encoded = String::with_capacity(version.len() + 1 + bytes.len() * 2);
        encoded.push_str(version);
        encoded.push(':');
        for byte in bytes {
            use std::fmt::Write;
            let _ = write!(&mut encoded, "{byte:02x}");
        }
        encoded
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::V2(bytes) => bytes,
            Self::V3(bytes) => bytes,
        }
    }

    pub const fn record_version(&self) -> u8 {
        match self {
            Self::V2(_) => 2,
            Self::V3(_) => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalVolumeIdentity {
    pub volume_guid: String,
    pub volume_serial: u64,
}

impl PersistentJournalVolumeIdentity {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.volume_guid, 512, "volume GUID")
    }

    pub fn canonical_serial(&self) -> String {
        self.volume_serial.to_string()
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PersistentJournalCapabilityState {
    Unknown,
    Supported,
    LiveOnly,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PersistentJournalContinuityState {
    BaselineRequired,
    CatchingUp,
    Current,
    RecoveryRequired,
    LiveOnly,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalFailure {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PersistentJournalRootFailureKind {
    JournalReset,
    JournalTrim,
    JournalReconstructionFailure,
    ContainmentFailure,
    BrokerAfterCurrentFailure,
    Transient,
    NonRecoverable,
    Cancelled,
}

impl PersistentJournalRootFailureKind {
    pub const fn recovery_reason(self) -> Option<LibraryRecoveryAuthorityReason> {
        match self {
            Self::JournalReset => Some(LibraryRecoveryAuthorityReason::JournalReset),
            Self::JournalTrim => Some(LibraryRecoveryAuthorityReason::JournalTrim),
            Self::JournalReconstructionFailure => {
                Some(LibraryRecoveryAuthorityReason::JournalReconstructionFailure)
            }
            Self::ContainmentFailure => Some(LibraryRecoveryAuthorityReason::ContainmentFailure),
            Self::BrokerAfterCurrentFailure => {
                Some(LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure)
            }
            Self::Transient | Self::NonRecoverable | Self::Cancelled => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalReadFailure {
    pub kind: PersistentJournalRootFailureKind,
    pub failure: PersistentJournalFailure,
    pub opening_boundary: Option<Box<LibraryRecoveryOpeningBoundary>>,
}

impl PersistentJournalReadFailure {
    pub fn validate(&self) -> Result<(), ScanError> {
        self.failure.validate()?;
        if let Some(boundary) = &self.opening_boundary {
            boundary.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalRootFailure {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub kind: PersistentJournalRootFailureKind,
    pub failure: PersistentJournalFailure,
    pub opening_boundary: Option<LibraryRecoveryOpeningBoundary>,
}

impl PersistentJournalRootFailure {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.root_id, 256, "journal failure root ID")?;
        PersistentJournalReadFailure {
            kind: self.kind,
            failure: self.failure.clone(),
            opening_boundary: self.opening_boundary.clone().map(Box::new),
        }
        .validate()
    }
}

impl PersistentJournalFailure {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.code, 128, "journal failure code")?;
        validate_identifier(&self.message, 4_096, "journal failure message")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalCapability {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub protocol_version: u16,
    pub contract_version: u16,
    pub state: PersistentJournalCapabilityState,
    pub continuity: PersistentJournalContinuityState,
    pub failure: Option<PersistentJournalFailure>,
    pub updated_unix_ms: i64,
}

impl PersistentJournalCapability {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.root_id, 256, "root ID")?;
        if self.contract_version != PERSISTENT_JOURNAL_CONTRACT_VERSION
            || self.updated_unix_ms < 0
            || (self.state == PersistentJournalCapabilityState::Unknown
                && self.protocol_version != 0)
            || (self.state != PersistentJournalCapabilityState::Unknown
                && self.protocol_version == 0)
            || (self.state == PersistentJournalCapabilityState::Supported && self.failure.is_some())
            || (self.continuity == PersistentJournalContinuityState::Current
                && self.state != PersistentJournalCapabilityState::Supported)
        {
            return Err(invalid_value("persistent journal capability"));
        }
        if let Some(failure) = &self.failure {
            failure.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalCheckpoint {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub volume: PersistentJournalVolumeIdentity,
    pub root_file_reference: JournalFileReference,
    pub journal_id: JournalIdentifier,
    pub next_unread_usn: JournalUsn,
    pub captured_exclusive_end: JournalUsn,
    pub covered_catalog_revision: u64,
    pub protocol_version: u16,
    pub contract_version: u16,
    pub continuity: PersistentJournalContinuityState,
    pub failure: Option<PersistentJournalFailure>,
    pub updated_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PersistentJournalBaselinePhase {
    Inventory,
    Replay,
    Absence,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalBaseline {
    pub change_id: super::LibraryChangeId,
    pub run_id: String,
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub volume: PersistentJournalVolumeIdentity,
    pub root_file_reference: JournalFileReference,
    pub journal_id: JournalIdentifier,
    pub opening_next_usn: JournalUsn,
    pub closing_next_usn: Option<JournalUsn>,
    pub protocol_version: u16,
    pub contract_version: u16,
    pub phase: PersistentJournalBaselinePhase,
    pub authorized_unix_ms: i64,
    pub updated_unix_ms: i64,
    pub completed_unix_ms: Option<i64>,
}

impl PersistentJournalBaseline {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.run_id, 512, "baseline run ID")?;
        validate_identifier(&self.root_id, 256, "root ID")?;
        self.volume.validate()?;
        if self.protocol_version == 0
            || self.contract_version != PERSISTENT_JOURNAL_CONTRACT_VERSION
            || self.authorized_unix_ms < 0
            || self.updated_unix_ms < self.authorized_unix_ms
            || self
                .closing_next_usn
                .is_some_and(|closing| closing < self.opening_next_usn)
            || (self.phase == PersistentJournalBaselinePhase::Inventory
                && self.closing_next_usn.is_some())
            || (self.phase != PersistentJournalBaselinePhase::Inventory
                && self.closing_next_usn.is_none())
            || (self.phase == PersistentJournalBaselinePhase::Completed)
                != self.completed_unix_ms.is_some()
            || self
                .completed_unix_ms
                .is_some_and(|completed| completed < self.updated_unix_ms)
        {
            return Err(invalid_value("persistent journal baseline"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalBaselineStartRequest {
    pub run_id: String,
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub authority_reason: LibraryRecoveryAuthorityReason,
    pub volume: PersistentJournalVolumeIdentity,
    pub root_file_reference: JournalFileReference,
    pub journal_id: JournalIdentifier,
    pub opening_next_usn: JournalUsn,
    pub protocol_version: u16,
    pub contract_version: u16,
    pub authorized_unix_ms: i64,
}

impl PersistentJournalBaselineStartRequest {
    pub fn validate(&self) -> Result<(), ScanError> {
        if !matches!(
            self.authority_reason,
            LibraryRecoveryAuthorityReason::ExistingRootBaseline
                | LibraryRecoveryAuthorityReason::FirstImportBoundary
        ) {
            return Err(invalid_value("persistent journal baseline authority"));
        }
        let baseline = PersistentJournalBaseline {
            change_id: super::LibraryChangeId::new(1).expect("one is a valid change ID"),
            run_id: self.run_id.clone(),
            root_id: self.root_id.clone(),
            root_generation: self.root_generation,
            volume: self.volume.clone(),
            root_file_reference: self.root_file_reference.clone(),
            journal_id: self.journal_id,
            opening_next_usn: self.opening_next_usn,
            closing_next_usn: None,
            protocol_version: self.protocol_version,
            contract_version: self.contract_version,
            phase: PersistentJournalBaselinePhase::Inventory,
            authorized_unix_ms: self.authorized_unix_ms,
            updated_unix_ms: self.authorized_unix_ms,
            completed_unix_ms: None,
        };
        baseline.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalBaselineClosingBoundary {
    pub change_id: super::LibraryChangeId,
    pub volume: PersistentJournalVolumeIdentity,
    pub root_file_reference: JournalFileReference,
    pub journal_id: JournalIdentifier,
    pub closing_next_usn: JournalUsn,
    pub protocol_version: u16,
    pub captured_unix_ms: i64,
}

impl PersistentJournalBaselineClosingBoundary {
    pub fn validate(&self) -> Result<(), ScanError> {
        self.volume.validate()?;
        if self.protocol_version == 0 || self.captured_unix_ms < 0 {
            return Err(invalid_value(
                "persistent journal baseline closing boundary",
            ));
        }
        Ok(())
    }
}

impl PersistentJournalCheckpoint {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.root_id, 256, "root ID")?;
        self.volume.validate()?;
        if self.next_unread_usn > self.captured_exclusive_end
            || self.protocol_version == 0
            || self.contract_version != PERSISTENT_JOURNAL_CONTRACT_VERSION
            || self.updated_unix_ms < 0
            || (self.continuity == PersistentJournalContinuityState::Current
                && (self.next_unread_usn != self.captured_exclusive_end || self.failure.is_some()))
        {
            return Err(invalid_value("persistent journal checkpoint"));
        }
        if let Some(failure) = &self.failure {
            failure.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalSourceRange {
    pub batch_id: String,
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub volume: PersistentJournalVolumeIdentity,
    pub journal_id: JournalIdentifier,
    pub requested_start_usn: JournalUsn,
    pub requested_end_usn: JournalUsn,
    pub covered_until_usn: JournalUsn,
    pub is_complete: bool,
    pub protocol_version: u16,
    pub contract_version: u16,
    pub state: PersistentJournalRangeState,
    pub enrolled_unix_ms: i64,
    pub checkpointed_unix_ms: Option<i64>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PersistentJournalRangeState {
    Enrolled,
    Checkpointed,
    Superseded,
}

impl PersistentJournalSourceRange {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_canonical_digest(&self.batch_id, "journal batch ID")?;
        validate_identifier(&self.root_id, 256, "root ID")?;
        self.volume.validate()?;
        if self.requested_start_usn >= self.requested_end_usn
            || self.covered_until_usn <= self.requested_start_usn
            || self.covered_until_usn > self.requested_end_usn
            || self.is_complete != (self.covered_until_usn == self.requested_end_usn)
            || self.protocol_version == 0
            || self.contract_version != PERSISTENT_JOURNAL_CONTRACT_VERSION
            || self.enrolled_unix_ms < 0
            || self.checkpointed_unix_ms.is_some_and(|value| value < 0)
            || (self.state == PersistentJournalRangeState::Checkpointed)
                != self.checkpointed_unix_ms.is_some()
        {
            return Err(invalid_value("persistent journal source range"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalPendingRename {
    pub carry_id: String,
    pub source_range_id: String,
    pub volume: PersistentJournalVolumeIdentity,
    pub journal_id: JournalIdentifier,
    pub file_reference: JournalFileReference,
    pub old_usn: JournalUsn,
    pub previous_root_id: String,
    pub previous_root_generation: LibraryRootGeneration,
    pub previous_relative_path: String,
    pub is_directory: bool,
    pub enrolled_unix_ms: i64,
}

impl PersistentJournalPendingRename {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.carry_id, 512, "pending rename carry ID")?;
        validate_identifier(&self.source_range_id, 512, "pending rename source range ID")?;
        self.volume.validate()?;
        validate_identifier(&self.previous_root_id, 256, "pending rename root ID")?;
        validate_relative_path(&self.previous_relative_path)?;
        if self.enrolled_unix_ms < 0 || self.carry_id != persistent_journal_pending_rename_id(self)
        {
            return Err(invalid_value("persistent journal pending rename"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalCrossRootLineage {
    pub lineage_id: String,
    pub owner_source_range_id: String,
    pub volume: PersistentJournalVolumeIdentity,
    pub journal_id: JournalIdentifier,
    pub file_reference: JournalFileReference,
    pub old_usn: Option<JournalUsn>,
    pub new_usn: Option<JournalUsn>,
    pub previous_carry_id: Option<String>,
    pub previous_root_id: String,
    pub previous_root_generation: LibraryRootGeneration,
    pub previous_relative_path: String,
    pub current_root_id: String,
    pub current_root_generation: LibraryRootGeneration,
    pub current_relative_path: String,
    pub state: PersistentJournalLineageState,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PersistentJournalLineageState {
    Pending,
    Completed,
    Superseded,
}

impl PersistentJournalCrossRootLineage {
    pub fn validate(&self) -> Result<(), ScanError> {
        validate_identifier(&self.lineage_id, 512, "journal lineage ID")?;
        validate_identifier(
            &self.owner_source_range_id,
            512,
            "journal lineage source range ID",
        )?;
        self.volume.validate()?;
        if let Some(carry_id) = &self.previous_carry_id {
            validate_canonical_digest(carry_id, "previous pending rename carry ID")?;
        }
        validate_identifier(&self.previous_root_id, 256, "previous root ID")?;
        validate_identifier(&self.current_root_id, 256, "current root ID")?;
        validate_relative_path(&self.previous_relative_path)?;
        validate_relative_path(&self.current_relative_path)?;
        let carried_coordinates_are_valid =
            match (self.previous_carry_id.as_ref(), self.old_usn, self.new_usn) {
                (None, None, None) => true,
                (Some(_), Some(old_usn), Some(new_usn)) => old_usn < new_usn,
                _ => false,
            };
        if self.previous_root_id == self.current_root_id || !carried_coordinates_are_valid {
            return Err(invalid_value("cross-root journal lineage"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalEnrollmentBatch {
    pub range: PersistentJournalSourceRange,
    pub intents: Vec<LibraryChangeIntent>,
    pub cross_root_lineage: Vec<PersistentJournalCrossRootLineage>,
    pub carried_cross_root_lineage: Vec<PersistentJournalCrossRootLineage>,
    pub pending_renames: Vec<PersistentJournalPendingRename>,
    pub consumed_pending_rename_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentJournalVolumePage {
    pub enrollment: PersistentJournalEnrollmentBatch,
    pub checkpoint: PersistentJournalCheckpoint,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PersistentJournalVolumeBatch {
    pub pages: Vec<PersistentJournalVolumePage>,
}

#[derive(Clone, Debug)]
pub struct PersistentJournalRootReadOutcome {
    pub root_id: String,
    pub root_generation: LibraryRootGeneration,
    pub opening_boundary: Option<LibraryRecoveryOpeningBoundary>,
    pub page: Result<
        Option<(
            PersistentJournalEnrollmentBatch,
            PersistentJournalCheckpoint,
        )>,
        PersistentJournalReadFailure,
    >,
}

impl PersistentJournalEnrollmentBatch {
    pub fn validate(&self) -> Result<(), ScanError> {
        self.range.validate()?;
        if self.intents.len() > MAX_PERSISTENT_JOURNAL_BATCH_INTENTS
            || self
                .cross_root_lineage
                .len()
                .saturating_add(self.carried_cross_root_lineage.len())
                > MAX_PERSISTENT_JOURNAL_HANDOFFS
            || self.pending_renames.len() > MAX_PERSISTENT_JOURNAL_PENDING_RENAMES
            || self.consumed_pending_rename_ids.len() > MAX_PERSISTENT_JOURNAL_PENDING_RENAMES
            || self.intents.iter().any(|intent| {
                intent.root_id != self.range.root_id
                    || intent.root_generation != self.range.root_generation
            })
        {
            return Err(invalid_value("persistent journal enrollment batch"));
        }
        for lineage in &self.cross_root_lineage {
            lineage.validate()?;
            if lineage.volume != self.range.volume
                || lineage.journal_id != self.range.journal_id
                || lineage.owner_source_range_id != self.range.batch_id
                || !((lineage.previous_root_id == self.range.root_id
                    && lineage.previous_root_generation == self.range.root_generation)
                    || (lineage.current_root_id == self.range.root_id
                        && lineage.current_root_generation == self.range.root_generation))
            {
                return Err(invalid_value("persistent journal enrollment lineage"));
            }
        }
        for lineage in &self.carried_cross_root_lineage {
            lineage.validate()?;
            if lineage.volume != self.range.volume
                || lineage.journal_id != self.range.journal_id
                || lineage.owner_source_range_id == self.range.batch_id
                || !((lineage.previous_root_id == self.range.root_id
                    && lineage.previous_root_generation == self.range.root_generation)
                    || (lineage.current_root_id == self.range.root_id
                        && lineage.current_root_generation == self.range.root_generation))
                || lineage.previous_carry_id.is_none()
            {
                return Err(invalid_value("persistent journal carried lineage"));
            }
        }
        let mut pending_ids = std::collections::BTreeSet::new();
        for pending in &self.pending_renames {
            pending.validate()?;
            if pending.source_range_id != self.range.batch_id
                || pending.volume != self.range.volume
                || pending.journal_id != self.range.journal_id
                || pending.previous_root_id != self.range.root_id
                || pending.previous_root_generation != self.range.root_generation
                || pending.old_usn < self.range.requested_start_usn
                || pending.old_usn >= self.range.covered_until_usn
                || !pending_ids.insert(pending.carry_id.as_str())
            {
                return Err(invalid_value("persistent journal pending rename owner"));
            }
        }
        let mut consumed = std::collections::BTreeSet::new();
        for carry_id in &self.consumed_pending_rename_ids {
            validate_identifier(carry_id, 512, "consumed pending rename carry ID")?;
            if pending_ids.contains(carry_id.as_str()) || !consumed.insert(carry_id.as_str()) {
                return Err(invalid_value("consumed persistent journal pending rename"));
            }
        }
        Ok(())
    }
}

impl PersistentJournalVolumeBatch {
    pub fn validate(&self) -> Result<(), ScanError> {
        if self.pages.is_empty() || self.pages.len() > 8 {
            return Err(invalid_value("persistent journal volume batch"));
        }
        let first = &self.pages[0].enrollment.range;
        let mut roots = std::collections::BTreeSet::new();
        for page in &self.pages {
            page.enrollment.validate()?;
            page.checkpoint.validate()?;
            let range = &page.enrollment.range;
            if range.volume != first.volume
                || range.journal_id != first.journal_id
                || range.requested_end_usn != first.requested_end_usn
                || page.checkpoint.root_id != range.root_id
                || page.checkpoint.root_generation != range.root_generation
                || page.checkpoint.volume != range.volume
                || page.checkpoint.journal_id != range.journal_id
                || page.checkpoint.next_unread_usn != range.covered_until_usn
                || page.checkpoint.captured_exclusive_end != range.requested_end_usn
                || !roots.insert((range.root_id.as_str(), range.root_generation))
            {
                return Err(invalid_value("persistent journal volume page"));
            }
        }
        Ok(())
    }
}

pub fn persistent_journal_pending_rename_id(pending: &PersistentJournalPendingRename) -> String {
    let mut fields = CanonicalDigest::new("persistent-journal-pending-rename-v1");
    fields.text(&pending.volume.volume_guid);
    fields.u64(pending.volume.volume_serial);
    fields.u64(pending.journal_id.value());
    fields.text(&pending.file_reference.to_canonical_text());
    fields.i64(pending.old_usn.value());
    fields.text(&pending.previous_root_id);
    fields.u64(pending.previous_root_generation.value());
    fields.text(&pending.previous_relative_path);
    fields.bool(pending.is_directory);
    fields.finish()
}

pub fn persistent_journal_batch_id(batch: &PersistentJournalEnrollmentBatch) -> String {
    persistent_journal_batch_id_from_payload(&persistent_journal_batch_payload(batch))
}

pub fn persistent_journal_batch_payload(batch: &PersistentJournalEnrollmentBatch) -> Vec<u8> {
    let mut payload = CanonicalPayload::default();
    let range = &batch.range;
    append_source_range_payload(&mut payload, range);
    let mut intents = batch
        .intents
        .iter()
        .map(canonical_intent)
        .collect::<Vec<_>>();
    intents.sort();
    payload.strings(&intents);

    let mut lineage = batch
        .cross_root_lineage
        .iter()
        .chain(&batch.carried_cross_root_lineage)
        .map(|lineage| canonical_lineage(lineage, &range.batch_id))
        .collect::<Vec<_>>();
    lineage.sort();
    payload.strings(&lineage);

    let mut pending = batch
        .pending_renames
        .iter()
        .map(canonical_pending_rename)
        .collect::<Vec<_>>();
    pending.sort();
    payload.strings(&pending);

    let mut consumed = batch.consumed_pending_rename_ids.clone();
    consumed.sort();
    payload.strings(&consumed);
    payload.finish()
}

pub fn persistent_journal_source_range_payload_prefix(
    range: &PersistentJournalSourceRange,
) -> Vec<u8> {
    let mut payload = CanonicalPayload::default();
    append_source_range_payload(&mut payload, range);
    payload.finish()
}

pub fn persistent_journal_batch_payload_matches_source_range(
    payload: &[u8],
    range: &PersistentJournalSourceRange,
) -> bool {
    payload.starts_with(&persistent_journal_source_range_payload_prefix(range))
}

pub fn persistent_journal_batch_payload_contains_lineage(
    payload: &[u8],
    current_range_id: &str,
    lineage: &PersistentJournalCrossRootLineage,
) -> bool {
    let Some((_, lineage_entries, _, _)) = decode_persistent_journal_batch_payload(payload) else {
        return false;
    };
    let mut candidates = vec![canonical_lineage(lineage, current_range_id)];
    if lineage.state != PersistentJournalLineageState::Pending {
        let mut originally_pending = lineage.clone();
        originally_pending.state = PersistentJournalLineageState::Pending;
        candidates.push(canonical_lineage(&originally_pending, current_range_id));
    }
    candidates
        .iter()
        .any(|candidate| lineage_entries.iter().any(|entry| entry == candidate))
}

pub fn persistent_journal_batch_payload_contains_pending_lineage_source(
    payload: &[u8],
    lineage: &PersistentJournalCrossRootLineage,
) -> bool {
    let (Some(carry_id), Some(old_usn)) = (lineage.previous_carry_id.as_deref(), lineage.old_usn)
    else {
        return false;
    };
    let Some((_, _, pending_entries, _)) = decode_persistent_journal_batch_payload(payload) else {
        return false;
    };
    pending_entries.iter().any(|entry| {
        let fields = entry.split('\0').collect::<Vec<_>>();
        fields.len() == 12
            && fields[0] == carry_id
            && fields[1] == "self"
            && fields[2] == lineage.volume.volume_guid
            && fields[3] == lineage.volume.volume_serial.to_string()
            && fields[4] == lineage.journal_id.value().to_string()
            && fields[5] == lineage.file_reference.to_canonical_text()
            && fields[6] == old_usn.value().to_string()
            && fields[7] == lineage.previous_root_id
            && fields[8] == lineage.previous_root_generation.value().to_string()
            && fields[9] == lineage.previous_relative_path
            && matches!(fields[10], "0" | "1")
            && fields[11].parse::<i64>().is_ok_and(|value| value >= 0)
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PersistentJournalBatchPayloadChildren {
    pub(crate) intents: Vec<LibraryChangeIntent>,
    pub(crate) lineage: Vec<String>,
    pub(crate) pending_renames: Vec<String>,
    pub(crate) consumed_pending_rename_ids: Vec<String>,
}

pub(crate) fn persistent_journal_batch_payload_children(
    payload: &[u8],
) -> Option<PersistentJournalBatchPayloadChildren> {
    let (intents, lineage, pending_renames, consumed_pending_rename_ids) =
        decode_persistent_journal_batch_payload(payload)?;
    if [
        &intents,
        &lineage,
        &pending_renames,
        &consumed_pending_rename_ids,
    ]
    .into_iter()
    .any(|entries| !entries.windows(2).all(|pair| pair[0] < pair[1]))
    {
        return None;
    }
    Some(PersistentJournalBatchPayloadChildren {
        intents: intents
            .into_iter()
            .map(persistent_journal_intent_from_payload_entry)
            .collect::<Option<Vec<_>>>()?,
        lineage: lineage.into_iter().map(str::to_owned).collect(),
        pending_renames: pending_renames.into_iter().map(str::to_owned).collect(),
        consumed_pending_rename_ids: consumed_pending_rename_ids
            .into_iter()
            .map(str::to_owned)
            .collect(),
    })
}

pub(crate) fn persistent_journal_canonical_intent_entry(intent: &LibraryChangeIntent) -> String {
    canonical_intent(intent)
}

fn persistent_journal_intent_from_payload_entry(entry: &str) -> Option<LibraryChangeIntent> {
    let fields = entry.split('\0').collect::<Vec<_>>();
    if fields.len() != 12 {
        return None;
    }
    let intent = LibraryChangeIntent {
        root_id: fields[0].to_owned(),
        root_generation: LibraryRootGeneration::new(parse_canonical_u64(fields[1])?)?,
        kind: match fields[2] {
            "reconcile" => super::LibraryChangeIntentKind::Reconcile,
            "rename" => super::LibraryChangeIntentKind::RenameCandidate,
            "freshness-unknown" => super::LibraryChangeIntentKind::FreshnessUnknown,
            _ => return None,
        },
        scope: match fields[3] {
            "path" => super::LibraryChangeScope::Path,
            "subtree" => super::LibraryChangeScope::Subtree,
            "root" => super::LibraryChangeScope::Root,
            _ => return None,
        },
        relative_path: fields[4].to_owned(),
        previous_relative_path: (!fields[5].is_empty()).then(|| fields[5].to_owned()),
        origin: match fields[6] {
            "live" => super::LibraryChangeOrigin::LiveNotification,
            "metadata-inventory" => super::LibraryChangeOrigin::MetadataInventory,
            "startup" => super::LibraryChangeOrigin::StartupCatchUp,
            "user-refresh" => super::LibraryChangeOrigin::UserRefresh,
            "audit" => super::LibraryChangeOrigin::ConsistencyAudit,
            _ => return None,
        },
        first_observed_unix_ms: parse_canonical_i64(fields[7])?,
        most_recent_observed_unix_ms: parse_canonical_i64(fields[8])?,
        first_sequence: parse_canonical_u64(fields[9])?,
        most_recent_sequence: parse_canonical_u64(fields[10])?,
        coalesced_observation_count: u32::try_from(parse_canonical_u64(fields[11])?).ok()?,
    };
    (canonical_intent(&intent) == entry).then_some(intent)
}

pub(crate) fn persistent_journal_canonical_lineage_entry(
    lineage: &PersistentJournalCrossRootLineage,
    current_range_id: &str,
) -> String {
    canonical_lineage(lineage, current_range_id)
}

pub(crate) fn persistent_journal_pending_rename_from_payload_entry(
    entry: &str,
    source_range_id: &str,
) -> Option<PersistentJournalPendingRename> {
    let fields = entry.split('\0').collect::<Vec<_>>();
    if fields.len() != 12 || fields[1] != "self" {
        return None;
    }
    let volume_serial = parse_canonical_u64(fields[3])?;
    let previous_root_generation = LibraryRootGeneration::new(parse_canonical_u64(fields[8])?)?;
    let is_directory = match fields[10] {
        "0" => false,
        "1" => true,
        _ => return None,
    };
    let enrolled_unix_ms = fields[11].parse::<i64>().ok()?;
    if enrolled_unix_ms < 0 || enrolled_unix_ms.to_string() != fields[11] {
        return None;
    }
    let pending = PersistentJournalPendingRename {
        carry_id: fields[0].to_owned(),
        source_range_id: source_range_id.to_owned(),
        volume: PersistentJournalVolumeIdentity {
            volume_guid: fields[2].to_owned(),
            volume_serial,
        },
        journal_id: JournalIdentifier::parse_canonical(fields[4]).ok()?,
        file_reference: JournalFileReference::parse_canonical(fields[5]).ok()?,
        old_usn: JournalUsn::parse_canonical(fields[6]).ok()?,
        previous_root_id: fields[7].to_owned(),
        previous_root_generation,
        previous_relative_path: fields[9].to_owned(),
        is_directory,
        enrolled_unix_ms,
    };
    pending.validate().ok()?;
    (canonical_pending_rename(&pending) == entry).then_some(pending)
}

fn parse_canonical_u64(value: &str) -> Option<u64> {
    let parsed = value.parse::<u64>().ok()?;
    (parsed.to_string() == value).then_some(parsed)
}

fn parse_canonical_i64(value: &str) -> Option<i64> {
    let parsed = value.parse::<i64>().ok()?;
    (parsed.to_string() == value).then_some(parsed)
}

type CanonicalPayloadLists<'a> = (Vec<&'a str>, Vec<&'a str>, Vec<&'a str>, Vec<&'a str>);

fn decode_persistent_journal_batch_payload(payload: &[u8]) -> Option<CanonicalPayloadLists<'_>> {
    let mut reader = CanonicalPayloadReader::new(payload);
    reader.text()?;
    reader.u64()?;
    reader.text()?;
    reader.u64()?;
    reader.u64()?;
    reader.i64()?;
    reader.i64()?;
    reader.i64()?;
    reader.bool()?;
    reader.u64()?;
    reader.u64()?;
    reader.i64()?;
    let intents = reader.strings()?;
    let lineage = reader.strings()?;
    let pending = reader.strings()?;
    let consumed = reader.strings()?;
    reader
        .is_finished()
        .then_some((intents, lineage, pending, consumed))
}

struct CanonicalPayloadReader<'a> {
    payload: &'a [u8],
    offset: usize,
}

impl<'a> CanonicalPayloadReader<'a> {
    fn new(payload: &'a [u8]) -> Self {
        Self { payload, offset: 0 }
    }

    fn bytes(&mut self) -> Option<&'a [u8]> {
        let length = usize::try_from(self.u64()?).ok()?;
        let end = self.offset.checked_add(length)?;
        let value = self.payload.get(self.offset..end)?;
        self.offset = end;
        Some(value)
    }

    fn text(&mut self) -> Option<&'a str> {
        std::str::from_utf8(self.bytes()?).ok()
    }

    fn strings(&mut self) -> Option<Vec<&'a str>> {
        let count = usize::try_from(self.u64()?).ok()?;
        let mut values = Vec::with_capacity(count.min(4_096));
        for _ in 0..count {
            values.push(self.text()?);
        }
        Some(values)
    }

    fn bool(&mut self) -> Option<bool> {
        match *self.payload.get(self.offset)? {
            0 => {
                self.offset += 1;
                Some(false)
            }
            1 => {
                self.offset += 1;
                Some(true)
            }
            _ => None,
        }
    }

    fn i64(&mut self) -> Option<i64> {
        let bytes = self.fixed::<8>()?;
        Some(i64::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Option<u64> {
        let bytes = self.fixed::<8>()?;
        Some(u64::from_le_bytes(bytes))
    }

    fn fixed<const LENGTH: usize>(&mut self) -> Option<[u8; LENGTH]> {
        let end = self.offset.checked_add(LENGTH)?;
        let bytes: [u8; LENGTH] = self.payload.get(self.offset..end)?.try_into().ok()?;
        self.offset = end;
        Some(bytes)
    }

    fn is_finished(&self) -> bool {
        self.offset == self.payload.len()
    }
}

fn append_source_range_payload(
    payload: &mut CanonicalPayload,
    range: &PersistentJournalSourceRange,
) {
    payload.text(&range.root_id);
    payload.u64(range.root_generation.value());
    payload.text(&range.volume.volume_guid);
    payload.u64(range.volume.volume_serial);
    payload.u64(range.journal_id.value());
    payload.i64(range.requested_start_usn.value());
    payload.i64(range.requested_end_usn.value());
    payload.i64(range.covered_until_usn.value());
    payload.bool(range.is_complete);
    payload.u64(u64::from(range.protocol_version));
    payload.u64(u64::from(range.contract_version));
    payload.i64(range.enrolled_unix_ms);
}

pub fn persistent_journal_batch_id_from_payload(payload: &[u8]) -> String {
    let mut digest = CanonicalDigest::new("persistent-journal-range-v5");
    digest.bytes(payload);
    digest.finish()
}

fn canonical_intent(intent: &LibraryChangeIntent) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        intent.root_id,
        intent.root_generation.value(),
        match intent.kind {
            super::LibraryChangeIntentKind::Reconcile => "reconcile",
            super::LibraryChangeIntentKind::RenameCandidate => "rename",
            super::LibraryChangeIntentKind::FreshnessUnknown => "freshness-unknown",
        },
        match intent.scope {
            super::LibraryChangeScope::Path => "path",
            super::LibraryChangeScope::Subtree => "subtree",
            super::LibraryChangeScope::Root => "root",
        },
        intent.relative_path,
        intent.previous_relative_path.as_deref().unwrap_or(""),
        match intent.origin {
            super::LibraryChangeOrigin::LiveNotification => "live",
            super::LibraryChangeOrigin::MetadataInventory => "metadata-inventory",
            super::LibraryChangeOrigin::StartupCatchUp => "startup",
            super::LibraryChangeOrigin::UserRefresh => "user-refresh",
            super::LibraryChangeOrigin::ConsistencyAudit => "audit",
        },
        intent.first_observed_unix_ms,
        intent.most_recent_observed_unix_ms,
        intent.first_sequence,
        intent.most_recent_sequence,
        intent.coalesced_observation_count,
    )
}

fn canonical_lineage(
    lineage: &PersistentJournalCrossRootLineage,
    current_range_id: &str,
) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        lineage.lineage_id,
        if lineage.owner_source_range_id == current_range_id {
            "self"
        } else {
            &lineage.owner_source_range_id
        },
        lineage.volume.volume_guid,
        lineage.volume.volume_serial,
        lineage.journal_id.value(),
        lineage.file_reference.to_canonical_text(),
        lineage.old_usn.map(JournalUsn::value).unwrap_or_default(),
        lineage.new_usn.map(JournalUsn::value).unwrap_or_default(),
        lineage.previous_carry_id.as_deref().unwrap_or(""),
        lineage.previous_root_id,
        lineage.previous_root_generation.value(),
        lineage.previous_relative_path,
        lineage.current_root_id,
        lineage.current_root_generation.value(),
        lineage.current_relative_path,
        match lineage.state {
            PersistentJournalLineageState::Pending => "pending",
            PersistentJournalLineageState::Completed => "completed",
            PersistentJournalLineageState::Superseded => "superseded",
        },
    )
}

fn canonical_pending_rename(pending: &PersistentJournalPendingRename) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
        pending.carry_id,
        "self",
        pending.volume.volume_guid,
        pending.volume.volume_serial,
        pending.journal_id.value(),
        pending.file_reference.to_canonical_text(),
        pending.old_usn.value(),
        pending.previous_root_id,
        pending.previous_root_generation.value(),
        pending.previous_relative_path,
        u8::from(pending.is_directory),
        pending.enrolled_unix_ms,
    )
}

struct CanonicalDigest(blake3::Hasher);

#[derive(Default)]
struct CanonicalPayload(Vec<u8>);

impl CanonicalPayload {
    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn strings(&mut self, values: &[String]) {
        self.u64(values.len() as u64);
        for value in values {
            self.text(value);
        }
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.0.extend_from_slice(value);
    }

    fn bool(&mut self, value: bool) {
        self.0.push(u8::from(value));
    }

    fn i64(&mut self, value: i64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn finish(self) -> Vec<u8> {
        self.0
    }
}

impl CanonicalDigest {
    fn new(namespace: &str) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(namespace.as_bytes());
        hasher.update(&[0]);
        Self(hasher)
    }

    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.0.update(value);
    }

    fn bool(&mut self, value: bool) {
        self.0.update(&[u8::from(value)]);
    }

    fn i64(&mut self, value: i64) {
        self.0.update(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.0.update(&value.to_le_bytes());
    }

    fn finish(self) -> String {
        self.0.finalize().to_hex().to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PersistentJournalEnrollmentReport {
    pub enrolled_root_count: u32,
    pub failed_root_count: u32,
    pub observation_count: u64,
    pub advanced_checkpoint_count: u32,
    pub root_failures: Vec<PersistentJournalRootFailure>,
}

fn decode_hex(value: &str) -> Result<Vec<u8>, ScanError> {
    if !value.len().is_multiple_of(2) {
        return Err(invalid_value("journal file reference"));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let encoded =
                std::str::from_utf8(pair).map_err(|_| invalid_value("journal file reference"))?;
            u8::from_str_radix(encoded, 16).map_err(|_| invalid_value("journal file reference"))
        })
        .collect()
}

fn validate_identifier(value: &str, max_len: usize, name: &str) -> Result<(), ScanError> {
    if value.trim().is_empty() || value.len() > max_len || value.contains('\0') {
        return Err(invalid_value(name));
    }
    Ok(())
}

fn validate_canonical_digest(value: &str, name: &str) -> Result<(), ScanError> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        return Err(invalid_value(name));
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), ScanError> {
    if value.is_empty()
        || value.len() > 32_767
        || value.contains('\0')
        || value.contains('\\')
        || value.starts_with('/')
        || value
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(invalid_value("journal relative path"));
    }
    Ok(())
}

fn invalid_value(name: &str) -> ScanError {
    ScanError::new(
        "persistent_journal_contract_invalid",
        format!("The {name} is outside the lossless persistent journal contract"),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        JournalFileReference, JournalIdentifier, JournalUsn, PERSISTENT_JOURNAL_CONTRACT_VERSION,
        PersistentJournalCheckpoint, PersistentJournalContinuityState,
        PersistentJournalVolumeIdentity,
    };
    use crate::domain::LibraryRootGeneration;

    #[test]
    fn canonical_numbers_reject_lossy_or_ambiguous_storage() {
        assert_eq!(
            JournalIdentifier::parse_canonical("18446744073709551615")
                .expect("maximum journal ID")
                .value(),
            u64::MAX
        );
        assert!(JournalIdentifier::parse_canonical("01").is_err());
        assert_eq!(
            JournalUsn::parse_canonical("9223372036854775807")
                .expect("maximum signed USN")
                .value(),
            i64::MAX
        );
        assert!(JournalUsn::parse_canonical("-1").is_err());
    }

    #[test]
    fn v2_and_v3_file_references_round_trip_without_width_loss() {
        let v2 =
            JournalFileReference::from_bytes(&[0, 1, 2, 3, 4, 5, 6, 255]).expect("V2 reference");
        let v3 = JournalFileReference::from_bytes(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 255,
        ])
        .expect("V3 reference");

        assert_eq!(
            JournalFileReference::parse_canonical(&v2.to_canonical_text()).expect("parse V2"),
            v2
        );
        assert_eq!(
            JournalFileReference::parse_canonical(&v3.to_canonical_text()).expect("parse V3"),
            v3
        );
        assert_ne!(v2.to_canonical_text(), v3.to_canonical_text());
    }

    #[test]
    fn checkpoint_preserves_v2_and_v3_root_references() {
        for root_file_reference in [
            JournalFileReference::V2([1; 8]),
            JournalFileReference::V3([2; 16]),
        ] {
            let checkpoint = PersistentJournalCheckpoint {
                root_id: "root".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: r"\\?\Volume{fixture}\".to_owned(),
                    volume_serial: u64::MAX,
                },
                root_file_reference,
                journal_id: JournalIdentifier::new(u64::MAX).expect("journal ID"),
                next_unread_usn: JournalUsn::new(i64::MAX).expect("USN"),
                captured_exclusive_end: JournalUsn::new(i64::MAX).expect("end USN"),
                covered_catalog_revision: 0,
                protocol_version: 1,
                contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                continuity: PersistentJournalContinuityState::Current,
                failure: None,
                updated_unix_ms: 0,
            };

            checkpoint.validate().expect("lossless checkpoint");
        }
    }
}
