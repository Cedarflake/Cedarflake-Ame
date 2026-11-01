import "package:flutter/material.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../application/library_classification_configuration.dart";

List<PopupMenuEntry<String>> libraryClassificationCategoryItems({
  required List<LibraryClassificationCategory> categories,
  required String? selectedCategoryId,
}) => [
  for (final category in categories)
    ameMenuChoice(
      value: "category:${category.id}",
      isSelected: selectedCategoryId == category.id,
      label: category.label,
    ),
];

Future<String?> showLibraryClassificationCategoryMenu({
  required BuildContext context,
  required RelativeRect position,
  required List<LibraryClassificationCategory> categories,
  required String? selectedCategoryId,
}) => showAmePopupMenu<String>(
  context: context,
  position: position,
  labels: [for (final category in categories) category.label],
  hasContentIcon: false,
  leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
  items: libraryClassificationCategoryItems(
    categories: categories,
    selectedCategoryId: selectedCategoryId,
  ),
);
