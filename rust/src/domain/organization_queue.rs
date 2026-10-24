use super::organization::OrganizationDisposition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganizationQueue {
    Pending,
    Deferred,
    Ignored,
}

impl OrganizationQueue {
    pub(crate) fn disposition(self) -> OrganizationDisposition {
        match self {
            Self::Pending => OrganizationDisposition::Pending,
            Self::Deferred => OrganizationDisposition::Deferred,
            Self::Ignored => OrganizationDisposition::Ignored,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrganizationCounts {
    pub total: u64,
    pub pending: u64,
    pub deferred: u64,
    pub organized: u64,
    pub ignored: u64,
}
