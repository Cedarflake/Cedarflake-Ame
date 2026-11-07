import "../../domain/library_metadata_filter.dart";
import "../library_filter_strings.dart";

enum LibraryFilterField {
  extension,
  minimumBytes,
  maximumBytes,
  minimumWidth,
  maximumWidth,
  minimumHeight,
  maximumHeight,
  dateStart,
  dateEnd,
  camera,
}

enum LibraryFilterSizeUnit {
  bytes(1, "B"),
  kilobytes(1024, "KB"),
  megabytes(1024 * 1024, "MB"),
  gigabytes(1024 * 1024 * 1024, "GB");

  const LibraryFilterSizeUnit(this.multiplier, this.label);
  final int multiplier;
  final String label;

  static LibraryFilterSizeUnit forRange(LibraryNumberRange range) {
    final bounds = [range.minimum, range.maximum].nonNulls.toList();
    if (bounds.isEmpty || bounds.every((value) => value == 0)) return megabytes;
    return values.reversed.firstWhere(
      (unit) => bounds.every((value) => value % unit.multiplier == 0),
    );
  }
}

class LibraryMetadataFilterDraft {
  LibraryMetadataFilterDraft({
    required Map<LibraryFilterField, String> values,
    required this.sizeUnit,
    required this.dateField,
  }) : values = Map.unmodifiable(values);

  factory LibraryMetadataFilterDraft.fromFilter(LibraryMetadataFilter filter) {
    final unit = LibraryFilterSizeUnit.forRange(filter.bytes);
    String number(int? value, [int divisor = 1]) =>
        value == null ? "" : (value ~/ divisor).toString();
    return LibraryMetadataFilterDraft(
      values: {
        LibraryFilterField.extension: filter.extension,
        LibraryFilterField.minimumBytes: number(
          filter.bytes.minimum,
          unit.multiplier,
        ),
        LibraryFilterField.maximumBytes: number(
          filter.bytes.maximum,
          unit.multiplier,
        ),
        LibraryFilterField.minimumWidth: number(filter.width.minimum),
        LibraryFilterField.maximumWidth: number(filter.width.maximum),
        LibraryFilterField.minimumHeight: number(filter.height.minimum),
        LibraryFilterField.maximumHeight: number(filter.height.maximum),
        LibraryFilterField.dateStart: filter.date.start ?? "",
        LibraryFilterField.dateEnd: filter.date.end ?? "",
        LibraryFilterField.camera: filter.cameraText,
      },
      sizeUnit: unit,
      dateField: filter.date.field,
    );
  }

  final Map<LibraryFilterField, String> values;
  final LibraryFilterSizeUnit sizeUnit;
  final LibraryDateField dateField;

  ({LibraryMetadataFilter? filter, Map<LibraryFilterField, String> errors})
  parse() {
    final errors = <LibraryFilterField, String>{};
    String text(LibraryFilterField field) => (values[field] ?? "").trim();
    int? number(LibraryFilterField field, {bool bytes = false}) {
      final value = text(field);
      if (value.isEmpty) return null;
      final pattern = bytes ? r"^\d+(?:\.\d+)?$" : r"^\d+$";
      if (value.length > 40 || !RegExp(pattern).hasMatch(value)) {
        errors[field] = bytes
            ? LibraryFilterStrings.invalidNumber
            : LibraryFilterStrings.invalidInteger;
        return null;
      }
      final parts = value.split(".");
      final scale = BigInt.from(
        10,
      ).pow(parts.length == 2 ? parts[1].length : 0);
      final amount =
          BigInt.parse(parts.join()) *
          BigInt.from(bytes ? sizeUnit.multiplier : 1);
      if (amount % scale != BigInt.zero) {
        errors[field] = LibraryFilterStrings.fractionalBytes;
        return null;
      }
      final result = amount ~/ scale;
      if (result > BigInt.from(0x7fffffffffffffff)) {
        errors[field] = LibraryFilterStrings.numberTooLarge;
        return null;
      }
      return result.toInt();
    }

    LibraryNumberRange range(
      LibraryFilterField minimum,
      LibraryFilterField maximum, {
      bool bytes = false,
    }) {
      final lower = number(minimum, bytes: bytes);
      final upper = number(maximum, bytes: bytes);
      if (lower != null && upper != null && lower > upper) {
        errors[maximum] = LibraryFilterStrings.reversedRange;
      }
      return LibraryNumberRange(minimum: lower, maximum: upper);
    }

    String? date(LibraryFilterField field) {
      final value = text(field);
      if (value.isEmpty) return null;
      if (parseLibraryFilterDate(value) == null) {
        errors[field] = LibraryFilterStrings.invalidDate;
      }
      return value;
    }

    final extension = text(
      LibraryFilterField.extension,
    ).replaceFirst(RegExp(r"^\.+"), "").toLowerCase();
    if (!RegExp(r"^[a-z0-9]{0,16}$").hasMatch(extension)) {
      errors[LibraryFilterField.extension] =
          LibraryFilterStrings.invalidExtension;
    }
    final camera = text(LibraryFilterField.camera);
    if (camera.runes.length > 128 ||
        camera.runes.any((rune) => rune < 32 || (rune >= 127 && rune <= 159))) {
      errors[LibraryFilterField.camera] = LibraryFilterStrings.invalidCamera;
    }
    final start = date(LibraryFilterField.dateStart);
    final end = date(LibraryFilterField.dateEnd);
    if (!errors.containsKey(LibraryFilterField.dateStart) &&
        !errors.containsKey(LibraryFilterField.dateEnd) &&
        start != null &&
        end != null &&
        start.compareTo(end) > 0) {
      errors[LibraryFilterField.dateEnd] = LibraryFilterStrings.reversedDates;
    }
    final filter = LibraryMetadataFilter(
      extension: extension,
      bytes: range(
        LibraryFilterField.minimumBytes,
        LibraryFilterField.maximumBytes,
        bytes: true,
      ),
      width: range(
        LibraryFilterField.minimumWidth,
        LibraryFilterField.maximumWidth,
      ),
      height: range(
        LibraryFilterField.minimumHeight,
        LibraryFilterField.maximumHeight,
      ),
      date: LibraryDateRange(field: dateField, start: start, end: end),
      cameraText: camera,
    ).normalized;
    return (filter: errors.isEmpty ? filter : null, errors: errors);
  }
}

DateTime? parseLibraryFilterDate(String value) {
  if (!RegExp(r"^\d{4}-\d{2}-\d{2}$").hasMatch(value)) return null;
  final parts = value.split("-").map(int.parse).toList();
  final date = DateTime(parts[0], parts[1], parts[2]);
  return parts[0] > 0 &&
          date.year == parts[0] &&
          date.month == parts[1] &&
          date.day == parts[2]
      ? date
      : null;
}

String formatLibraryFilterDate(DateTime value) =>
    "${value.year.toString().padLeft(4, "0")}-"
    "${value.month.toString().padLeft(2, "0")}-"
    "${value.day.toString().padLeft(2, "0")}";
