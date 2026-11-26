/// flutter_rust_bridge:ignore
#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "C" fn ame_application_update_startup() -> i32 {
    std::panic::catch_unwind(crate::adapters::application_updates::startup)
        .map(|()| 0)
        .unwrap_or(1)
}
