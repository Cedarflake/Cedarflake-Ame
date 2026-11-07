use crate::domain::analysis_capacity::AnalysisCapacity;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

pub(crate) fn current_capacity() -> Option<AnalysisCapacity> {
    let mut memory = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // The initialized, correctly sized output lives for the complete synchronous call (ADR 0041).
    if unsafe { GlobalMemoryStatusEx(&raw mut memory) } == 0 {
        return None;
    }
    Some(AnalysisCapacity {
        processors: std::thread::available_parallelism().ok()?.get(),
        physical_bytes: memory.ullTotalPhys,
        available_bytes: memory.ullAvailPhys,
        commit_headroom_bytes: memory.ullAvailPageFile,
    })
}
