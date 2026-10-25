import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "ame_menu.dart";

PopupMenuItem<T> ameMenuChoice<T>({
  required T value,
  required String label,
  required IconData icon,
  required bool isSelected,
  Key? key,
}) => PopupMenuItem<T>(
  key: key,
  value: value,
  child: Semantics(
    key: ValueKey("menu-choice-$label"),
    checked: isSelected,
    inMutuallyExclusiveGroup: true,
    child: Row(
      children: [
        SizedBox(
          width: AmeMenuMetrics.selectionIndicatorSlotWidth,
          child: isSelected
              ? const ExcludeSemantics(
                  child: Icon(
                    Symbols.circle_rounded,
                    size: AmeMenuMetrics.selectionIndicatorSize,
                    fill: 1,
                  ),
                )
              : null,
        ),
        const SizedBox(width: AmeMenuMetrics.iconLabelGap),
        Expanded(
          child: AmeMenuItemContent(
            icon: icon,
            label: label,
            isSelected: isSelected,
          ),
        ),
      ],
    ),
  ),
);
