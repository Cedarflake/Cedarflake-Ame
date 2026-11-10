import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "library_path_text.dart";

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
        setState(() => _error = "暂时无法读取结果清单记录，请重试。");
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
        if (_loading) const LinearProgressIndicator(),
        if (_error case final error?)
          TextButton(onPressed: () => _load(_before), child: Text(error)),
        Expanded(
          child: _items.isEmpty && !_loading
              ? const Center(child: Text("暂无导出记录"))
              : AmeScrollMotion(
                  child: ListView.builder(
                    itemCount: _items.length,
                    itemBuilder: (context, index) {
                      final item = _items[index];
                      final status = switch (item.reportState) {
                        LibraryFileReportState.pending => "清单尚未生成",
                        LibraryFileReportState.notNeeded => "没有特殊结果，无需清单",
                        LibraryFileReportState.written => "结果清单已保存",
                        LibraryFileReportState.failed => "清单写入失败，文件操作结果已保留",
                      };
                      return ListTile(
                        contentPadding: const EdgeInsets.symmetric(vertical: 6),
                        leading: const Icon(Symbols.description_rounded),
                        title: Text(status),
                        subtitle: Text(
                          "${item.selected} 个文件 · 已跳过 ${item.skipped} 个\n${displayLibraryPath(item.directory)}",
                          maxLines: 3,
                          overflow: TextOverflow.ellipsis,
                        ),
                        onTap: () => showDialog<void>(
                          context: context,
                          builder: (context) => AlertDialog(
                            title: const Text("结果清单"),
                            content: SelectableText(
                              "$status\n${item.createdAt}\n${displayLibraryPath(item.reportPath ?? item.directory)}${item.failure == null ? "" : "\n${item.failure!.detail}"}",
                            ),
                            actions: [
                              TextButton(
                                onPressed: () => Navigator.pop(context),
                                child: const Text("知道了"),
                              ),
                            ],
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
                                      ? "生成清单"
                                      : "重新生成",
                                ),
                              ),
                      );
                    },
                  ),
                ),
        ),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: [
            TextButton(
              onPressed: _loading || _before == null ? null : () => _load(),
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
    );
  }
}
