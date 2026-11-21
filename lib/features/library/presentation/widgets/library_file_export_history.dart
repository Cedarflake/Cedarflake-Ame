import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "library_path_text.dart";
import "library_file_export_details.dart";

class LibraryFileExportHistory extends ConsumerStatefulWidget {
  const LibraryFileExportHistory({super.key});
  @override
  ConsumerState<LibraryFileExportHistory> createState() =>
      _ExportHistoryState();
}

class _ExportHistoryState extends ConsumerState<LibraryFileExportHistory> {
  List<LibraryFileExport> _items = const [];
  BigInt? _before;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  Future<void> _load([BigInt? before]) async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final items = await ref
          .read(libraryFileOperationsProvider)
          .exports(before: before);
      if (mounted) {
        setState(() {
          _items = items;
          _before = before;
        });
      }
    } on Object {
      if (mounted) {
        setState(() => _error = context.l10n.reportHistoryUnavailable);
      }
    } finally {
      if (mounted) {
        setState(() => _loading = false);
      }
    }
  }

  Future<void> _write(LibraryFileExport item) async {
    await ref
        .read(libraryFileActionsProvider.notifier)
        .writeReport(item.batchId);
    if (mounted) {
      await _load(_before);
    }
  }

  @override
  Widget build(BuildContext context) {
    final busy = ref.watch(libraryFileActionsProvider).busy;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          child: AmeLoadingContent(
            isLoading: _loading && _items.isEmpty,
            semanticsLabel: context.l10n.loadingReport,
            child: switch ((_items.isEmpty, _error, _loading)) {
              (true, final String error, _) => AmeDialogReadFailure(
                message: error,
                onRetry: () => _load(_before),
              ),
              (true, null, false) => Center(
                child: Text(context.l10n.noExportHistory),
              ),
              _ => AmeScrollMotion(
                child: ListView.builder(
                  itemCount: _items.length,
                  itemBuilder: (context, index) {
                    final item = _items[index];
                    final status = switch (item.reportState) {
                      LibraryFileReportState.pending =>
                        context.l10n.reportPending,
                      LibraryFileReportState.notNeeded =>
                        context.l10n.noReportNeeded,
                      LibraryFileReportState.written =>
                        context.l10n.reportSaved,
                      LibraryFileReportState.failed =>
                        context.l10n.reportWriteFailedStatus,
                    };
                    final counts = [
                      if (item.selected > 0)
                        context.l10n.fileCount(item.selected),
                      if (item.skipped > 0)
                        context.l10n.filesSkipped(item.skipped),
                    ];
                    return ListTile(
                      contentPadding: const EdgeInsets.symmetric(vertical: 6),
                      leading: const Icon(Symbols.description_rounded),
                      title: Text(status),
                      subtitle: Text(
                        [
                          if (counts.isNotEmpty) counts.join(" · "),
                          displayLibraryPath(item.directory),
                        ].join("\n"),
                        maxLines: 3,
                        overflow: TextOverflow.ellipsis,
                      ),
                      onTap: () => showDialog<void>(
                        context: context,
                        builder: (context) => LibraryFileExportDetails(
                          item: item,
                          status: status,
                        ),
                      ),
                      trailing:
                          item.reportState == LibraryFileReportState.notNeeded
                          ? null
                          : TextButton(
                              onPressed: busy ? null : () => _write(item),
                              child: Text(
                                item.reportState ==
                                        LibraryFileReportState.pending
                                    ? context.l10n.createReport
                                    : context.l10n.regenerate,
                              ),
                            ),
                    );
                  },
                ),
              ),
            },
          ),
        ),
        if (_error case final error? when _items.isNotEmpty)
          AmeDialogReadFailure(message: error, onRetry: () => _load(_before)),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            AmeLoadingProgress(
              active: _loading && _items.isNotEmpty,
              size: 24,
              semanticsLabel: context.l10n.updatingReport,
            ),
            const Spacer(),
            TextButton(
              onPressed: _loading || _before == null ? null : () => _load(),
              child: Text(context.l10n.latestRecords),
            ),
            TextButton(
              onPressed: _loading || _items.length < 40
                  ? null
                  : () => _load(_items.last.id),
              child: Text(context.l10n.earlierRecords),
            ),
          ],
        ),
      ],
    );
  }
}
