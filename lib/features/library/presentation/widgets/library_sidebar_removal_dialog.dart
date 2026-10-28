import "package:flutter/material.dart";

import "../../application/library_sidebar_removal.dart";

Future<Set<String>> showLibrarySidebarRemovalDialog({
  required BuildContext context,
  required LibrarySidebarRemoval removal,
  required String title,
  required String detail,
  required String actionLabel,
}) async {
  await showDialog<void>(
    context: context,
    barrierDismissible: false,
    builder: (context) => _SidebarRemovalDialog(
      removal: removal,
      title: title,
      detail: detail,
      actionLabel: actionLabel,
    ),
  );
  final completed = removal.completed;
  removal.dispose();
  return completed;
}

class _SidebarRemovalDialog extends StatefulWidget {
  const _SidebarRemovalDialog({
    required this.removal,
    required this.title,
    required this.detail,
    required this.actionLabel,
  });
  final LibrarySidebarRemoval removal;
  final String title;
  final String detail;
  final String actionLabel;

  @override
  State<_SidebarRemovalDialog> createState() => _SidebarRemovalDialogState();
}

class _SidebarRemovalDialogState extends State<_SidebarRemovalDialog> {
  LibrarySidebarRemoval get removal => widget.removal;

  @override
  void dispose() {
    removal.dispose();
    super.dispose();
  }

  Future<void> _run(BuildContext context) async {
    await removal.run();
    if (context.mounted && removal.isComplete && removal.error == null) {
      Navigator.of(context).pop();
    }
  }

  String get _actionLabel {
    if (removal.isRunning) {
      final status = removal.isStopping ? "正在停止" : "正在处理";
      return "$status ${removal.completed.length} / ${removal.entries.length}";
    }
    if (removal.error != null) return "重试";
    return widget.actionLabel;
  }

  VoidCallback? _cancelAction(BuildContext context) {
    if (!removal.isRunning) return () => Navigator.of(context).pop();
    if (removal.isStopping) return null;
    return removal.stop;
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: removal,
    builder: (context, _) => PopScope(
      canPop: !removal.isRunning,
      child: AlertDialog(
        key: const Key("library-sidebar-removal-dialog"),
        title: Text(widget.title),
        content: SizedBox(
          width: 400,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(widget.detail),
              const SizedBox(height: 12),
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 200),
                child: ListView.builder(
                  shrinkWrap: true,
                  itemCount: removal.entries.length,
                  itemBuilder: (context, index) => Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    child: Text(removal.entries[index].name),
                  ),
                ),
              ),
              if (removal.error case final error?) ...[
                const SizedBox(height: 12),
                Text(
                  "已完成 ${removal.completed.length} / ${removal.entries.length} 项。$error",
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: _cancelAction(context),
            child: Text(removal.isRunning ? "停止" : "取消"),
          ),
          FilledButton(
            onPressed: removal.isRunning || removal.isComplete
                ? null
                : () => _run(context),
            child: Text(_actionLabel),
          ),
        ],
      ),
    ),
  );
}
