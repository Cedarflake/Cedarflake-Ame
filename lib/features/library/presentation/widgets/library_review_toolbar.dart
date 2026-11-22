import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_organization_queue.dart";
import "../coordination/library_organization_mode.dart";

class LibraryReviewToolbar extends StatelessWidget {
  const LibraryReviewToolbar({required this.mode, this.counts, super.key});

  static const _actionStyle = ButtonStyle(
    padding: WidgetStatePropertyAll(EdgeInsets.all(8)),
  );

  final LibraryOrganizationMode mode;
  final LibraryOrganizationCounts? counts;

  @override
  Widget build(BuildContext context) => Row(
    key: const Key("library-review-toolbar"),
    mainAxisSize: MainAxisSize.min,
    children: [
      AmePopupMenuButton<LibraryOrganizationQueue>(
        labels: LibraryOrganizationQueue.values.map(
          (value) => _name(context.l10n, value),
        ),
        leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
        items: [
          for (final queue in LibraryOrganizationQueue.values)
            ameMenuChoice(
              key: ValueKey("library-review-queue-${queue.name}"),
              value: queue,
              isSelected: queue == mode.queue,
              icon: _icon(queue),
              label: _name(context.l10n, queue),
            ),
        ],
        onSelected: mode.selectQueue,
        builder: (context, openMenu) => AmeTooltip(
          message: _label(context.l10n, mode.queue),
          child: TextButton.icon(
            key: const Key("library-review-queues"),
            style: _actionStyle,
            onPressed: mode.busy ? null : openMenu,
            icon: Icon(_icon(mode.queue), size: 20),
            label: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  _name(context.l10n, mode.queue),
                  semanticsLabel: _label(context.l10n, mode.queue),
                ),
                const SizedBox(width: 4),
                const Icon(Symbols.arrow_drop_down_rounded, size: 18),
              ],
            ),
          ),
        ),
      ),
      const SizedBox(width: 4),
      TextButton.icon(
        key: const Key("library-review-exit"),
        style: _actionStyle,
        onPressed: mode.exit,
        icon: const Icon(Symbols.close_rounded, size: 20),
        label: Text(context.l10n.exitOrganization),
      ),
      if (mode.continuation?.canResume == true) ...[
        const SizedBox(width: 4),
        TextButton.icon(
          key: const Key("library-review-resume"),
          style: _actionStyle,
          onPressed: mode.busy ? null : mode.resume,
          icon: const Icon(Symbols.arrow_forward_rounded, size: 20),
          label: Text(context.l10n.continueOrganizing),
        ),
      ],
    ],
  );

  static String _name(
    AppLocalizations strings,
    LibraryOrganizationQueue queue,
  ) => switch (queue) {
    LibraryOrganizationQueue.pending => strings.toOrganize,
    LibraryOrganizationQueue.deferred => strings.organizeLater,
    LibraryOrganizationQueue.ignored => strings.ignored,
  };

  static IconData _icon(LibraryOrganizationQueue queue) => switch (queue) {
    LibraryOrganizationQueue.pending => Symbols.inbox_rounded,
    LibraryOrganizationQueue.deferred => Symbols.schedule_rounded,
    LibraryOrganizationQueue.ignored => Symbols.hide_image_rounded,
  };

  String _label(AppLocalizations strings, LibraryOrganizationQueue queue) {
    final label = _name(strings, queue);
    return counts == null ? label : "$label ${counts!.countFor(queue)}";
  }
}
