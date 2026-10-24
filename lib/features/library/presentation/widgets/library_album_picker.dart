import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_album_pages.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_organization_requests.dart";
import "../../application/library_progress_request.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_review.dart";
import "../library_organization_messages.dart";
import "library_album_editor.dart";
import "library_conflict_albums.dart";

class LibraryAlbumChoice {
  const LibraryAlbumChoice(this.album) : progress = null;
  const LibraryAlbumChoice.progress(this.progress) : album = null;
  final LibraryOrganizationAlbum? album;
  final LibraryProgressAction? progress;
}

Future<LibraryAlbumChoice?> showLibraryAlbumPicker({
  required BuildContext context,
  required String actionLabel,
  IconData actionIcon = Symbols.bookmark_add_rounded,
  LibraryOrganizationAlbum? initialAlbum,
  LibraryOrganizationAlbum? currentAlbum,
  String? clearLabel,
  IconData clearIcon = Symbols.bookmark_remove_rounded,
  String? explanation,
  LibraryOrganizationTarget? conflictTarget,
  LibraryAlbumMembership? membership,
  String? initialError,
  LibraryOrganizationRequest Function(LibraryProgressAction)?
  progressRequestFactory,
  String ignoreLabel = "忽略此图",
}) => showModalBottomSheet<LibraryAlbumChoice>(
  context: context,
  showDragHandle: true,
  isScrollControlled: true,
  useSafeArea: true,
  enableDrag: false,
  builder: (_) => LibraryAlbumPicker(
    actionLabel: actionLabel,
    actionIcon: actionIcon,
    initialAlbum: initialAlbum,
    currentAlbum: currentAlbum,
    clearLabel: clearLabel,
    clearIcon: clearIcon,
    explanation: explanation,
    conflictTarget: conflictTarget,
    membership: membership,
    initialError: initialError,
    progressRequestFactory: progressRequestFactory,
    ignoreLabel: ignoreLabel,
  ),
);

class LibraryAlbumPicker extends ConsumerStatefulWidget {
  const LibraryAlbumPicker({
    required this.actionLabel,
    this.actionIcon = Symbols.bookmark_add_rounded,
    this.initialAlbum,
    this.currentAlbum,
    this.clearLabel,
    this.clearIcon = Symbols.bookmark_remove_rounded,
    this.explanation,
    this.conflictTarget,
    this.membership,
    this.initialError,
    this.progressRequestFactory,
    this.ignoreLabel = "忽略此图",
    super.key,
  });
  final String actionLabel;
  final IconData actionIcon;
  final LibraryOrganizationAlbum? initialAlbum;
  final LibraryOrganizationAlbum? currentAlbum;
  final String? clearLabel;
  final IconData clearIcon;
  final String? explanation;
  final LibraryOrganizationTarget? conflictTarget;
  final LibraryAlbumMembership? membership;
  final String? initialError;
  final LibraryOrganizationRequest Function(LibraryProgressAction)?
  progressRequestFactory;
  final String ignoreLabel;

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
  LibraryProgressSession? _progress;

  @override
  void initState() {
    super.initState();
    _selected = widget.initialAlbum;
    _error = widget.initialError;
    final create = widget.progressRequestFactory;
    if (create != null) _progress = LibraryProgressSession(create);
  }

  @override
  void dispose() {
    widget.membership?.dispose();
    _progress?.dispose();
    super.dispose();
  }

  Future<void> _confirm(LibraryAlbumChoice choice) async {
    if (_saving) return;
    final membership = widget.membership;
    if (membership == null) {
      Navigator.of(context).pop(choice);
      return;
    }
    await _save(membership.request(choice.album), choice);
  }

  Future<void> _saveProgress(LibraryProgressAction action) async {
    final progress = _progress;
    if (_saving || progress == null) return;
    await _save(progress.request(action), LibraryAlbumChoice.progress(action));
  }

