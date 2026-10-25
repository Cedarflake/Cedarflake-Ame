import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../domain/library_organization_queue.dart";
import "../library_organization_mode.dart";

class LibraryReviewDockControls extends StatelessWidget {
  const LibraryReviewDockControls({required this.mode, this.counts, super.key});

  final LibraryOrganizationMode mode;
  final LibraryOrganizationCounts? counts;

  static const _buttonStyle = ButtonStyle(
    minimumSize: WidgetStatePropertyAll(Size(40, 40)),
    padding: WidgetStatePropertyAll(EdgeInsets.all(10)),
    elevation: WidgetStatePropertyAll(3),
    shape: WidgetStatePropertyAll(StadiumBorder()),
    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
  );

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    // The framework blocks pointer and semantics actions without repainting the
    // controls as disabled. The mode also rejects busy keyboard commands.
    return AbsorbPointer(
      absorbing: mode.busy,
      child: AmeScrollMotion(
        scrollDirection: Axis.horizontal,
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: IntrinsicHeight(
            child: Row(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.center,
              children: [
                AmeTooltip(
                  message: "退出整理",
                  child: FilledButton.tonal(
                    key: const Key("library-review-exit"),
                    style: _buttonStyle,
                    onPressed: mode.exit,
                    child: const Icon(
                      Symbols.close_rounded,
                      size: 20,
                      semanticLabel: "退出整理",
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Material(
                  key: const Key("library-review-queues"),
                  elevation: 3,
                  color: colors.surfaceContainer,
                  borderRadius: BorderRadius.circular(28),
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(minHeight: 52),
                    child: Padding(
                      padding: const EdgeInsets.all(7),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        spacing: 4,
                        children: [
                          for (final queue in LibraryOrganizationQueue.values)
                            AmeTooltip(
                              message: _label(queue),
                              child: ChoiceChip(
                                key: ValueKey(
                                  "library-review-queue-${queue.name}",
                                ),
                                label: Icon(
                                  switch (queue) {
                                    LibraryOrganizationQueue.pending =>
                                      Symbols.inbox_rounded,
                                    LibraryOrganizationQueue.deferred =>
                                      Symbols.schedule_rounded,
                                    LibraryOrganizationQueue.ignored =>
                                      Symbols.hide_image_rounded,
                                  },
                                  size: 20,
                                  semanticLabel: _label(queue),
                                ),
                                selected: mode.queue == queue,
                                showCheckmark: false,
                                shape: const StadiumBorder(),
                                side: BorderSide.none,
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 15,
                                  vertical: 7,
                                ),
                                labelPadding: EdgeInsets.zero,
                                materialTapTargetSize:
                                    MaterialTapTargetSize.shrinkWrap,
                                backgroundColor: colors.surfaceContainer,
                                selectedColor: colors.secondaryContainer,
                                onSelected: (selected) {
                                  if (selected) mode.selectQueue(queue);
                                },
                              ),
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
                if (mode.continuation?.canResume == true) ...[
                  const SizedBox(width: 12),
                  AmeTooltip(
                    message: "继续整理",
                    child: FilledButton.tonal(
                      key: const Key("library-review-resume"),
                      style: _buttonStyle,
                      onPressed: mode.resume,
                      child: const Icon(
                        Symbols.arrow_forward_rounded,
                        size: 20,
                        semanticLabel: "继续整理",
                      ),
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }

  String _label(LibraryOrganizationQueue queue) {
    final label = switch (queue) {
      LibraryOrganizationQueue.pending => "待整理",
      LibraryOrganizationQueue.deferred => "稍后整理",
      LibraryOrganizationQueue.ignored => "已忽略",
    };
    return counts == null ? label : "$label ${counts!.countFor(queue)}";
  }
}
