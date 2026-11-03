use super::{DiscoveredFile, ScanIssue};

pub enum FileVisitOutcome {
    File(DiscoveredFile),
    TerminalMedia {
        file: DiscoveredFile,
        issue: ScanIssue,
        report_issue: bool,
    },
    RetryableFile {
        file: DiscoveredFile,
        issue: ScanIssue,
    },
    Issue(ScanIssue),
    Directory,
    Ignored,
}

pub struct FileVisit {
    pub relative_path: String,
    pub outcome: FileVisitOutcome,
}
