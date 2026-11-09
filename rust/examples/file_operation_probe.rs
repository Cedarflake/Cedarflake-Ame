#[cfg(windows)]
#[path = "file_operation_probe/scenarios.rs"]
mod scenarios;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.len() != 2 {
        return Err("Expected scenario and guest-local result file".into());
    }
    let in_sandbox =
        std::env::var("USERNAME").is_ok_and(|name| name.eq_ignore_ascii_case("WDAGUtilityAccount"));
    if !in_sandbox {
        return Err("This mutation probe runs only inside Windows Sandbox".into());
    }
    if arguments[0] == "show-report" {
        use std::io::Read;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_OK, MessageBoxW};
        let mut report = String::new();
        std::fs::File::open(&arguments[1])?
            .take(16 * 1024 + 1)
            .read_to_string(&mut report)?;
        if report.len() > 16 * 1024 {
            return Err("The probe report is too large".into());
        }
        let text: Vec<u16> = std::ffi::OsStr::new(&report)
            .encode_wide()
            .chain(Some(0))
            .collect();
        let title: Vec<u16> = "Ame file-operation admission"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        // SAFETY: ADR 0042; the probe owns both terminated buffers for this synchronous dialog.
        unsafe {
            MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK);
        }
        return Ok(());
    }
    let base =
        PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("Missing guest-local storage")?)
            .join("AmeR8");
    std::fs::create_dir_all(&base)?;
    let fixture = tempfile::Builder::new()
        .prefix("native-adapter-")
        .tempdir_in(base)?;
    // Retain only these guest-owned samples for native Recycle Bin restoration and inspection.
    let retained = fixture.keep();
    let result = scenarios::run(&arguments[0], &retained);
    let text = match &result {
        Ok(receipt) => format!("PASS\r\nfixture={}\r\n{receipt}", retained.display()),
        Err(error) => format!("FAIL\r\nfixture={}\r\n{error}", retained.display()),
    };
    std::fs::write(&arguments[1], text)?;
    result.map(|_| ())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("This probe requires Windows Sandbox");
    std::process::exit(2);
}
