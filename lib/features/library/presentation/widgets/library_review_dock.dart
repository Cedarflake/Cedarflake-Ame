import "package:flutter/material.dart";

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
    final continuation = mode.continuation;
    final status = switch (continuation?.status) {
      LibraryContinuationStatus.anchorUnavailable => "上次整理的图片暂时不可用",
      LibraryContinuationStatus.scopeUnavailable => "上次整理的范围暂时不可用",
      LibraryContinuationStatus.endOfQueue => "已到上次整理位置之后的末尾",
      _ => null,
    };
    const style = ButtonStyle(
      minimumSize: WidgetStatePropertyAll(Size(0, 48)),
      padding: WidgetStatePropertyAll(
        EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      ),
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
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Material(
                    elevation: 3,
                    color: Theme.of(context).colorScheme.surfaceContainer,
                    borderRadius: BorderRadius.circular(28),
                    child: SegmentedButton<LibraryOrganizationQueue>(
                      style: style,
                      selected: {mode.queue},
                      onSelectionChanged: mode.busy
                          ? null
                          : (selection) => mode.selectQueue(selection.single),
                      segments: [
                        for (final queue in LibraryOrganizationQueue.values)
                          ButtonSegment(
                            value: queue,
                            label: Text(_label(queue)),
                          ),
                      ],
                    ),
                  ),
                  if (continuation?.canResume == true) ...[
                    const SizedBox(width: 12),
                    FilledButton(
                      key: const Key("library-review-resume"),
                      style: style,
                      onPressed: mode.busy ? null : mode.resume,
                      child: const Text("继续整理"),
                    ),
                  ],
                ],
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
