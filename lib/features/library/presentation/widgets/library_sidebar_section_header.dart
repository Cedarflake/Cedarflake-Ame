import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../../app/presentation/ame_selection_checkbox.dart";

class LibrarySidebarSectionHeader extends StatelessWidget {
  const LibrarySidebarSectionHeader({
    required this.title,
    required this.createLabel,
    required this.createKey,
    required this.onCreate,
    this.selectedCount = 0,
    this.allSelected = false,
    this.onSelectAll,
    this.selectAllLabel,
    this.removeLabel,
    this.onRemove,
    super.key,
  });

  final String title;
  final String createLabel;
  final Key createKey;
  final VoidCallback? onCreate;
  final int selectedCount;
  final bool allSelected;
  final VoidCallback? onSelectAll;
  final String? selectAllLabel;
  final String? removeLabel;
  final VoidCallback? onRemove;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 8, 6, 0),
    child: Row(
      children: [
        Expanded(
          child: Text(
            selectedCount > 0 ? "已选择 $selectedCount" : title,
            style: Theme.of(
              context,
            ).textTheme.titleSmall?.copyWith(fontWeight: ameFontWeightSemibold),
          ),
        ),
        if (selectedCount > 0)
          AmeTooltip(
            message: allSelected ? "取消全选" : selectAllLabel ?? "全选$title",
            child: AmeSelectionCheckbox(
              key: const Key("library-sidebar-select-all"),
              markExtent: 18,
              tapTargetSize: 36,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(2),
              ),
              value: allSelected ? true : null,
              tristate: true,
              isOutlined: true,
              semanticLabel: selectAllLabel ?? "全选$title",
              onChanged: onSelectAll == null ? null : (_) => onSelectAll!(),
            ),
          ),
        if (selectedCount > 0)
          AmeTooltip(
            message: removeLabel ?? "删除",
            child: IconButton(
              key: const Key("library-sidebar-remove-selected"),
              onPressed: onRemove,
              style: _actionStyle,
              icon: const Icon(Symbols.delete_rounded),
            ),
          ),
        AmeTooltip(
          message: createLabel,
          child: IconButton(
            key: createKey,
            onPressed: onCreate,
            style: _actionStyle,
            icon: const Icon(Symbols.add_rounded),
          ),
        ),
      ],
    ),
  );

  static final _actionStyle = IconButton.styleFrom(
    iconSize: 20,
    minimumSize: const Size.square(36),
    maximumSize: const Size.square(36),
    padding: const EdgeInsets.all(8),
    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
  );
}
