use serde::{Deserialize, Serialize};

use super::ScanError;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryNumberRange {
    pub minimum: Option<u64>,
    pub maximum: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GalleryDateField {
    #[default]
    Capture,
    Created,
    Modified,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryDateRange {
    pub field: GalleryDateField,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryMetadataFilter {
    pub extension: String,
    pub bytes: GalleryNumberRange,
    pub width: GalleryNumberRange,
    pub height: GalleryNumberRange,
    pub date: GalleryDateRange,
    pub camera_text: String,
}

impl GalleryMetadataFilter {
    pub(crate) fn normalize(&mut self) {
        self.extension = self
            .extension
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        self.camera_text = self.camera_text.trim().to_owned();
        if self.date.start.is_none() && self.date.end.is_none() {
            self.date.field = GalleryDateField::Capture;
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ScanError> {
        if self.extension.len() > 16
            || !self
                .extension
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric())
            || self.camera_text.chars().count() > 128
            || self.camera_text.chars().any(char::is_control)
        {
            return Err(invalid());
        }
        for range in [&self.bytes, &self.width, &self.height] {
            if range.minimum.is_some_and(|value| value > i64::MAX as u64)
                || range.maximum.is_some_and(|value| value > i64::MAX as u64)
                || matches!((range.minimum, range.maximum), (Some(min), Some(max)) if min > max)
            {
                return Err(invalid());
            }
        }
        for date in [&self.date.start, &self.date.end].into_iter().flatten() {
            if !valid_date(date) {
                return Err(invalid());
            }
        }
        if matches!((&self.date.start, &self.date.end), (Some(start), Some(end)) if start > end) {
            return Err(invalid());
        }
        Ok(())
    }
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return false;
    }
    let year = value[..4].parse::<u32>().unwrap_or(0);
    let month = value[5..7].parse::<u32>().unwrap_or(0);
    let day = value[8..].parse::<u32>().unwrap_or(0);
    let days = match month {
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || year.is_multiple_of(4) && !year.is_multiple_of(100) => 29,
        2 => 28,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    year > 0 && day > 0 && day <= days
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_metadata_filter_invalid",
        "Metadata filters require valid dates, ordered ranges and bounded literal text",
    )
}

#[cfg(test)]
mod tests;
