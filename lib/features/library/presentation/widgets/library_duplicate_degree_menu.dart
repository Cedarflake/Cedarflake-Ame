import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_similar_review.dart";
import "../library_duplicate_strings.dart";

class LibraryDuplicateDegreeMenu extends StatelessWidget {
  const LibraryDuplicateDegreeMenu({
    required this.degree,
    required this.onChanged,
    super.key,
  });

  final LibrarySimilarityDegree degree;
  final ValueChanged<LibrarySimilarityDegree> onChanged;

  @override
  Widget build(BuildContext context) =>
      AmePopupMenuButton<LibrarySimilarityDegree>(
        labels: LibrarySimilarityDegree.values.map(_label),
        hasContentIcon: false,
        leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
        items: [
          for (final value in LibrarySimilarityDegree.values)
            ameMenuChoice(
              value: value,
              label: _label(value),
              isSelected: value == degree,
            ),
        ],
        onSelected: onChanged,
        builder: (context, open) => AmeTooltip(
          message:
              "${LibraryDuplicateStrings.similarityDegree}：${_label(degree)}",
          child: TextButton.icon(
            key: const Key("duplicate-similarity-degree"),
            onPressed: open,
            icon: const Icon(Symbols.tune_rounded),
            label: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(_label(degree)),
                const SizedBox(width: 4),
                const Icon(Symbols.arrow_drop_down_rounded, size: 18),
              ],
            ),
          ),
        ),
      );
}

String _label(LibrarySimilarityDegree value) => switch (value) {
  LibrarySimilarityDegree.strict => LibraryDuplicateStrings.strict,
  LibrarySimilarityDegree.standard => LibraryDuplicateStrings.standard,
  LibrarySimilarityDegree.broad => LibraryDuplicateStrings.broad,
};
