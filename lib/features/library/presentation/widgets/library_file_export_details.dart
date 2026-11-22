import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_dialog_diagnostic.dart";
import "../../../../app/presentation/ame_dialog_location.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_notification_time.dart";
import "../../application/library_platform_actions.dart";
import "../../domain/library_file_operations.dart";
import "library_path_text.dart";

class LibraryFileExportDetails extends ConsumerWidget {
  const LibraryFileExportDetails({
    required this.item,
    required this.status,
    super.key,
  });

  final LibraryFileExport item;
  final String status;

  @override
  Widget build(BuildContext context, WidgetRef ref) => AmeDialog(
    title: context.l10n.operationReport,
    content: AmeDialogBody(
      children: [
        AmeDialogItem(icon: Symbols.description_rounded, title: status),
        AmeDialogField(
          label: context.l10n.timeCreated,
          child: Text(
            formatAmeNotificationOccurredAt(context.l10n, item.createdAt),
          ),
        ),
        if (item.selected > 0 || item.skipped > 0) ...[
          AmeDialogField(
            label: context.l10n.affectedFiles,
            child: Text(
              [
                if (item.selected > 0)
                  context.l10n.selectedCountShort(item.selected),
                if (item.skipped > 0) context.l10n.filesSkipped(item.skipped),
              ].join(" · "),
            ),
          ),
        ],
        AmeDialogLocation(
          label: context.l10n.destinationFolder,
          path: displayLibraryPath(item.directory),
          onOpen: () {
            final platform = ref.read(libraryPlatformActionsProvider);
            return item.reportPath == null
                ? platform.revealDirectory(item.directory)
                : platform.revealFile(item.reportPath!);
          },
        ),
        if (item.reportPath case final path?) ...[
          AmeDialogField(
            label: context.l10n.reportFile,
            child: Text(path.split(RegExp(r"[\\/]")).last),
          ),
        ],
        if (item.failure case final failure?) ...[
          AmeDialogDiagnostic(detail: failure.detail),
        ],
      ],
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: Text(context.l10n.acknowledge),
      ),
    ],
  );
}
