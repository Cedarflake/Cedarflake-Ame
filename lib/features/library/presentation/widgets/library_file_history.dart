import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../../../app/presentation/ame_choice.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "../library_file_strings.dart";
import "library_file_action_dialog.dart";
import "library_file_export_history.dart";

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
      if (mounted) setState(() => _error = "暂时无法读取文件操作记录，请重试。");
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
        builder: (context) => AlertDialog(
          title: const Text("继续移动这个文件？"),
          content: const Text("将核对已保存的副本，然后移除原位置的文件。"),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text("取消"),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text("继续移动"),
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
    await showLibraryFileAction(
      context: context,
      ref: ref,
      selection: LibraryFilesAtLocations([item.target]),
      action: item.action == LibraryFileAction.deletePermanently
          ? LibraryFileAction.recycle
          : item.action,
      initialName: item.source.split(RegExp(r"[\\/]")).last,
    );
    if (mounted) await _load(_before);
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
                  child: Text(
                    "文件操作记录",
                    style: Theme.of(context).textTheme.headlineSmall,
                  ),
                ),
                AmeChoice<bool>(
                  value: _showExports,
                  width: 160,
                  entries: const [
                    AmeChoiceEntry(value: false, label: "文件记录"),
                    AmeChoiceEntry(value: true, label: "结果清单"),
                  ],
                  onSelected: (value) {
                    if (value != null) setState(() => _showExports = value);
                  },
                ),
              ],
            ),
            const SizedBox(height: 16),
            if (_loading && !_showExports) const LinearProgressIndicator(),
            if (_error case final error? when !_showExports)
              TextButton(onPressed: () => _load(_before), child: Text(error)),
            Expanded(
              child: switch ((_showExports, _items.isEmpty && !_loading)) {
                (true, _) => const LibraryFileExportHistory(),
                (false, true) => const Center(child: Text("暂无文件操作记录")),
                (false, false) => AmeScrollMotion(
                  child: ListView.builder(
                    itemCount: _items.length,
                    itemBuilder: (context, index) {
                      final item = _items[index];
                      final status = item.failure == null
                          ? LibraryFileStrings.phase(item.phase)
                          : LibraryFileStrings.failure(
                              item.failure,
                              action: item.action,
                            );
                      return ListTile(
                        onTap: item.detail == null
                            ? null
                            : () => showDialog<void>(
                                context: context,
                                builder: (context) => AlertDialog(
                                  title: const Text("文件操作详情"),
                                  content: SelectableText(item.detail!),
                                  actions: [
                                    TextButton(
                                      onPressed: () => Navigator.pop(context),
                                      child: const Text("知道了"),
                                    ),
                                  ],
                                ),
                              ),
                        contentPadding: const EdgeInsets.symmetric(vertical: 6),
                        leading: const Icon(Symbols.draft_rounded),
                        title: Text(
                          "${LibraryFileStrings.action(item.action)} · ${item.source.split(RegExp(r"[\\/]")).last}",
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        subtitle: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(status),
                            Text(
                              item.destination ?? item.source,
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
                                child: const Text("重试"),
                              ),
                            if (item.canContinueMove)
                              TextButton(
                                onPressed: busy
                                    ? null
                                    : () => _recover(item, continueMove: true),
                                child: const Text("继续移动"),
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
                                child: const Text("核对结果"),
                              ),
                          ],
                        ),
                      );
                    },
                  ),
                ),
              },
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
                        if (mounted) {
                          setState(() => _error = "无法打开回收站，请从 Windows 桌面打开。");
                        }
                      }
                    },
                    icon: const Icon(Symbols.recycling_rounded),
                    label: const Text("打开回收站"),
                  ),
                  const Spacer(),
                  TextButton(
                    onPressed: _loading || _before == null
                        ? null
                        : () => _load(),
                    child: const Text("最新记录"),
                  ),
                  TextButton(
                    onPressed: _loading || _items.length < 40
                        ? null
                        : () => _load(_items.last.id),
                    child: const Text("更早记录"),
                  ),
                ],
              ),
          ],
        ),
      ),
    );
  }
}
