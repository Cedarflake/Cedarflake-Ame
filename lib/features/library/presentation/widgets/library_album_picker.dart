import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_album_pages.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_organization_actions.dart";
import "../../domain/library_organization.dart";
import "../library_organization_messages.dart";
import "library_album_editor.dart";

class LibraryAlbumChoice {
  const LibraryAlbumChoice(this.album);
  final LibraryOrganizationAlbum? album;
}

Future<LibraryAlbumChoice?> showLibraryAlbumPicker({
  required BuildContext context,
  required String actionLabel,
  LibraryOrganizationAlbum? initialAlbum,
  String? clearLabel,
  String? explanation,
  LibraryAlbumMembership? membership,
  String? initialError,
}) => showDialog<LibraryAlbumChoice>(
  context: context,
  builder: (_) => LibraryAlbumPicker(
    actionLabel: actionLabel,
    initialAlbum: initialAlbum,
    clearLabel: clearLabel,
    explanation: explanation,
    membership: membership,
    initialError: initialError,
  ),
);

class LibraryAlbumPicker extends ConsumerStatefulWidget {
  const LibraryAlbumPicker({
    required this.actionLabel,
    this.initialAlbum,
    this.clearLabel,
    this.explanation,
    this.membership,
    this.initialError,
    super.key,
  });
  final String actionLabel;
  final LibraryOrganizationAlbum? initialAlbum;
  final String? clearLabel;
  final String? explanation;
  final LibraryAlbumMembership? membership;
  final String? initialError;

  @override
  ConsumerState<LibraryAlbumPicker> createState() => _LibraryAlbumPickerState();
}

class _LibraryAlbumPickerState extends ConsumerState<LibraryAlbumPicker> {
  LibraryOrganizationAlbum? _selected;
  String? _after;
  final List<String?> _previous = [];
  bool _saving = false;
  bool _stopping = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _selected = widget.initialAlbum;
    _error = widget.initialError;
  }

  @override
  void dispose() {
    widget.membership?.dispose();
    super.dispose();
  }

  Future<void> _confirm(LibraryAlbumChoice choice) async {
    if (_saving) return;
    final membership = widget.membership;
    if (membership == null) {
      Navigator.of(context).pop(choice);
      return;
    }
    setState(() {
      _saving = true;
      _error = null;
    });
    final result = await ref
        .read(libraryOrganizationActionsProvider.notifier)
        .submit(membership.request(choice.album));
    if (!mounted) return;
    setState(() => _saving = false);
    if (_stopping && result is! LibraryOrganizationSaved) {
      Navigator.of(context).pop();
      return;
    }
    switch (result) {
      case LibraryOrganizationSaved():
        Navigator.of(context).pop(choice);
      case LibraryOrganizationRejected(:final failure):
        setState(() => _error = libraryOrganizationErrorMessage(failure.code));
      case LibraryOrganizationRetired():
        break;
    }
  }

  Future<void> _create(BigInt revision) async {
    setState(() => _saving = true);
    try {
      final result = await showLibraryAlbumEditor(
        context: context,
        kind: LibraryAlbumEditKind.create,
        revision: revision,
      );
      if (!mounted || result?.album == null) return;
      setState(() => _selected = result!.album);
      await widget.membership?.albumCreated();
    } on Object catch (error) {
      if (mounted) {
        setState(
          () => _error = libraryOrganizationErrorMessage(
            error is LibraryOrganizationFailure
                ? error.code
                : "unexpected_error",
          ),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _saving = false);
        if (_stopping) Navigator.of(context).pop();
      }
    }
  }

  void _cancel() {
    if (_stopping) return;
    if (_saving && widget.membership?.supportsCancellation != true) return;
    if (!_saving) {
      Navigator.of(context).pop();
      return;
    }
    setState(() => _stopping = true);
    widget.membership?.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final page = ref.watch(libraryAlbumPageProvider(_after));
    final saving = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    final busy = saving || _saving;
    return PopScope(
      canPop: !_saving,
      onPopInvokedWithResult: (didPop, result) {
        if (!didPop) _cancel();
      },
      child: AlertDialog(
        key: const Key("library-album-picker"),
        title: const Text("选择相册"),
        content: SizedBox(
          width: 440,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (widget.explanation case final explanation?) Text(explanation),
              if (_error case final error?)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Text(
                    error,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
              if (_selected case final selected?)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Text("已选择：${selected.name}"),
                ),
              page.when(
                skipLoadingOnRefresh: true,
                data: (value) => ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 360),
                  child: value.albums.isEmpty
                      ? const Padding(
                          padding: EdgeInsets.symmetric(vertical: 24),
                          child: Text("还没有相册，可以先新建一个。"),
                        )
                      : RadioGroup<String>(
                          groupValue: _selected?.id,
                          onChanged: (id) {
                            if (_saving) return;
                            final selected = value.albums
                                .where((album) => album.id == id)
                                .firstOrNull;
                            setState(() => _selected = selected);
                          },
                          child: AmeScrollMotion(
                            child: ListView.builder(
                              shrinkWrap: true,
                              itemCount: value.albums.length,
                              itemBuilder: (context, index) {
                                final album = value.albums[index];
                                return RadioListTile<String>(
                                  key: ValueKey("album-choice-${album.id}"),
                                  value: album.id,
                                  title: Text(album.name),
                                );
                              },
                            ),
                          ),
                        ),
                ),
                loading: () => const Padding(
                  padding: EdgeInsets.symmetric(vertical: 24),
                  child: LinearProgressIndicator(),
                ),
                error: (error, stack) => Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Text("相册暂时无法加载。"),
                    TextButton(
                      onPressed: () =>
                          ref.invalidate(libraryAlbumPageProvider(_after)),
                      child: const Text("重试"),
                    ),
                  ],
                ),
              ),
              if (page.value case final value?)
                if (_previous.isNotEmpty || value.nextAlbumId != null)
                  Row(
                    children: [
                      TextButton(
                        onPressed:
                            _previous.isEmpty || page.isLoading || _saving
                            ? null
                            : () => setState(
                                () => _after = _previous.removeLast(),
                              ),
                        child: const Text("上一页"),
                      ),
                      const Spacer(),
                      TextButton(
                        onPressed:
                            value.nextAlbumId == null ||
                                page.isLoading ||
                                _saving
                            ? null
                            : () => setState(() {
                                _previous.add(_after);
                                _after = value.nextAlbumId;
                              }),
                        child: const Text("下一页"),
                      ),
                    ],
                  ),
            ],
          ),
        ),
        actions: [
          TextButton(
            key: const Key("library-album-create"),
            onPressed: busy || page.isLoading || page.value == null
                ? null
                : () => _create(page.value!.revision),
            child: const Text("新建相册"),
          ),
          if (widget.clearLabel case final label?)
            TextButton(
              onPressed: busy
                  ? null
                  : () => _confirm(const LibraryAlbumChoice(null)),
              child: Text(label),
            ),
          TextButton(
            onPressed:
                _stopping ||
                    (_saving && widget.membership?.supportsCancellation != true)
                ? null
                : _cancel,
            child: Text(_stopping ? "正在停止" : "取消"),
          ),
          FilledButton(
            key: const Key("library-album-confirm"),
            onPressed:
                busy || _selected == null || page.isLoading || page.hasError
                ? null
                : () => _confirm(LibraryAlbumChoice(_selected)),
            child: Text(_saving ? "正在保存" : widget.actionLabel),
          ),
        ],
      ),
    );
  }
}
