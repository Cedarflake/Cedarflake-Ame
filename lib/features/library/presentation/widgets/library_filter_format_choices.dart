import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../library_filter_strings.dart";

class LibraryFilterFormatChoices extends StatelessWidget {
  const LibraryFilterFormatChoices({
    required this.selected,
    required this.onToggled,
    required this.customController,
    required this.showCustom,
    required this.onShowCustomChanged,
    this.error,
    super.key,
  });

  static const formats = ["jpg", "jpeg", "png", "webp", "gif", "avif", "heic"];

  final Set<String> selected;
  final ValueChanged<String> onToggled;
  final TextEditingController customController;
  final bool showCustom;
  final ValueChanged<bool> onShowCustomChanged;
  final String? error;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      AmeScrollMotion(
        scrollDirection: Axis.horizontal,
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Row(
            children: [
              for (final format in formats) ...[
                _format(format),
                const SizedBox(width: 6),
              ],
              ActionChip(
                avatar: Icon(
                  showCustom ? Symbols.remove_rounded : Symbols.add_rounded,
                  size: 18,
                ),
                label: const Text(LibraryFilterStrings.otherFormat),
                onPressed: () => onShowCustomChanged(!showCustom),
              ),
            ],
          ),
        ),
      ),
      if (showCustom) ...[
        const SizedBox(height: 12),
        TextFormField(
          key: const ValueKey("library-filter-extension"),
          controller: customController,
          textInputAction: TextInputAction.next,
          decoration: const InputDecoration(
            labelText: LibraryFilterStrings.extension,
            hintText: LibraryFilterStrings.extensionHint,
            border: OutlineInputBorder(),
          ),
          validator: (_) => error,
        ),
      ],
    ],
  );

  Widget _format(String format) => FilterChip(
    label: Text(format.toUpperCase()),
    selected: selected.contains(format),
    onSelected: (_) => onToggled(format),
  );
}
