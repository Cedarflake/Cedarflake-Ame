#[cfg(windows)]
#[test]
fn journal_broker_binary_exposes_protocol_probe_and_requires_scm_for_service_mode() {
    let binary = env!("CARGO_BIN_EXE_cedarflake_ame_journal_broker");
    let probe = std::process::Command::new(binary)
        .arg("--protocol-info")
        .output()
        .expect("run broker protocol probe");
    assert!(probe.status.success());
    let output = String::from_utf8(probe.stdout).expect("UTF-8 protocol probe");
    assert!(output.contains("protocol=5"));
    assert!(output.contains("max_frame=1048576"));

    let outside_scm = std::process::Command::new(binary)
        .output()
        .expect("run broker outside SCM");
    assert!(!outside_scm.status.success());
    let error = String::from_utf8(outside_scm.stderr).expect("UTF-8 broker error");
    assert!(error.contains("journal_broker_not_started_by_scm"));
}

#[cfg(windows)]
#[test]
fn disposable_console_host_preserves_source_and_restarts_with_fresh_identity() {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::windows::io::FromRawHandle;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    fn exchange(
        binary: &str,
        request: Option<&[u8]>,
    ) -> ([u8; 48], std::process::ExitStatus, String) {
        let mut child = Command::new(binary)
            .arg("--console")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start disposable broker host");
        let stdout = child.stdout.take().expect("console host stdout");
        let mut output = BufReader::new(stdout);
        let mut pipe_line = String::new();
        output.read_line(&mut pipe_line).expect("read pipe name");
        let pipe_name = pipe_line
            .trim()
            .strip_prefix("pipe=")
            .expect("disposable pipe announcement");
        assert!(pipe_name.starts_with(r"\\.\pipe\CedarflakeAme.JournalBroker.Test."));

        let mut pipe_name_utf16: Vec<u16> = pipe_name.encode_utf16().collect();
        pipe_name_utf16.push(0);
        // SAFETY: the path is the bounded, NUL-terminated name emitted by this disposable child;
        // the checked handle transfers to File as its sole owner and no pointer escapes the call.
        let raw_pipe = unsafe {
            windows_sys::Win32::Storage::FileSystem::CreateFileW(
                pipe_name_utf16.as_ptr(),
                windows_sys::Win32::Foundation::GENERIC_READ
                    | windows_sys::Win32::Foundation::GENERIC_WRITE,
                0,
                std::ptr::null(),
                windows_sys::Win32::Storage::FileSystem::OPEN_EXISTING,
                windows_sys::Win32::Storage::FileSystem::SECURITY_IMPERSONATION
                    | windows_sys::Win32::Storage::FileSystem::SECURITY_SQOS_PRESENT,
                std::ptr::null_mut(),
            )
        };
        assert_ne!(
            raw_pipe,
            windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE,
            "connect disposable named pipe: {}",
            std::io::Error::last_os_error()
        );
        // SAFETY: raw_pipe was checked above and ownership transfers exactly once to File.
        let mut pipe = unsafe { std::fs::File::from_raw_handle(raw_pipe.cast()) };
        let mut handshake = [0_u8; 48];
        pipe.read_exact(&mut handshake)
            .expect("read server handshake");
        let mut client_proof = handshake;
        client_proof[..8].copy_from_slice(b"AMEJCP3\0");
        pipe.write_all(&client_proof[..7])
            .expect("write partial connection-bound client proof");
        std::thread::sleep(Duration::from_millis(20));
        pipe.write_all(&client_proof[7..])
            .expect("complete connection-bound client proof");
        let mut server_accept = [0_u8; 48];
        pipe.read_exact(&mut server_accept)
            .expect("read server admission acknowledgement");
        assert_eq!(&server_accept[..8], b"AMEJOK3\0");
        assert_eq!(&server_accept[8..], &handshake[8..]);
        if let Some(request) = request {
            pipe.write_all(request).expect("write disposable request");
        }
        drop(pipe);

        let deadline = Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(status) = child.try_wait().expect("poll console host") {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().expect("terminate stuck console host");
                panic!("disposable console host exceeded its bounded shutdown");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let mut error = String::new();
        child
            .stderr
            .take()
            .expect("console host stderr")
            .read_to_string(&mut error)
            .expect("read console host stderr");
        (handshake, status, error)
    }

    fn assert_handshake(handshake: &[u8; 48]) {
        assert_eq!(&handshake[..8], b"AMEJH3\0\0");
        assert_eq!(u16::from_le_bytes([handshake[8], handshake[9]]), 5);
        assert_eq!(&handshake[10..16], &[0; 6]);
        assert_ne!(&handshake[16..32], &[0; 16]);
        assert_ne!(&handshake[32..40], &[0; 8]);
        assert_ne!(&handshake[40..48], &[0; 8]);
    }

    fn assert_local_ntfs(path: &std::path::Path) {
        let mut path_utf16: Vec<u16> = path.as_os_str().to_string_lossy().encode_utf16().collect();
        path_utf16.push(0);
        let mut volume_path = [0_u16; 261];
        // SAFETY: both buffers are initialized, bounded, NUL-terminated where required, and live
        // for the synchronous metadata-only query. No pointer escapes.
        assert_ne!(
            unsafe {
                windows_sys::Win32::Storage::FileSystem::GetVolumePathNameW(
                    path_utf16.as_ptr(),
                    volume_path.as_mut_ptr(),
                    u32::try_from(volume_path.len()).expect("volume path capacity"),
                )
            },
            0
        );
        let mut filesystem = [0_u16; 32];
        // SAFETY: volume_path was filled by GetVolumePathNameW and all optional outputs are null;
        // filesystem is a live bounded output buffer and the call cannot mutate the source root.
        assert_ne!(
            unsafe {
                windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW(
                    volume_path.as_ptr(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    filesystem.as_mut_ptr(),
                    u32::try_from(filesystem.len()).expect("filesystem capacity"),
                )
            },
            0
        );
        let length = filesystem
            .iter()
            .position(|unit| *unit == 0)
            .expect("terminated filesystem name");
        assert_eq!(
            String::from_utf16(&filesystem[..length])
                .expect("filesystem UTF-16")
                .to_ascii_uppercase(),
            "NTFS"
        );
    }

    let source = tempfile::tempdir().expect("disposable source root");
    assert_local_ntfs(source.path());
    let media_path = source.path().join("untouched.jpg");
    let media_bytes = b"broker-must-not-open-media-content";
    std::fs::write(&media_path, media_bytes).expect("write disposable source fixture");
    let before = std::fs::metadata(&media_path).expect("source metadata before broker");

    let binary = env!("CARGO_BIN_EXE_cedarflake_ame_journal_broker");
    let (first, first_status, first_error) = exchange(binary, None);
    assert_handshake(&first);
    assert!(first_status.success(), "{first_error}");
    let (second, second_status, second_error) = exchange(binary, None);
    assert_handshake(&second);
    assert!(second_status.success(), "{second_error}");
    assert_ne!(&first[16..32], &second[16..32]);
    assert_ne!(&first[40..48], &second[40..48]);

    let (_, malformed_status, _) = exchange(binary, Some(&0_u32.to_le_bytes()));
    assert!(!malformed_status.success());
    let after = std::fs::metadata(&media_path).expect("source metadata after broker");
    assert_eq!(after.len(), before.len());
    assert_eq!(
        std::fs::read(&media_path).expect("verify disposable source bytes"),
        media_bytes
    );
    assert_eq!(
        std::fs::read_dir(source.path())
            .expect("source entries")
            .count(),
        1
    );
}

#[cfg(windows)]
#[test]
fn console_host_rejects_missing_client_proof_with_bounded_timeout() {
    use std::io::{BufRead, BufReader, Read};
    use std::os::windows::io::FromRawHandle;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let binary = env!("CARGO_BIN_EXE_cedarflake_ame_journal_broker");
    let mut child = Command::new(binary)
        .arg("--console")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start disposable broker host");
    let stdout = child.stdout.take().expect("console host stdout");
    let mut output = BufReader::new(stdout);
    let mut pipe_line = String::new();
    output.read_line(&mut pipe_line).expect("read pipe name");
    let pipe_name = pipe_line
        .trim()
        .strip_prefix("pipe=")
        .expect("disposable pipe announcement");
    let mut pipe_name_utf16: Vec<u16> = pipe_name.encode_utf16().collect();
    pipe_name_utf16.push(0);
    // SAFETY: the bounded, NUL-terminated pipe name came from this disposable child. The checked
    // handle transfers to File exactly once and requests impersonation-level SQOS.
    let raw_pipe = unsafe {
        windows_sys::Win32::Storage::FileSystem::CreateFileW(
            pipe_name_utf16.as_ptr(),
            windows_sys::Win32::Foundation::GENERIC_READ
                | windows_sys::Win32::Foundation::GENERIC_WRITE,
            0,
            std::ptr::null(),
            windows_sys::Win32::Storage::FileSystem::OPEN_EXISTING,
            windows_sys::Win32::Storage::FileSystem::SECURITY_IMPERSONATION
                | windows_sys::Win32::Storage::FileSystem::SECURITY_SQOS_PRESENT,
            std::ptr::null_mut(),
        )
    };
    assert_ne!(
        raw_pipe,
        windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE,
        "connect disposable named pipe: {}",
        std::io::Error::last_os_error()
    );
    // SAFETY: raw_pipe was checked above and ownership transfers exactly once to File.
    let mut pipe = unsafe { std::fs::File::from_raw_handle(raw_pipe.cast()) };
    let mut handshake = [0_u8; 48];
    pipe.read_exact(&mut handshake)
        .expect("read server handshake");
    assert_eq!(&handshake[..8], b"AMEJH3\0\0");

    let started_wait = Instant::now();
    let deadline = started_wait + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll console host") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("terminate stuck console host");
            panic!("console host did not bound a missing client proof");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(!status.success());
    assert!(started_wait.elapsed() < Duration::from_secs(5));
    let mut error = String::new();
    child
        .stderr
        .take()
        .expect("console host stderr")
        .read_to_string(&mut error)
        .expect("read console host stderr");
    assert!(error.contains("exceeded its bounded deadline"), "{error}");
}
