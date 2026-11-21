import "../../../../app/presentation/ame_localizations.dart";
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

  List<LibraryMetadataFilterCondition> conditions(AppLocalizations strings) => [
    for (final extension in filter.extensions)
      (
        label: strings.filterFormat(extension.toUpperCase()),
        remaining: filter.copyWith(
          extensions: filter.extensions
              .where((value) => value != extension)
              .toList(),
        ),
      ),
    ..._otherConditions(strings),
  ];

  String? _dateLabel(AppLocalizations strings, String? key) {
    if (key == null) return null;
    final date = parseLibraryFilterDate(key);
    return date == null ? key : strings.galleryDay(date);
  }

  List<LibraryMetadataFilterCondition> _otherConditions(
    AppLocalizations strings,
  ) => [
    if (filter.date.start != null || filter.date.end != null)
      (
        label: strings.labelAndValue(
          LibraryFilterStrings.dateFieldName(strings, filter.date.field),
          _range(
            _dateLabel(strings, filter.date.start),
            _dateLabel(strings, filter.date.end),
          ),
        ),
        remaining: filter.copyWith(date: const LibraryDateRange()),
      ),
    if (_hasBounds(filter.bytes))
      (
        label: _bytesLabel(strings),
        remaining: filter.copyWith(bytes: const LibraryNumberRange()),
      ),
    if (_hasBounds(filter.width))
      (
        label: strings.filterWidth(_numberRange(filter.width)),
        remaining: filter.copyWith(width: const LibraryNumberRange()),
      ),
    if (_hasBounds(filter.height))
      (
        label: strings.filterHeight(_numberRange(filter.height)),
        remaining: filter.copyWith(height: const LibraryNumberRange()),
      ),
    if (filter.cameraText.isNotEmpty)
      (
        label: strings.filterCamera(filter.cameraText),
        remaining: filter.copyWith(cameraText: ""),
      ),
  ];

  String description(AppLocalizations strings) => [
    if (filter.extensions.isNotEmpty)
      strings.filterFormat(
        filter.extensions
            .map((value) => value.toUpperCase())
            .join(strings.inlineListSeparator),
      ),
    for (final condition in _otherConditions(strings)) condition.label,
  ].join("\n");

  String _bytesLabel(AppLocalizations strings) {
    final unit = LibraryFilterSizeUnit.forRange(filter.bytes);
    String? value(int? bytes) =>
        bytes == null ? null : (bytes ~/ unit.multiplier).toString();
    return strings.filterFileSize(
      _range(value(filter.bytes.minimum), value(filter.bytes.maximum)),
      unit.label,
    );
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
