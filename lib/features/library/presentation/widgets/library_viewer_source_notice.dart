import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../coordination/library_viewer_source_state.dart";
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
          ? context.l10n.viewerSourceChanged
          : context.l10n.viewerSourceMissing,
      actionLabel: changed
          ? context.l10n.retryLoading
          : context.l10n.backToLibrary,
      onAction: changed ? onReload : onBack,
      actionKey: const Key("viewer-source-action"),
    );
  }
}
