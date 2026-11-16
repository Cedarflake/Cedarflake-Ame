import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../../app/presentation/ame_selection_checkbox.dart";

class LibrarySidebarSectionHeader extends StatelessWidget {
  const LibrarySidebarSectionHeader({
    required this.title,
    this.count,
    this.trailing,
    this.selectedCount = 0,
    this.allSelected = false,
    this.onSelectAll,
    this.selectAllLabel,
    this.removeLabel,
    this.onRemove,
    super.key,
  });

  final String title;
  final int? count;
  final Widget? trailing;
  final int selectedCount;
  final bool allSelected;
  final VoidCallback? onSelectAll;
  final String? selectAllLabel;
  final String? removeLabel;
  final VoidCallback? onRemove;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 24, 4, 0),
    child: ConstrainedBox(
      constraints: const BoxConstraints(minHeight: 36),
      child: Row(
        children: [
          Expanded(
            child: Row(
              children: [
                Flexible(
                  child: Text(
                    selectedCount > 0 ? "已选择 $selectedCount" : title,
                    style: Theme.of(context).textTheme.labelLarge?.copyWith(
                      fontWeight: ameFontWeightMedium,
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
                if (selectedCount == 0 && count != null) ...[
                  const SizedBox(width: 4),
                  Text(
                    "($count)",
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ],
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
          if (trailing != null)
            SizedBox(width: 40, child: Center(child: trailing)),
        ],
      ),
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
