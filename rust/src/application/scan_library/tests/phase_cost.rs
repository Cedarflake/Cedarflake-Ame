use std::time::Duration;

use crate::domain::ScanEvent;

/// Event boundaries bound phases; they do not attribute I/O or scheduler time.
pub(super) struct ScanPhaseCost {
    scan_id: &'static str,
    expected_items: u64,
    last_observation: Duration,
    started: Option<Duration>,
    finalizing: Option<Duration>,
    validated: Option<Duration>,
    completed: Option<Duration>,
    validated_items: u64,
    complete_counter_reports: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ScanPhaseReport {
    admission: Duration,
    discovery: Duration,
    validation: Duration,
    publication: Duration,
    retirement: Duration,
    complete_counter_reports: u64,
}

impl ScanPhaseCost {
    pub(super) fn new(scan_id: &'static str, expected_items: u64) -> Self {
        Self {
            scan_id,
            expected_items,
            last_observation: Duration::ZERO,
            started: None,
            finalizing: None,
            validated: None,
            completed: None,
            validated_items: 0,
            complete_counter_reports: 0,
        }
    }

    pub(super) fn observe(
        &mut self,
        event: &ScanEvent,
        elapsed: Duration,
    ) -> Result<(), &'static str> {
        if elapsed < self.last_observation {
            return Err("non-monotonic scan observation");
        }
        let scan_id = match event {
            ScanEvent::Started { scan_id, .. }
            | ScanEvent::Progress { scan_id, .. }
            | ScanEvent::Finalizing { scan_id, .. }
            | ScanEvent::AssetDiscovered { scan_id, .. }
            | ScanEvent::Issue { scan_id, .. }
            | ScanEvent::Completed { scan_id, .. }
            | ScanEvent::Cancelled { scan_id, .. }
            | ScanEvent::Paused { scan_id, .. }
            | ScanEvent::Stale { scan_id, .. }
            | ScanEvent::Failed { scan_id, .. } => scan_id,
        };
        if scan_id != self.scan_id {
            return Err("foreign scan observation");
        }
        if self.completed.is_some() {
            return Err("scan observation after completion");
        }
        if self.started.is_none() && !matches!(event, ScanEvent::Started { .. }) {
            return Err("scan observation before start");
        }
        self.last_observation = elapsed;
        match event {
            ScanEvent::Started { .. } => {
                if self.started.is_some() {
                    return Err("repeated scan start");
                }
                self.started = Some(elapsed);
            }
            ScanEvent::Finalizing {
                validated_items,
                total_items,
                ..
            } => {
                if self.finalizing.is_none() && *validated_items != 0 {
                    return Err("missing initial validation boundary");
                }
                if *total_items != self.expected_items
                    || *validated_items > *total_items
                    || *validated_items < self.validated_items
                {
                    return Err("inconsistent validation counters");
                }
                self.finalizing.get_or_insert(elapsed);
                self.validated_items = *validated_items;
                if validated_items == total_items {
                    self.validated.get_or_insert(elapsed);
                    self.complete_counter_reports += 1;
                }
            }
            ScanEvent::Completed {
                asset_count,
                issue_count,
                was_limited,
                ..
            } => {
                if *asset_count != self.expected_items
                    || self.validated.is_none()
                    || *issue_count != 0
                    || *was_limited
                {
                    return Err("completion lacks full current validation");
                }
                self.completed = Some(elapsed);
            }
            ScanEvent::Progress { .. } | ScanEvent::AssetDiscovered { .. } => {
                if self.finalizing.is_some() {
                    return Err("discovery observation after finalization");
                }
            }
            ScanEvent::Issue { .. } => return Err("unexpected issue in successful workload"),
            ScanEvent::Cancelled { .. }
            | ScanEvent::Paused { .. }
            | ScanEvent::Stale { .. }
            | ScanEvent::Failed { .. } => return Err("unsuccessful terminal scan observation"),
        }
        Ok(())
    }

    pub(super) fn finish(self, elapsed: Duration) -> Result<ScanPhaseReport, &'static str> {
        if elapsed < self.last_observation {
            return Err("scan return precedes its last observation");
        }
        let started = self.started.ok_or("missing scan start")?;
        let finalizing = self.finalizing.ok_or("missing finalization")?;
        let validated = self.validated.ok_or("missing complete validation")?;
        let completed = self.completed.ok_or("missing published completion")?;
        Ok(ScanPhaseReport {
            admission: started,
            discovery: finalizing - started,
            validation: validated - finalizing,
            publication: completed - validated,
            retirement: elapsed - completed,
            complete_counter_reports: self.complete_counter_reports,
        })
    }
}

impl ScanPhaseReport {
    pub(super) fn print(&self, operation: &str) {
        println!(
            "AME_SYNTHETIC_SCAN_PHASE operation={operation} admission_us={} discovery_us={} \
             validation_us={} publication_us={} retirement_us={} complete_counter_reports={}",
            self.admission.as_micros(),
            self.discovery.as_micros(),
            self.validation.as_micros(),
            self.publication.as_micros(),
            self.retirement.as_micros(),
            self.complete_counter_reports,
        );
    }
}

#[cfg(test)]
mod tests;
