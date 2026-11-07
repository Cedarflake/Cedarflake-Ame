import "package:flutter/material.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_metadata_filter.dart";
import "../coordination/library_metadata_filter_draft.dart";
import "../library_filter_strings.dart";

class LibraryMetadataFilterSummary extends StatelessWidget {
  const LibraryMetadataFilterSummary({
    required this.filter,
    required this.onChanged,
    super.key,
  });

  final LibraryMetadataFilter filter;
  final ValueChanged<LibraryMetadataFilter> onChanged;

  @override
  Widget build(BuildContext context) {
    if (!filter.isActive) return const SizedBox.shrink();
    return Padding(
      key: const Key("library-filter-summary"),
      padding: const EdgeInsets.fromLTRB(28, 0, 20, 12),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Wrap(
          spacing: 8,
          runSpacing: 4,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            if (filter.date.start != null || filter.date.end != null)
              _chip(
                "${LibraryFilterStrings.dateFieldName(filter.date.field)}：${_range(filter.date.start, filter.date.end)}",
                filter.copyWith(date: const LibraryDateRange()),
              ),
            if (filter.extension.isNotEmpty)
              _chip("扩展名：${filter.extension}", filter.copyWith(extension: "")),
            if (_hasBounds(filter.bytes))
              _chip(
                _bytesLabel(),
                filter.copyWith(bytes: const LibraryNumberRange()),
              ),
            if (_hasBounds(filter.width))
              _chip(
                "宽度：${_numberRange(filter.width)} px",
                filter.copyWith(width: const LibraryNumberRange()),
              ),
            if (_hasBounds(filter.height))
              _chip(
                "高度：${_numberRange(filter.height)} px",
                filter.copyWith(height: const LibraryNumberRange()),
              ),
            if (filter.cameraText.isNotEmpty)
              _chip("相机：${filter.cameraText}", filter.copyWith(cameraText: "")),
            TextButton(
              onPressed: () => onChanged(const LibraryMetadataFilter()),
              child: const Text(LibraryFilterStrings.clear),
            ),
          ],
        ),
      ),
    );
  }

  Widget _chip(String label, LibraryMetadataFilter remaining) =>
      AmeOverlayTraversalBoundary(
        child: InputChip(
          label: Text(label, overflow: TextOverflow.ellipsis),
          deleteButtonTooltipMessage: "移除$label",
          onDeleted: () => onChanged(remaining.normalized),
        ),
      );

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