  Future<void> _save(
    LibraryOrganizationRequest request,
    LibraryAlbumChoice choice,
  ) async {
    setState(() {
      _saving = true;
      _error = null;
    });
    final result = await ref
        .read(libraryOrganizationActionsProvider.notifier)
        .submit(request);
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
    _progress?.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final page = ref.watch(libraryAlbumPageProvider(_after));
    final saving = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    final busy = saving || _saving;
    var saveLabel = widget.actionLabel;
    if (_stopping) {
      saveLabel = "正在停止";
    } else if (_saving) {
      saveLabel = "正在保存";
    }
    return PopScope(
      canPop: !_saving,
      onPopInvokedWithResult: (didPop, result) {
        if (!didPop) _cancel();
      },
      child: SafeArea(
        child: Padding(
          key: const Key("library-album-picker"),
          padding: const EdgeInsets.fromLTRB(24, 8, 24, 24),
          child: ConstrainedBox(
            constraints: BoxConstraints(
              maxHeight: MediaQuery.sizeOf(context).height * .8,
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text("选择相册", style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 16),
                Flexible(
                  child: SizedBox(
                    width: double.infinity,
                    child: AmeScrollMotion(
                      child: SingleChildScrollView(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            if (widget.explanation case final explanation?)
                              Text(explanation),
                            if (widget.conflictTarget case final target?)
                              LibraryConflictAlbumsView(target: target),
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
                            page.when(
                              skipLoadingOnRefresh: true,
                              data: (value) => ConstrainedBox(
                                constraints: const BoxConstraints(
                                  maxHeight: 360,
                                ),
                                child: value.albums.isEmpty
                                    ? const Padding(
                                        padding: EdgeInsets.symmetric(
                                          vertical: 24,
                                        ),
                                        child: Text("还没有相册，请先新建一个。"),
                                      )
                                    : AmeScrollMotion(
                                        child: SingleChildScrollView(
                                          child: Column(
                                            mainAxisSize: MainAxisSize.min,
                                            children: [
                                              for (final album in value.albums)
                                                CheckboxListTile(
                                                  key: ValueKey(
                                                    "album-choice-${album.id}",
                                                  ),
                                                  value:
                                                      _selected?.id == album.id,
                                                  controlAffinity:
                                                      ListTileControlAffinity
                                                          .leading,
                                                  onChanged: busy
                                                      ? null
                                                      : (checked) => setState(
                                                          () {
                                                            _selected =
                                                                checked == true
                                                                ? album
                                                                : null;
                                                          },
                                                        ),
                                                  title: Text(album.name),
                                                ),
                                            ],
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
                                    onPressed: () => ref.invalidate(
                                      libraryAlbumPageProvider(_after),
                                    ),
                                    child: const Text("重试"),
                                  ),
                                ],
                              ),
                            ),
                            if (page.value case final value?)
                              if (_previous.isNotEmpty ||
                                  value.nextAlbumId != null)
                                Row(
                                  children: [
                                    TextButton(
                                      onPressed:
                                          _previous.isEmpty ||
                                              page.isLoading ||
                                              _saving
                                          ? null
                                          : () => setState(
                                              () => _after = _previous
                                                  .removeLast(),
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
                    ),
                  ),
                ),
                const SizedBox(height: 16),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  alignment: WrapAlignment.end,
                  children: [
                    if (_progress != null) ...[
                      TextButton.icon(
                        key: const Key("library-album-ignore"),
                        onPressed: busy
                            ? null
                            : () => _saveProgress(LibraryProgressAction.ignore),
                        icon: const Icon(Symbols.hide_image_rounded, size: 20),
                        label: Text(widget.ignoreLabel),
                      ),
                      TextButton.icon(
                        key: const Key("library-album-defer"),
                        onPressed: busy
                            ? null
                            : () => _saveProgress(LibraryProgressAction.defer),
                        icon: const Icon(Symbols.schedule_rounded, size: 20),
                        label: const Text("稍后整理"),
                      ),
                    ],
                    TextButton.icon(
                      key: const Key("library-album-create"),
                      onPressed: busy || page.isLoading || page.value == null
                          ? null
                          : () => _create(page.value!.revision),
                      icon: const Icon(Symbols.add_rounded, size: 20),
                      label: const Text("新建相册"),
                    ),
                    if (widget.clearLabel case final label?)
                      TextButton.icon(
                        onPressed: busy
                            ? null
                            : () => _confirm(const LibraryAlbumChoice(null)),
                        icon: Icon(widget.clearIcon, size: 20),
                        label: Text(label),
                      ),
                    FilledButton.icon(
                      key: const Key("library-album-confirm"),
                      onPressed:
                          busy ||
                              _selected == null ||
                              _selected?.id == widget.currentAlbum?.id ||
                              page.isLoading ||
                              page.hasError
                          ? null
                          : () => _confirm(LibraryAlbumChoice(_selected)),
                      icon: Icon(widget.actionIcon, size: 20),
                      label: Text(saveLabel),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
