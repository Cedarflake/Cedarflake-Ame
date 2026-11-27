import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../../app/presentation/ame_selection_checkbox.dart";

class LibrarySidebarSectionHeader extends StatelessWidget {
  const LibrarySidebarSectionHeader({
    required this.title,
    this.count,
    this.trailing,
    this.action,
    this.createLabel,
    this.createKey,
    this.onCreate,
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
  final Widget? action;
  final String? createLabel;
  final Key? createKey;
  final VoidCallback? onCreate;
  final int selectedCount;
  final bool allSelected;
  final VoidCallback? onSelectAll;
  final String? selectAllLabel;
  final String? removeLabel;
  final VoidCallback? onRemove;

  @override
  Widget build(BuildContext context) {
    final labelStyle = Theme.of(context).textTheme.labelLarge?.copyWith(
      fontWeight: ameFontWeightMedium,
      color: Theme.of(context).colorScheme.onSurfaceVariant,
    );
    final heading = Semantics(
      label: selectedCount > 0
          ? context.l10n.selectedCount(selectedCount)
          : null,
      excludeSemantics: selectedCount > 0,
      child: Row(
        children: [
          Flexible(
            child: selectedCount > 0
                ? LayoutBuilder(
                    builder: (context, constraints) {
                      final measure = TextPainter(
                        text: TextSpan(
                          text: context.l10n.selectedLong,
                          style: labelStyle,
                        ),
                        textDirection: Directionality.of(context),
                        textScaler: MediaQuery.textScalerOf(context),
                      )..layout();
                      final compact = measure.width > constraints.maxWidth;
                      measure.dispose();
                      return Text(
                        compact
                            ? context.l10n.selectedShort
                            : context.l10n.selectedLong,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: labelStyle,
                      );
                    },
                  )
                : Text(
                    title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: labelStyle,
                  ),
          ),
          if (selectedCount > 0 || count != null) ...[
            const SizedBox(width: 4),
            Text(
              "(${selectedCount > 0 ? selectedCount : count})",
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
    );
    final actions = Row(
      mainAxisSize: MainAxisSize.min,
      mainAxisAlignment: MainAxisAlignment.end,
      children: [
        if (selectedCount > 0)
          AmeTooltip(
            message: allSelected
                ? context.l10n.deselectAll
                : selectAllLabel ?? context.l10n.selectAllNamed(title),
            child: AmeSelectionCheckbox(
              key: const Key("library-sidebar-select-all"),
              markExtent: 18,
              tapTargetSize: 36,
              value: allSelected ? true : null,
              tristate: true,
              isOutlined: true,
              semanticLabel:
                  selectAllLabel ?? context.l10n.selectAllNamed(title),
              onChanged: onSelectAll == null ? null : (_) => onSelectAll!(),
            ),
          ),
        if (selectedCount > 0)
          AmeTooltip(
            message: removeLabel ?? context.l10n.deleteAction,
            child: IconButton(
              key: const Key("library-sidebar-remove-selected"),
              onPressed: onRemove,
              style: _actionStyle,
              icon: const Icon(Symbols.delete_rounded),
            ),
          ),
        if (selectedCount == 0 && createLabel != null)
          SizedBox(
            width: 40,
            child: Center(
              child: AmeTooltip(
                message: createLabel!,
                child: IconButton(
                  key: createKey,
                  onPressed: onCreate,
                  style: _actionStyle,
                  icon: const Icon(Symbols.add_rounded),
                ),
              ),
            ),
          ),
        if (action != null) SizedBox(width: 40, child: Center(child: action)),
        if (trailing != null)
          SizedBox(width: 40, child: Center(child: trailing)),
      ],
    );
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 24, 4, 0),
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: Row(
          children: [
            Expanded(child: heading),
            actions,
          ],
        ),
      ),
    );
  }

  static final _actionStyle = IconButton.styleFrom(
    iconSize: 20,
    minimumSize: const Size.square(36),
    maximumSize: const Size.square(36),
    padding: const EdgeInsets.all(8),
    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
  );
}
