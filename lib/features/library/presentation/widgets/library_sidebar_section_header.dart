import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";

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
    padding: EdgeInsets.fromLTRB(selectedCount > 0 ? 8 : 16, 8, 4, 0),
    child: Row(
      children: [
        if (selectedCount > 0)
          SizedBox.square(
            dimension: 40,
            child: Checkbox(
              key: const Key("library-sidebar-select-all"),
              value: allSelected ? true : null,
              tristate: true,
              semanticLabel: selectAllLabel ?? "全选$title",
              onChanged: onSelectAll == null ? null : (_) => onSelectAll!(),
            ),
          ),
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
            message: removeLabel ?? "删除",
            child: IconButton(
              key: const Key("library-sidebar-remove-selected"),
              onPressed: onRemove,
              icon: const Icon(Symbols.delete_rounded),
            ),
          ),
        AmeTooltip(
          message: createLabel,
          child: IconButton(
            key: createKey,
            onPressed: onCreate,
            icon: const Icon(Symbols.add_rounded),
          ),
        ),
      ],
    ),
  );
}
