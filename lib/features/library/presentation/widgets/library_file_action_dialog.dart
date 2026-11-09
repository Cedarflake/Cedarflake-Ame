import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../application/directory_picker.dart";
import "../../application/library_file_operations.dart";
import "../../application/library_file_selection.dart";
import "../../domain/library_file_operations.dart";
import "../library_file_strings.dart";

Future<void> showLibraryFileAction({
  required BuildContext context,
  required WidgetRef ref,
  required LibraryFileSelection selection,
  required LibraryFileAction action,
  String? initialName,
}) async {
  final commands = ref.read(libraryFileActionsProvider.notifier);
  if (ref.read(libraryFileActionsProvider).busy) return;
  final session = LibraryFileSelectionSession(
    ref.read(libraryFileOperationsProvider),
    selection,
  );
  final command = await showDialog<LibraryFileCommand>(
    context: context,
    builder: (_) => _FileActionDialog(
      session: session,
      action: action,
      initialName: initialName,
    ),
  );
  if (command == null) {
    await session.close();
  } else {
    unawaited(commands.execute(session, command));
  }
}

class _FileActionDialog extends ConsumerStatefulWidget {
  const _FileActionDialog({
    required this.session,
    required this.action,
    this.initialName,
  });
  final LibraryFileSelectionSession session;
  final LibraryFileAction action;
  final String? initialName;
  @override
  ConsumerState<_FileActionDialog> createState() => _FileActionDialogState();
}

class _FileActionDialogState extends ConsumerState<_FileActionDialog> {
  late final TextEditingController _name = TextEditingController(
    text: widget.initialName,
  );
  LibraryFileSelectionSummary? _summary;
  LibraryFileFailure? _failure;
  String? _directory;
  bool _permanent = false;
  bool _picking = false;

  @override
  void initState() {
    super.initState();
    unawaited(_prepare());
  }

  Future<void> _prepare() async {
    try {
      final summary = await widget.session.prepare();
      if (mounted) setState(() => _summary = summary);
    } on Object catch (error) {
      if (mounted) {
        setState(
          () => _failure = error is LibraryFileFailure
              ? error
              : LibraryFileFailure("file_operation_failed", error.toString()),
        );
      }
    }
  }

  Future<void> _pick() async {
    setState(() => _picking = true);
    try {
      final path = await ref.read(directoryPickerProvider).pickDirectory();
      if (mounted && path != null) {
        setState(() {
          _directory = path;
          _failure = null;
        });
      }
    } on Object catch (error) {
      if (mounted) {
        setState(
          () => _failure = LibraryFileFailure(
            "file_operation_destination_unavailable",
            error.toString(),
          ),
        );
      }
    } finally {
      if (mounted) setState(() => _picking = false);
    }
  }

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final destination =
        widget.action == LibraryFileAction.copy ||
        widget.action == LibraryFileAction.move;
    final rename = widget.action == LibraryFileAction.rename;
    final deletion = widget.action == LibraryFileAction.recycle;
    final canSubmit =
        !ref.watch(libraryFileActionsProvider).busy &&
        _summary != null &&
        _failure == null &&
        !_picking &&
        (!destination || _directory != null) &&
        (!rename || _name.text.trim().isNotEmpty);
    final label = LibraryFileStrings.action(widget.action);
    return AlertDialog(
      title: Text("$label图片"),
      content: SizedBox(
        width: 440,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (_summary case final summary?)
              Text("${summary.files} 个文件")
            else if (_failure == null)
              const LinearProgressIndicator(),
            if (_failure case final failure?) ...[
              const SizedBox(height: 12),
              Text(
                LibraryFileStrings.failure(failure),
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
            if (destination) ...[
              const SizedBox(height: 16),
              OutlinedButton.icon(
                onPressed: _picking ? null : _pick,
                icon: const Icon(Symbols.folder_open_rounded),
                label: Text(
                  _directory ?? "选择目标文件夹",
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
            if (rename) ...[
              const SizedBox(height: 16),
              TextField(
                controller: _name,
                autofocus: true,
                decoration: const InputDecoration(labelText: "文件名"),
                onChanged: (_) => setState(() {}),
              ),
            ],
            if (deletion) ...[
              const SizedBox(height: 12),
              Text(
                _permanent ? "这些文件将被永久删除，无法从回收站恢复。" : "这些文件将移入 Windows 回收站。",
              ),
              const SizedBox(height: 8),
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text("永久删除"),
                value: _permanent,
                onChanged: (value) =>
                    setState(() => _permanent = value ?? false),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text("取消"),
        ),
        FilledButton(
          onPressed: canSubmit
              ? () => Navigator.pop(
                  context,
                  LibraryFileCommand(
                    _permanent
                        ? LibraryFileAction.deletePermanently
                        : widget.action,
                    directory: _directory,
                    name: rename ? _name.text.trim() : null,
                  ),
                )
              : null,
          child: Text(label),
        ),
      ],
    );
  }
}
