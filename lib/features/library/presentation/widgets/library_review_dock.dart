import "package:flutter/material.dart";

import "../../domain/library_organization_queue.dart";
import "../../domain/library_review.dart";
import "../library_organization_mode.dart";
import "library_review_dock_controls.dart";

class LibraryReviewDock extends StatelessWidget {
  const LibraryReviewDock({required this.mode, this.counts, super.key});
  final LibraryOrganizationMode mode;
  final LibraryOrganizationCounts? counts;

  @override
  Widget build(BuildContext context) {
    final status = switch (mode.continuation?.status) {
      LibraryContinuationStatus.anchorUnavailable => "上次整理的图片暂时不可用",
      LibraryContinuationStatus.scopeUnavailable => "上次整理的范围暂时不可用",
      LibraryContinuationStatus.endOfQueue => "已到上次整理位置之后的末尾",
      _ => null,
    };
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
          LibraryReviewDockControls(mode: mode, counts: counts),
        ],
      ),
    );
  }
}
