import "package:flutter/material.dart";

import "../../domain/library_organization_queue.dart";
import "../library_organization_mode.dart";
import "library_review_dock_controls.dart";

class LibraryReviewDock extends StatelessWidget {
  const LibraryReviewDock({required this.mode, this.counts, super.key});
  final LibraryOrganizationMode mode;
  final LibraryOrganizationCounts? counts;

  @override
  Widget build(BuildContext context) {
    return Padding(
      key: const Key("library-review-dock"),
      padding: const EdgeInsets.fromLTRB(24, 0, 24, 8),
      child: Align(
        heightFactor: 1,
        child: LibraryReviewDockControls(mode: mode, counts: counts),
      ),
    );
  }
}
