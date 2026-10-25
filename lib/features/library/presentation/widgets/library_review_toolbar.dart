import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_organization_queue.dart";
import "../library_organization_mode.dart";

class LibraryReviewToolbar extends StatelessWidget {
  const LibraryReviewToolbar({required this.mode, this.counts, super.key});

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
          labels: LibraryOrganizationQueue.values.map(_label),
          leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
          items: [
            for (final queue in LibraryOrganizationQueue.values)
              ameMenuChoice(
                key: ValueKey("library-review-queue-${queue.name}"),
                value: queue,
                isSelected: queue == mode.queue,
                icon: _icon(queue),
                label: _label(queue),
              ),
          ],
          onSelected: mode.selectQueue,
          builder: (context, openMenu) => AmeTooltip(
            message: _label(mode.queue),
            child: TextButton.icon(
              key: const Key("library-review-queues"),
              onPressed: () {
                if (!mode.busy) openMenu();
              },
              icon: Icon(_icon(mode.queue), size: 20),
              label: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(_name(mode.queue), semanticsLabel: _label(mode.queue)),
                  const SizedBox(width: 4),
                  const Icon(Symbols.arrow_drop_down_rounded, size: 20),
                ],
              ),
            ),
          ),
        ),
        const SizedBox(width: 12),
        TextButton.icon(
          key: const Key("library-review-exit"),
          onPressed: mode.exit,
          icon: const Icon(Symbols.close_rounded, size: 20),
          label: const Text("退出整理"),
        ),
        if (mode.continuation?.canResume == true) ...[
          const SizedBox(width: 4),
          TextButton.icon(
            key: const Key("library-review-resume"),
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
