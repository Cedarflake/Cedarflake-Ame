import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "ame_menu.dart";

PopupMenuItem<T> ameMenuChoice<T>({
  required T value,
  required String label,
  IconData? icon,
  required bool isSelected,
  bool isToggle = false,
  double indicatorWidth = AmeMenuMetrics.selectionIndicatorSlotWidth,
  bool enabled = true,
  Key? key,
}) => PopupMenuItem<T>(
  key: key,
  value: value,
  enabled: enabled,
  child: Semantics(
    key: ValueKey("menu-choice-$label"),
    checked: isSelected,
    inMutuallyExclusiveGroup: !isToggle,
    child: Row(
      children: [
        SizedBox(
          width: indicatorWidth,
          child: isSelected
              ? ExcludeSemantics(
                  child: Icon(
                    isToggle ? Symbols.check_rounded : Symbols.circle_rounded,
                    size: isToggle ? 18 : AmeMenuMetrics.selectionIndicatorSize,
                    fill: isToggle ? 0 : 1,
                  ),
                )
              : null,
        ),
        const SizedBox(width: AmeMenuMetrics.iconLabelGap),
        Expanded(
          child: icon == null
              ? Text(label)
              : AmeMenuItemContent(
                  icon: icon,
                  label: label,
                  isSelected: isSelected,
                ),
        ),
      ],
    ),
  ),
);
