use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;

pub(super) fn acquire() -> Result<RuntimeOwnership<'static>, ScanError> {
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    RuntimeOwnership::acquire(&ACTIVE)
}

pub(super) struct RuntimeOwnership<'a> {
    active: &'a AtomicBool,
    retired: bool,
}

impl<'a> RuntimeOwnership<'a> {
    fn acquire(active: &'a AtomicBool) -> Result<Self, ScanError> {
        active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                ScanError::new(
                    "classification_runtime_active",
                    "The previous classification runtime still owns its resources",
                )
            })?;
        Ok(Self {
            active,
            retired: true,
        })
    }

    pub(super) fn require_retirement(&mut self) {
        self.retired = false;
    }
    pub(super) fn confirm_retirement(&mut self) {
        self.retired = true;
    }
}

impl Drop for RuntimeOwnership<'_> {
    fn drop(&mut self) {
        if self.retired {
            self.active.store(false, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_runtime_cannot_open_until_the_previous_owner_retires() {
        let active = AtomicBool::new(false);
        let mut owner = RuntimeOwnership::acquire(&active).unwrap();
        assert!(RuntimeOwnership::acquire(&active).is_err());
        owner.require_retirement();
        owner.confirm_retirement();
        drop(owner);
        drop(RuntimeOwnership::acquire(&active).unwrap());
    }

    #[test]
    fn unproven_retirement_cannot_authorize_another_model_owner() {
        let active = AtomicBool::new(false);
        let mut owner = RuntimeOwnership::acquire(&active).unwrap();
        owner.require_retirement();
        drop(owner);
        assert!(RuntimeOwnership::acquire(&active).is_err());
    }
}
