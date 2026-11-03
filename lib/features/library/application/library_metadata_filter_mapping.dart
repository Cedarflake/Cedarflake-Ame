import "../../../src/rust/domain/gallery_metadata_filter.dart" as rust;
import "../domain/library_metadata_filter.dart";

rust.GalleryMetadataFilter mapLibraryMetadataFilter(
  LibraryMetadataFilter filter,
) => rust.GalleryMetadataFilter(
  extension_: filter.extension,
  bytes: _toRange(filter.bytes),
  width: _toRange(filter.width),
  height: _toRange(filter.height),
  date: rust.GalleryDateRange(
    field: switch (filter.date.field) {
      LibraryDateField.capture => rust.GalleryDateField.capture,
      LibraryDateField.created => rust.GalleryDateField.created,
      LibraryDateField.modified => rust.GalleryDateField.modified,
    },
    start: filter.date.start,
    end: filter.date.end,
  ),
  cameraText: filter.cameraText,
);

LibraryMetadataFilter mapRustMetadataFilter(
  rust.GalleryMetadataFilter filter,
) => LibraryMetadataFilter(
  extension: filter.extension_,
  bytes: _fromRange(filter.bytes),
  width: _fromRange(filter.width),
  height: _fromRange(filter.height),
  date: LibraryDateRange(
    field: switch (filter.date.field) {
      rust.GalleryDateField.capture => LibraryDateField.capture,
      rust.GalleryDateField.created => LibraryDateField.created,
      rust.GalleryDateField.modified => LibraryDateField.modified,
    },
    start: filter.date.start,
    end: filter.date.end,
  ),
  cameraText: filter.cameraText,
);

rust.GalleryNumberRange _toRange(LibraryNumberRange range) =>
    rust.GalleryNumberRange(
      minimum: range.minimum == null ? null : BigInt.from(range.minimum!),
      maximum: range.maximum == null ? null : BigInt.from(range.maximum!),
    );

LibraryNumberRange _fromRange(rust.GalleryNumberRange range) =>
    LibraryNumberRange(
      minimum: range.minimum?.toInt(),
      maximum: range.maximum?.toInt(),
    );
