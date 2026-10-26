import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_organization_queue.dart";
import "../library_organization_mode.dart";

class LibraryReviewToolbar extends StatelessWidget {
  const LibraryReviewToolbar({required this.mode, this.counts, super.key});

  static const _actionStyle = ButtonStyle(
    padding: WidgetStatePropertyAll(EdgeInsets.all(8)),
  );

  final LibraryOrganizationMode mode;
  final LibraryOrganizationCounts? counts;

  @override
  Widget build(BuildContext context) => AbsorbPointer(
    absorbing: mode.busy,
    child: Row(
      key: const Key("library-review-toolbar"),
      mainAxisSize: MainAxisSize.min,
      children: [
        AmePopupMenuButton<LibraryOrganizationQueue>(
          labels: LibraryOrganizationQueue.values.map(_name),
          leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
          items: [
            for (final queue in LibraryOrganizationQueue.values)
              ameMenuChoice(
                key: ValueKey("library-review-queue-${queue.name}"),
                value: queue,
                isSelected: queue == mode.queue,
                icon: _icon(queue),
                label: _name(queue),
              ),
          ],
          onSelected: mode.selectQueue,
          builder: (context, openMenu) => AmeTooltip(
            message: _label(mode.queue),
            child: IconButton(
              key: const Key("library-review-queues"),
              color: Theme.of(context).colorScheme.primary,
              onPressed: () {
                if (!mode.busy) openMenu();
              },
              icon: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    _icon(mode.queue),
                    size: 20,
                    semanticLabel: _label(mode.queue),
                  ),
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
          label: const Text("退出整理"),
        ),
        if (mode.continuation?.canResume == true) ...[
          const SizedBox(width: 4),
          TextButton.icon(
            key: const Key("library-review-resume"),
            style: _actionStyle,
            onPressed: mode.resume,
            icon: const Icon(Symbols.arrow_forward_rounded, size: 20),
            label: const Text("继续整理"),
          ),
        ],
      ],
    ),
  );

  static String _name(LibraryOrganizationQueue queue) => switch (queue) {
    LibraryOrganizationQueue.pending => "待整理",
    LibraryOrganizationQueue.deferred => "稍后整理",
    LibraryOrganizationQueue.ignored => "已忽略",
  };

  static IconData _icon(LibraryOrganizationQueue queue) => switch (queue) {
    LibraryOrganizationQueue.pending => Symbols.inbox_rounded,
    LibraryOrganizationQueue.deferred => Symbols.schedule_rounded,
    LibraryOrganizationQueue.ignored => Symbols.hide_image_rounded,
  };

  String _label(LibraryOrganizationQueue queue) {
    final label = _name(queue);
    return counts == null ? label : "$label ${counts!.countFor(queue)}";
  }
}
