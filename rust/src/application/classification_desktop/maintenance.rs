use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::domain::ScanError;
use crate::domain::classification::maintenance::CategoryMaintenancePlan;

use super::super::classification_runtime::{
    PreparedCategoryMaintenance, commit_category_maintenance,
};

pub(crate) struct ClassificationDesktopMaintenance {
    database: PathBuf,
    plan: CategoryMaintenancePlan,
    cancelled: Arc<AtomicBool>,
}

impl ClassificationDesktopMaintenance {
    pub(crate) fn new(database: PathBuf, plan: CategoryMaintenancePlan) -> Self {
        Self {
            database,
            plan,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn commit(&self) -> Result<bool, ScanError> {
        match commit_category_maintenance(
            &self.database,
            PreparedCategoryMaintenance::from(&self.plan),
            &self.cancelled,
        ) {
            Ok(_) => Ok(true),
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "classification_changed"
                        | "classification_discovery_sources_changed"
                        | "classification_cancelled"
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
