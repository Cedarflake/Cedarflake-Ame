import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_message_text.dart";
import "../coordination/library_organization_mode.dart";
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
      message: message.resolve(context.l10n),
      actionLabel: context.l10n.retry,
      actionKey: const Key("library-review-retry"),
      onAction: !enabled || mode.busy ? null : mode.retry,
    );
  }
}
