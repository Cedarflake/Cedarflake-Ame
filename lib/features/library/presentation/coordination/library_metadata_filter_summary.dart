import "../../domain/library_metadata_filter.dart";
import "../library_filter_strings.dart";
import "library_metadata_filter_draft.dart";

typedef LibraryMetadataFilterCondition = ({
  String label,
  LibraryMetadataFilter remaining,
});

class LibraryMetadataFilterSummary {
  const LibraryMetadataFilterSummary(this.filter);

  final LibraryMetadataFilter filter;

  List<LibraryMetadataFilterCondition> get conditions => [
    for (final extension in filter.extensions)
      (
        label: "格式：${extension.toUpperCase()}",
        remaining: filter.copyWith(
          extensions: filter.extensions
              .where((value) => value != extension)
              .toList(),
        ),
      ),
    ..._otherConditions,
  ];

  List<LibraryMetadataFilterCondition> get _otherConditions => [
    if (filter.date.start != null || filter.date.end != null)
      (
        label:
            "${LibraryFilterStrings.dateFieldName(filter.date.field)}：${_range(filter.date.start, filter.date.end)}",
        remaining: filter.copyWith(date: const LibraryDateRange()),
      ),
    if (_hasBounds(filter.bytes))
      (
        label: _bytesLabel(),
        remaining: filter.copyWith(bytes: const LibraryNumberRange()),
      ),
    if (_hasBounds(filter.width))
      (
        label: "宽度：${_numberRange(filter.width)} px",
        remaining: filter.copyWith(width: const LibraryNumberRange()),
      ),
    if (_hasBounds(filter.height))
      (
        label: "高度：${_numberRange(filter.height)} px",
        remaining: filter.copyWith(height: const LibraryNumberRange()),
      ),
    if (filter.cameraText.isNotEmpty)
      (
        label: "相机：${filter.cameraText}",
        remaining: filter.copyWith(cameraText: ""),
      ),
  ];

  String get description => [
    if (filter.extensions.isNotEmpty)
      "格式：${filter.extensions.map((value) => value.toUpperCase()).join("、")}",
    for (final condition in _otherConditions) condition.label,
  ].join("\n");

  String _bytesLabel() {
    final unit = LibraryFilterSizeUnit.forRange(filter.bytes);
    String? value(int? bytes) =>
        bytes == null ? null : (bytes ~/ unit.multiplier).toString();
    return "文件大小：${_range(value(filter.bytes.minimum), value(filter.bytes.maximum))} ${unit.label}";
  }

  static bool _hasBounds(LibraryNumberRange range) =>
      range.minimum != null || range.maximum != null;

  static String _numberRange(LibraryNumberRange range) =>
      _range(range.minimum?.toString(), range.maximum?.toString());

  static String _range(String? minimum, String? maximum) =>
      switch ((minimum, maximum)) {
        (final lower?, final upper?) when lower == upper => lower,
        (final lower?, final upper?) => "$lower – $upper",
        (final lower?, null) => "≥ $lower",
        (null, final upper?) => "≤ $upper",
        _ => "",
      };
}
