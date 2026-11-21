import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_dialog_diagnostic.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../../../app/presentation/ame_choice.dart";
import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "../library_file_strings.dart";
import "library_file_action_dialog.dart";
import "library_file_export_history.dart";
import "library_path_text.dart";

Future<void> showLibraryFileHistory(BuildContext context) =>
    showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      isScrollControlled: true,
      useSafeArea: true,
      builder: (_) => const _FileHistory(),
    );

class _FileHistory extends ConsumerStatefulWidget {
  const _FileHistory();
  @override
  ConsumerState<_FileHistory> createState() => _FileHistoryState();
}

class _FileHistoryState extends ConsumerState<_FileHistory> {
  List<LibraryFileRecord> _items = const [];
  BigInt? _before;
  bool _loading = true;
  String? _error;
  bool _showExports = false;

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
          .history(before: before);
      if (mounted) {
        setState(() {
          _items = items;
          _before = before;
        });
      }
    } on Object {
      if (mounted) setState(() => _error = context.l10n.fileHistoryUnavailable);
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _recover(
    LibraryFileRecord item, {
    bool continueMove = false,
  }) async {
    if (continueMove) {
      final accepted = await showDialog<bool>(
        context: context,
        builder: (context) => AmeDialog(
          title: context.l10n.confirmContinueMove,
          content: AmeDialogItem(
            icon: Symbols.drive_file_move_rounded,
            title: item.source.split(RegExp(r"[\\/]")).last,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: Text(context.l10n.cancel),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: Text(context.l10n.continueMove),
            ),
          ],
        ),
      );
      if (accepted != true || !mounted) return;
    }
    await ref.read(libraryFileActionsProvider.notifier).recover([
      item.id,
    ], continueMove: continueMove);
    if (mounted) await _load(_before);
  }

  Future<void> _retry(LibraryFileRecord item) async {
    final submitted = await showLibraryFileAction(
      context: context,
      ref: ref,
      selection: LibraryFilesAtLocations([item.target]),
      action: item.action == LibraryFileAction.deletePermanently
          ? LibraryFileAction.recycle
          : item.action,
      initialName: item.source.split(RegExp(r"[\\/]")).last,
    );
    if (submitted == null) return;
    await submitted.completion;
    if (mounted) await _load();
  }

  @override
  Widget build(BuildContext context) {
    final busy = ref.watch(libraryFileActionsProvider).busy;
    return SizedBox(
      width: 760,
      height: MediaQuery.sizeOf(context).height * .68,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(24, 0, 24, 20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: Row(
                    children: [
                      Flexible(
                        child: Text(
                          context.l10n.fileOperationHistory,
                          style: Theme.of(context).textTheme.headlineSmall,
                        ),
                      ),
                      const SizedBox(width: 12),
                      AmeLoadingProgress(
                        active: _loading && !_showExports && _items.isNotEmpty,
                        size: 24,
                        semanticsLabel: context.l10n.updatingFileRecords,
                      ),
                    ],
                  ),
                ),
                AmeChoice<bool>.menu(
                  value: _showExports,
                  entries: [
                    AmeChoiceEntry(
                      value: false,
                      label: context.l10n.fileRecords,
                    ),
                    AmeChoiceEntry(
                      value: true,
                      label: context.l10n.operationReport,
                    ),
                  ],
                  onSelected: (value) {
                    if (value != null) setState(() => _showExports = value);
                  },
                ),
              ],
            ),
            const SizedBox(height: 16),
            Expanded(
              child: AmeLoadingContent(
                isLoading: _loading && !_showExports && _items.isEmpty,
                semanticsLabel: context.l10n.loadingFileRecords,
                child: switch ((_showExports, _items.isEmpty && !_loading)) {
                  (true, _) => const LibraryFileExportHistory(),
                  (false, true) when _error != null => AmeDialogReadFailure(
                    message: _error!,
                    onRetry: () => _load(_before),
                  ),
                  (false, true) => Center(
                    child: Text(context.l10n.noFileOperations),
                  ),
                  (false, false) => AmeScrollMotion(
                    child: ListView.builder(
                      itemCount: _items.length,
                      itemBuilder: (context, index) {
                        final item = _items[index];
                        final status = item.failure == null
                            ? LibraryFileStrings.phase(context.l10n, item.phase)
                            : LibraryFileStrings.failure(
                                context.l10n,
                                item.failure,
                                action: item.action,
                              );
                        return ListTile(
                          onTap: item.detail == null
                              ? null
                              : () => showDialog<void>(
                                  context: context,
                                  builder: (context) => AmeDialog(
                                    title: context.l10n.fileOperationDetails,
                                    content: AmeDialogDiagnostic(
                                      detail: item.detail!,
                                    ),
                                    actions: [
                                      TextButton(
                                        onPressed: () => Navigator.pop(context),
                                        child: Text(context.l10n.acknowledge),
                                      ),
                                    ],
                                  ),
                                ),
                          contentPadding: const EdgeInsets.symmetric(
                            vertical: 6,
                          ),
                          leading: const Icon(Symbols.draft_rounded),
                          title: Text(
                            "${LibraryFileStrings.action(context.l10n, item.action)} · ${item.source.split(RegExp(r"[\\/]")).last}",
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          subtitle: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(status),
                              Text(
                                displayLibraryPath(
                                  item.destination ?? item.source,
                                ),
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                              ),
                            ],
                          ),
                          trailing: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              if (item.canRetry)
                                TextButton(
                                  onPressed: busy ? null : () => _retry(item),
                                  child: Text(context.l10n.retry),
                                ),
                              if (item.canContinueMove)
                                TextButton(
                                  onPressed: busy
                                      ? null
                                      : () =>
                                            _recover(item, continueMove: true),
                                  child: Text(context.l10n.continueMove),
                                )
                              else if ({
                                LibraryFileRecordPhase.prepared,
                                LibraryFileRecordPhase.nameOccupied,
                                LibraryFileRecordPhase.executing,
                                LibraryFileRecordPhase.removingSource,
                                LibraryFileRecordPhase.nativeCompleted,
                                LibraryFileRecordPhase.uncertain,
                              }.contains(item.phase))
                                TextButton(
                                  onPressed: busy ? null : () => _recover(item),
                                  child: Text(context.l10n.checkResults),
                                ),
                            ],
                          ),
                        );
                      },
                    ),
                  ),
                },
              ),
            ),
            if (_error case final error?
                when !_showExports && _items.isNotEmpty)
              AmeDialogReadFailure(
                message: error,
                onRetry: () => _load(_before),
              ),
            const SizedBox(height: 12),
            if (!_showExports)
              Row(
                children: [
                  TextButton.icon(
                    onPressed: () async {
                      try {
                        await ref
                            .read(libraryFileOperationsProvider)
                            .openRecycleBin();
                      } on Object {
                        if (context.mounted) {
                          showAmeTransientFeedback(
                            context,
                            message: context.l10n.recycleBinOpenFailed,
                            severity: AmeNotificationSeverity.error,
                          );
                        }
                      }
                    },
                    icon: const Icon(Symbols.recycling_rounded),
                    label: Text(context.l10n.openRecycleBin),
                  ),
                  const Spacer(),
                  TextButton(
                    onPressed: _loading || _before == null
                        ? null
                        : () => _load(),
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
        ),
      ),
    );
  }
}
