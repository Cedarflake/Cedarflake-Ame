use super::file_operation_exports;

pub(super) fn batches_schema() -> String {
    file_operation_exports::BATCHES.replace("'source'", "'source', 'album_year'")
}
