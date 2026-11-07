enum LibraryDateField { capture, created, modified }

class LibraryNumberRange {
  const LibraryNumberRange({this.minimum, this.maximum});

  final int? minimum;
  final int? maximum;

  @override
  int get hashCode => Object.hash(minimum, maximum);

  @override
  bool operator ==(Object other) =>
      other is LibraryNumberRange &&
      minimum == other.minimum &&
      maximum == other.maximum;
}

class LibraryDateRange {
  const LibraryDateRange({
    this.field = LibraryDateField.capture,
    this.start,
    this.end,
  });

  final LibraryDateField field;
  final String? start;
  final String? end;

  @override
  int get hashCode => Object.hash(field, start, end);

  @override
  bool operator ==(Object other) =>
      other is LibraryDateRange &&
      field == other.field &&
      start == other.start &&
      end == other.end;
}

class LibraryMetadataFilter {
  const LibraryMetadataFilter({
    this.extensions = const [],
    this.bytes = const LibraryNumberRange(),
    this.width = const LibraryNumberRange(),
    this.height = const LibraryNumberRange(),
    this.date = const LibraryDateRange(),
    this.cameraText = "",
  });

  final List<String> extensions;
  final LibraryNumberRange bytes;
  final LibraryNumberRange width;
  final LibraryNumberRange height;
  final LibraryDateRange date;
  final String cameraText;

  LibraryMetadataFilter copyWith({
    List<String>? extensions,
    LibraryNumberRange? bytes,
    LibraryNumberRange? width,
    LibraryNumberRange? height,
    LibraryDateRange? date,
    String? cameraText,
  }) => LibraryMetadataFilter(
    extensions: extensions ?? this.extensions,
    bytes: bytes ?? this.bytes,
    width: width ?? this.width,
    height: height ?? this.height,
    date: date ?? this.date,
    cameraText: cameraText ?? this.cameraText,
  );

  LibraryMetadataFilter get normalized => LibraryMetadataFilter(
    extensions: List.unmodifiable(
      extensions
          .map(
            (value) =>
                value.trim().replaceFirst(RegExp(r"^\.+"), "").toLowerCase(),
          )
          .where((value) => value.isNotEmpty)
          .toSet()
          .toList()
        ..sort(),
    ),
    bytes: bytes,
    width: width,
    height: height,
    date: date.start == null && date.end == null
        ? const LibraryDateRange()
        : date,
    cameraText: cameraText.trim(),
  );

  bool get isActive => normalized != const LibraryMetadataFilter();

  @override
  int get hashCode => Object.hash(
    Object.hashAll(extensions),
    bytes,
    width,
    height,
    date,
    cameraText,
  );

  @override
  bool operator ==(Object other) =>
      other is LibraryMetadataFilter &&
      extensions.length == other.extensions.length &&
      extensions.asMap().entries.every(
        (entry) => entry.value == other.extensions[entry.key],
      ) &&
      bytes == other.bytes &&
      width == other.width &&
      height == other.height &&
      date == other.date &&
      cameraText == other.cameraText;
}
