import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../library_strings.dart";
import "../library_viewer_source_state.dart";
import "library_viewer_notice.dart";

class LibraryViewerSourceNoticeBanner extends StatelessWidget {
  const LibraryViewerSourceNoticeBanner({
    required this.notice,
    required this.onBack,
    this.onReload,
    super.key,
  });

  final LibraryViewerSourceNotice notice;
  final VoidCallback onBack;
  final VoidCallback? onReload;

  @override
  Widget build(BuildContext context) {
    final changed = notice == LibraryViewerSourceNotice.changed;
    return LibraryViewerNotice(
      icon: Symbols.info_rounded,
      message: changed
          ? LibraryStrings.viewerSourceChanged
          : LibraryStrings.viewerSourceMissing,
      actionLabel: changed
          ? LibraryStrings.reload
          : LibraryStrings.backToLibrary,
      onAction: changed ? onReload : onBack,
      actionKey: const Key("viewer-source-action"),
    );
  }
}
