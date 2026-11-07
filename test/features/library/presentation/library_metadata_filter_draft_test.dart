import "package:cedarflake_ame/features/library/domain/library_metadata_filter.dart";
import "package:cedarflake_ame/features/library/presentation/coordination/library_metadata_filter_draft.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("editing preserves exact stored bytes and explicit date meaning", () {
    const filter = LibraryMetadataFilter(
      extension: "png",
      bytes: LibraryNumberRange(
        minimum: 9007199254740993,
        maximum: 9223372036854775807,
      ),
      width: LibraryNumberRange(minimum: 1),
      height: LibraryNumberRange(maximum: 3000),
      date: LibraryDateRange(
        field: LibraryDateField.modified,
        start: "2024-02-29",
      ),
      cameraText: "α7 相机",
    );
    final draft = LibraryMetadataFilterDraft.fromFilter(filter);
    expect(draft.parse().filter, filter);
    expect(draft.sizeUnit, LibraryFilterSizeUnit.bytes);
  });

  test(
    "invalid conditions never publish a partial or silently widened filter",
    () {
      final cases = <Map<LibraryFilterField, String>>[
        {LibraryFilterField.minimumBytes: "9223372036854775808"},
        {LibraryFilterField.minimumBytes: "0.1"},
        {LibraryFilterField.minimumWidth: "-1"},
        {
          LibraryFilterField.minimumWidth: "3",
          LibraryFilterField.maximumWidth: "2",
        },
        {LibraryFilterField.dateStart: "2025-02-29"},
        {LibraryFilterField.dateStart: "0000-01-01"},
        {
          LibraryFilterField.dateStart: "2026-04-01",
          LibraryFilterField.dateEnd: "2026-03-31",
        },
        {LibraryFilterField.extension: "*.png"},
        {LibraryFilterField.camera: "Sony\u0001camera"},
      ];
      for (final values in cases) {
        final result = LibraryMetadataFilterDraft(
          values: values,
          sizeUnit: LibraryFilterSizeUnit.bytes,
          dateField: LibraryDateField.capture,
        ).parse();
        expect(result.filter, isNull, reason: values.toString());
        expect(
          result.errors.keys,
          anyElement(isIn(values.keys)),
          reason: values.toString(),
        );
      }
    },
  );
}
