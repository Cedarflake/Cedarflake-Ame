use std::io;

use windows_sys::Win32::Foundation::{APPMODEL_ERROR_NO_PACKAGE, ERROR_SUCCESS};
use windows_sys::Win32::Storage::Packaging::Appx::GetCurrentPackageFamilyName;

pub(crate) const STORE_PACKAGE_FAMILY: &str = "Cedarflake.Cedarflake-Ame_t341beqjcwv0y";

pub(crate) fn current_package_family() -> io::Result<Option<String>> {
    let mut buffer = [0_u16; 128];
    let mut length = buffer.len() as u32;
    // Both outputs live through the call; the returned length is checked before slicing.
    let status = unsafe { GetCurrentPackageFamilyName(&mut length, buffer.as_mut_ptr()) };
    decode_family(status, length as usize, &buffer)
}

fn decode_family(status: u32, length: usize, buffer: &[u16]) -> io::Result<Option<String>> {
    if status == APPMODEL_ERROR_NO_PACKAGE {
        return Ok(None);
    }
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let family = buffer
        .get(..length)
        .filter(|value| value.len() > 1 && value.last() == Some(&0))
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Invalid package family length")
        })?;
    let value = &family[..family.len() - 1];
    if value.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid package family terminator",
        ));
    }
    String::from_utf16(value)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;

    #[test]
    fn only_explicit_no_package_status_is_unpackaged() {
        assert_eq!(
            decode_family(APPMODEL_ERROR_NO_PACKAGE, 0, &[]).unwrap(),
            None
        );
        assert!(decode_family(ERROR_INSUFFICIENT_BUFFER, 500, &[0; 128]).is_err());
        assert!(decode_family(ERROR_SUCCESS, 129, &[0; 128]).is_err());
        assert!(decode_family(ERROR_SUCCESS, 1, &[0]).is_err());
    }

    #[test]
    fn admits_complete_utf16_family_and_rejects_malformed_output() {
        let value: Vec<_> = STORE_PACKAGE_FAMILY.encode_utf16().chain([0]).collect();
        assert_eq!(
            decode_family(ERROR_SUCCESS, value.len(), &value)
                .unwrap()
                .as_deref(),
            Some(STORE_PACKAGE_FAMILY)
        );
        for value in [&[65, 0, 66, 0][..], &[65, 66], &[0xd800, 0]] {
            assert!(decode_family(ERROR_SUCCESS, value.len(), value).is_err());
        }
    }
}
