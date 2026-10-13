#[derive(Clone, Copy)]
pub(super) struct JournalReadInterval {
    start_usn: i64,
    end_usn: i64,
}

impl JournalReadInterval {
    pub(super) fn new(start_usn: i64, end_usn: i64) -> Option<Self> {
        if start_usn < 0 || end_usn < start_usn {
            return None;
        }
        Some(Self { start_usn, end_usn })
    }

    pub(super) fn is_empty(self) -> bool {
        self.start_usn == self.end_usn
    }

    pub(super) fn admits_coverage(self, covered_until_usn: i64, is_complete: bool) -> bool {
        if covered_until_usn < self.start_usn || covered_until_usn > self.end_usn {
            return false;
        }
        if self.is_empty() {
            return covered_until_usn == self.end_usn && is_complete;
        }
        covered_until_usn > self.start_usn && is_complete == (covered_until_usn == self.end_usn)
    }
}

#[cfg(test)]
mod tests;
