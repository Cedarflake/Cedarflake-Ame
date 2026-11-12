import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../application/directory_picker.dart";
import "../../../../app/presentation/ame_choice.dart";
import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_dialog_location.dart";
import "../../application/library_file_operations.dart";
import "../../application/library_file_selection.dart";
import "../../domain/library_file_operations.dart";
import "../library_file_strings.dart";
import "library_path_text.dart";

class LibraryFileActionSubmission {
  const LibraryFileActionSubmission(this.completion);
  final Future<LibraryFileProgress?> completion;
}

Future<LibraryFileActionSubmission?> showLibraryFileAction({
  required BuildContext context,
  required WidgetRef ref,
  required LibraryFileSelection selection,
  required LibraryFileAction action,
  String? initialName,
  LibraryFileOrganizationRule initialRule = LibraryFileOrganizationRule.direct,
}) async {
  final commands = ref.read(libraryFileActionsProvider.notifier);
  if (ref.read(libraryFileActionsProvider).busy) return null;
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
      initialRule: initialRule,
    ),
  );
  if (command == null) {
    await session.close();
    return null;
  } else {
    return LibraryFileActionSubmission(commands.execute(session, command));
  }
}

class _FileActionDialog extends ConsumerStatefulWidget {
  const _FileActionDialog({
    required this.session,
    required this.action,
    this.initialName,
    required this.initialRule,
  });
  final LibraryFileSelectionSession session;
  final LibraryFileAction action;
  final String? initialName;
  final LibraryFileOrganizationRule initialRule;
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
  late LibraryFileOrganizationRule _rule = widget.initialRule;

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
        _summary!.files > 0 &&
        _failure == null &&
        !_picking &&
        (!destination || _directory != null) &&
        (!rename || (_summary?.files == 1 && _name.text.trim().isNotEmpty));
    final label = LibraryFileStrings.action(widget.action);
    final preparationFailed = _summary == null && _failure != null;
    return AmeDialog(
      isLoading: _summary == null && _failure == null,
      loadingLabel: "正在读取图片",
      title: deletion && _summary != null
          ? LibraryFileStrings.deletionTitle(_summary!)
          : "$label图片",
      content: preparationFailed
          ? AmeDialogError(
              LibraryFileStrings.failure(_failure, action: widget.action),
            )
          : Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                if (_summary case final summary?
                    when destination && summary.files > 1) ...[
                  Text("${summary.files} 个文件"),
                  const SizedBox(height: 16),
                ],
                if (destination) ...[
                  AmeDialogField(
                    label: "目标文件夹",
                    child: _directory == null
                        ? Align(
                            alignment: Alignment.centerLeft,
                            child: TextButton.icon(
                              onPressed: _picking ? null : _pick,
                              icon: const Icon(Symbols.folder_open_rounded),
                              label: const Text("选择文件夹"),
                            ),
                          )
                        : AmeDialogPath(
                            path: displayLibraryPath(_directory!),
                            trailing: TextButton(
                              onPressed: _picking ? null : _pick,
                              child: const Text("更改"),
                            ),
                          ),
                  ),
                  const SizedBox(height: 16),
                  AmeDialogField(
                    label: "文件夹结构",
                    child: Align(
                      alignment: AlignmentDirectional.centerStart,
                      child: AmeChoice<LibraryFileOrganizationRule>.menu(
                        value: _rule,
                        entries: [
                          for (final rule in LibraryFileOrganizationRule.values)
                            AmeChoiceEntry(
                              value: rule,
                              label: LibraryFileStrings.rule(rule),
                            ),
                        ],
                        onSelected: (value) {
                          if (value != null) setState(() => _rule = value);
                        },
                      ),
                    ),
                  ),
                  if (_rule != LibraryFileOrganizationRule.direct) ...[
                    const SizedBox(height: 8),
                    Text(LibraryFileStrings.ruleDescription(_rule)),
                  ],
                ],
                if (rename) ...[
                  TextField(
                    controller: _name,
                    autofocus: true,
                    decoration: const InputDecoration(labelText: "文件名"),
                    onChanged: (_) => setState(() {}),
                  ),
                ],
                if (deletion) ...[
                  if (_summary == null || _summary!.files > 0) ...[
                    Text(
                      _permanent ? "文件将被永久删除，无法从回收站恢复。" : "文件将移入 Windows 回收站。",
                    ),
                    const SizedBox(height: 8),
                    MergeSemantics(
                      child: GestureDetector(
                        behavior: HitTestBehavior.opaque,
                        excludeFromSemantics: true,
                        onTap: () => setState(() => _permanent = !_permanent),
                        child: Row(
                          children: [
                            Checkbox(
                              key: const Key("library-file-permanent-delete"),
                              value: _permanent,
                              onChanged: (value) =>
                                  setState(() => _permanent = value ?? false),
                            ),
                            const SizedBox(width: 4),
                            const Flexible(child: Text("永久删除")),
                          ],
                        ),
                      ),
                    ),
                  ],
                ],
                if (_failure case final failure?) ...[
                  const SizedBox(height: 16),
                  AmeDialogError(
                    LibraryFileStrings.failure(failure, action: widget.action),
                  ),
                ],
              ],
            ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(preparationFailed ? "返回" : "取消"),
        ),
        if (!preparationFailed)
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
                      rule: _rule,
                    ),
                  )
                : null,
            child: Text(_permanent ? "永久删除" : label),
          ),
      ],
    );
  }
}
