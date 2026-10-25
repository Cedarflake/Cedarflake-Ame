import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../library_organization_mode.dart";
import "library_viewer_notice.dart";

class LibraryReviewModeNotice extends StatelessWidget {
  const LibraryReviewModeNotice({
    required this.mode,
    this.enabled = true,
    super.key,
  });

  final LibraryOrganizationMode mode;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final message = mode.problem;
    if (message == null) return const SizedBox.shrink();
    return LibraryViewerNotice(
      icon: Symbols.error_rounded,
      message: message,
      actionLabel: "重试",
      actionKey: const Key("library-review-retry"),
      onAction: !enabled || mode.busy ? null : mode.retry,
    );
  }
}
