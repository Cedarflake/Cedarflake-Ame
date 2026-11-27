use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows_sys::core::w;

use crate::domain::application_update::{
    ApplicationUpdateChannel, ApplicationUpdateDelivery, ApplicationUpdateError,
    ApplicationUpdateFailureCode, ApplicationUpdateResult,
};
use crate::ports::application_updates::{ApplicationUpdatePackage, ApplicationUpdateSource};

pub(super) struct StoreApplicationUpdates;

impl ApplicationUpdateSource for StoreApplicationUpdates {
    fn delivery(&self) -> ApplicationUpdateDelivery {
        ApplicationUpdateDelivery::MicrosoftStore
    }

    fn take_result(&self) -> Result<Option<ApplicationUpdateResult>, ApplicationUpdateError> {
        Ok(None)
    }

    fn check(
        &self,
        _: ApplicationUpdateChannel,
    ) -> Result<Option<Box<dyn ApplicationUpdatePackage>>, ApplicationUpdateError> {
        Err(ApplicationUpdateError::new(
            ApplicationUpdateFailureCode::Unavailable,
            "Microsoft Store manages this package's updates",
        ))
    }

    fn open_store(&self) -> Result<(), ApplicationUpdateError> {
        // Only a fixed product URI is handed to the shell; no caller-supplied parameters.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                w!("open"),
                w!("ms-windows-store://pdp/?productid=9NRXBZ70WGVX"),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        } as isize;
        if result <= 32 {
            return Err(ApplicationUpdateError::new(
                ApplicationUpdateFailureCode::Store,
                format!("Could not open Microsoft Store (ShellExecuteW: {result})"),
            ));
        }
        Ok(())
    }
}
