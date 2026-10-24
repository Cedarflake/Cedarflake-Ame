import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../domain/library_organization_queue.dart";
import "../../domain/library_review.dart";
import "../library_organization_mode.dart";

class LibraryReviewDock extends StatelessWidget {
  const LibraryReviewDock({required this.mode, this.counts, super.key});
  final LibraryOrganizationMode mode;
  final LibraryOrganizationCounts? counts;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final continuation = mode.continuation;
    final status = switch (continuation?.status) {
      LibraryContinuationStatus.anchorUnavailable => "上次整理的图片暂时不可用",
      LibraryContinuationStatus.scopeUnavailable => "上次整理的范围暂时不可用",
      LibraryContinuationStatus.endOfQueue => "已到上次整理位置之后的末尾",
      _ => null,
    };
    const style = ButtonStyle(
      minimumSize: WidgetStatePropertyAll(Size(0, 52)),
      padding: WidgetStatePropertyAll(
        EdgeInsets.symmetric(horizontal: 18, vertical: 12),
      ),
      elevation: WidgetStatePropertyAll(3),
      shape: WidgetStatePropertyAll(StadiumBorder()),
      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
    );
    return Padding(
      key: const Key("library-review-dock"),
      padding: const EdgeInsets.fromLTRB(24, 8, 24, 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (mode.problem case final problem?)
            TextButton(
              onPressed: mode.busy ? null : mode.retry,
              child: Text(problem),
            )
          else if (status != null)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Text(status),
            ),
          AmeScrollMotion(
            scrollDirection: Axis.horizontal,
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(vertical: 8),
              child: IntrinsicHeight(
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
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
                              for (final queue
                                  in LibraryOrganizationQueue.values)
                                ChoiceChip(
                                  avatar: Icon(switch (queue) {
                                    LibraryOrganizationQueue.pending =>
                                      Symbols.inbox_rounded,
                                    LibraryOrganizationQueue.deferred =>
                                      Symbols.schedule_rounded,
                                    LibraryOrganizationQueue.ignored =>
                                      Symbols.hide_image_rounded,
                                  }, size: 20),
                                  label: Text(_label(queue)),
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
                                  onSelected: mode.busy
                                      ? null
                                      : (selected) {
                                          if (selected) mode.selectQueue(queue);
                                        },
                                ),
                            ],
                          ),
                        ),
                      ),
                    ),
                    if (continuation?.canResume == true) ...[
                      const SizedBox(width: 12),
                      FilledButton.tonalIcon(
                        key: const Key("library-review-resume"),
                        style: style,
                        onPressed: mode.busy ? null : mode.resume,
                        icon: const Icon(
                          Symbols.arrow_forward_rounded,
                          size: 20,
                        ),
                        label: const Text("继续整理"),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        ],
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
