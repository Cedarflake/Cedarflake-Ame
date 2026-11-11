import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

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
        Expanded(
          child: AmeLoadingContent(
            isLoading: _loading && _items.isEmpty,
            semanticsLabel: "正在读取结果清单",
            child: switch ((_items.isEmpty, _error, _loading)) {
              (true, final String error, _) => AmeDialogReadFailure(
                message: error,
                onRetry: () => _load(_before),
              ),
              (true, null, false) => const Center(child: Text("暂无导出记录")),
              _ => AmeScrollMotion(
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
                    final counts = [
                      if (item.selected > 0) "${item.selected} 个文件",
                      if (item.skipped > 0) "已跳过 ${item.skipped} 个",
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
                                    ? "生成清单"
                                    : "重新生成",
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
              semanticsLabel: "正在更新结果清单",
            ),
            const Spacer(),
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
