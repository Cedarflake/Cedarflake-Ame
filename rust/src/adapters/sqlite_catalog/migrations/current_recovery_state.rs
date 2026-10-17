use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{SCHEMA_VERSION, database_error};

enum RecoveryRepair {
    TerminalInventory,
    RetiredLiveGap,
    RemovedRootAuthority,
    RetainedSourceState,
    ForegroundScanCapture,
}

impl RecoveryRepair {
    fn needed(&self, connection: &Connection) -> Result<bool, ScanError> {
        match self {
            Self::TerminalInventory => {
                super::terminal_inventory_repair::needed(connection, SCHEMA_VERSION)
            }
            Self::RetiredLiveGap => {
                super::retired_live_gap_repair::needed(connection, SCHEMA_VERSION)
            }
            Self::RemovedRootAuthority => super::retired_root_authority::repair_needed(connection),
            Self::RetainedSourceState => super::retained_source_state::repair_needed(connection),
            Self::ForegroundScanCapture => {
                super::foreground_scan_capture::repair_needed(connection)
            }
        }
    }

    fn apply(&self, transaction: &Transaction<'_>) -> Result<(), ScanError> {
        match self {
            Self::TerminalInventory => {
                super::terminal_inventory_repair::apply(transaction, SCHEMA_VERSION)
            }
            Self::RetiredLiveGap => {
                super::retired_live_gap_repair::apply(transaction, SCHEMA_VERSION)
            }
            Self::RemovedRootAuthority => {
                super::retired_root_authority::repair_in_transaction(transaction)
            }
            Self::RetainedSourceState => {
                super::retained_source_state::repair_in_transaction(transaction)
            }
            Self::ForegroundScanCapture => {
                super::foreground_scan_capture::repair_in_transaction(transaction)
            }
        }
    }
}

pub(super) fn repair_and_validate(connection: &mut Connection) -> Result<(), ScanError> {
    let repairs = [
        RecoveryRepair::TerminalInventory,
        RecoveryRepair::RetiredLiveGap,
        RecoveryRepair::RemovedRootAuthority,
        RecoveryRepair::RetainedSourceState,
        RecoveryRepair::ForegroundScanCapture,
    ];
    let mut needs_repair = false;
    for repair in &repairs {
        needs_repair |= repair.needed(connection)?;
    }
    if !needs_repair {
        return super::current_schema::validate_current_schema_contract(connection);
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    for repair in &repairs {
        repair.apply(&transaction)?;
    }
    super::current_schema::validate_current_schema_contract_with_source_revision_rows(
        &transaction,
    )?;
    transaction.commit().map_err(database_error)
}

#[cfg(test)]
mod tests;
