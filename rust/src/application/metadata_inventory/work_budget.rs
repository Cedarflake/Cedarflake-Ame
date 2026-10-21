use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub(super) enum InventoryWorkBudget {
    OnePage,
    TimeSlice {
        deadline: Instant,
    },
    #[cfg(test)]
    Complete,
}

impl InventoryWorkBudget {
    pub(super) fn recovery_slice() -> Self {
        Self::TimeSlice {
            deadline: Instant::now() + Duration::from_millis(250),
        }
    }

    pub(super) fn should_yield(self) -> bool {
        match self {
            Self::OnePage => true,
            Self::TimeSlice { deadline } => Instant::now() >= deadline,
            #[cfg(test)]
            Self::Complete => false,
        }
    }
}
