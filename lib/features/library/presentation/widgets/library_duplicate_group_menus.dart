import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_similar_review.dart";
import "../coordination/library_duplicate_workspace.dart";
import "../library_duplicate_strings.dart";

class LibraryDuplicateGroupMenus extends StatelessWidget {
  const LibraryDuplicateGroupMenus({
    required this.view,
    required this.degree,
    required this.onViewChanged,
    required this.onDegreeChanged,
    super.key,
  });
  final LibraryDuplicateView view;
  final LibrarySimilarityDegree degree;
  final ValueChanged<LibraryDuplicateView> onViewChanged;
  final ValueChanged<LibrarySimilarityDegree> onDegreeChanged;

  @override
  Widget build(BuildContext context) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      AmePopupMenuButton<LibraryDuplicateView>(
        labels: LibraryDuplicateView.values.map(_viewLabel),
        hasContentIcon: false,
        leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
        items: [
          for (final value in LibraryDuplicateView.values)
            ameMenuChoice(
              value: value,
              label: _viewLabel(value),
              isSelected: value == view,
            ),
        ],
        onSelected: onViewChanged,
        builder: (context, open) => AmeTooltip(
          message: "${LibraryDuplicateStrings.groupView}：${_viewLabel(view)}",
          child: IconButton(
            key: const Key("duplicate-group-menu"),
            onPressed: open,
            icon: const Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Symbols.layers_rounded),
                Icon(Symbols.arrow_drop_down_rounded, size: 18),
              ],
            ),
          ),
        ),
      ),
      if (view == LibraryDuplicateView.similar)
        AmePopupMenuButton<LibrarySimilarityDegree>(
          labels: LibrarySimilarityDegree.values.map(_degreeLabel),
          hasContentIcon: false,
          leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
          items: [
            for (final value in LibrarySimilarityDegree.values)
              ameMenuChoice(
                value: value,
                label: _degreeLabel(value),
                isSelected: value == degree,
              ),
          ],
          onSelected: onDegreeChanged,
          builder: (context, open) => AmeTooltip(
            message:
                "${LibraryDuplicateStrings.similarityDegree}：${_degreeLabel(degree)}",
            child: IconButton(
              key: const Key("duplicate-similarity-degree"),
              onPressed: open,
              icon: const Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Symbols.tune_rounded),
                  Icon(Symbols.arrow_drop_down_rounded, size: 18),
                ],
              ),
            ),
          ),
        ),
    ],
  );
}

String _viewLabel(LibraryDuplicateView value) => switch (value) {
  LibraryDuplicateView.exact => LibraryDuplicateStrings.pendingGroups,
  LibraryDuplicateView.ignored => LibraryDuplicateStrings.ignoredGroups,
  LibraryDuplicateView.similar => LibraryDuplicateStrings.similarGroups,
};
String _degreeLabel(LibrarySimilarityDegree value) => switch (value) {
  LibrarySimilarityDegree.strict => LibraryDuplicateStrings.strict,
  LibrarySimilarityDegree.standard => LibraryDuplicateStrings.standard,
  LibrarySimilarityDegree.broad => LibraryDuplicateStrings.broad,
};
