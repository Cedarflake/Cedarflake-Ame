use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};

use crate::journal_broker::wire::WireError;

#[cfg(test)]
std::thread_local! {
    static CALL_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(in crate::journal_broker) fn ordinal_equals(
    left: &[u16],
    right: &[u16],
    ignore_case: bool,
) -> Result<bool, WireError> {
    #[cfg(test)]
    CALL_COUNT.with(|count| count.set(count.get() + 1));
    let left_len = i32::try_from(left.len()).map_err(|_| WireError::LengthOverflow)?;
    let right_len = i32::try_from(right.len()).map_err(|_| WireError::LengthOverflow)?;

    // SAFETY: the explicit UTF-16 unit counts fit i32, both slices remain live for the call,
    // CompareStringOrdinal does not retain either pointer, and a zero result fails closed below.
    let result = unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left_len,
            right.as_ptr(),
            right_len,
            i32::from(ignore_case),
        )
    };
    if result == 0 {
        return Err(WireError::NameComparisonUnavailable);
    }
    Ok(result == CSTR_EQUAL)
}

#[cfg(test)]
pub(in crate::journal_broker) fn reset_call_count() {
    CALL_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(in crate::journal_broker) fn call_count() -> usize {
    CALL_COUNT.with(std::cell::Cell::get)
}

#[cfg(test)]
mod tests {
    use super::ordinal_equals;

    #[test]
    fn ordinal_comparer_supports_empty_explicit_length_slices() {
        assert!(ordinal_equals(&[], &[], true).expect("empty comparison"));
    }

    #[test]
    fn ordinal_comparer_uses_windows_ignore_case_semantics() {
        let upper: Vec<u16> = "PHOTOS".encode_utf16().collect();
        let lower: Vec<u16> = "photos".encode_utf16().collect();
        assert!(ordinal_equals(&upper, &lower, true).expect("ordinal ignore-case comparison"));
        assert!(!ordinal_equals(&upper, &lower, false).expect("ordinal exact comparison"));
    }
}
