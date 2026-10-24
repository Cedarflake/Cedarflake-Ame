import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../settings/application/ame_preferences.dart";
import "../../application/library_album_proposal.dart";
import "../../application/library_batch_requests.dart";
import "../../application/library_organization.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_organization_batch.dart";
import "../../application/library_selection_session.dart";
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_organization_batch.dart";
import "../gallery_selection.dart";
import "../library_organization_messages.dart";
import "library_album_picker.dart";

class LibrarySelectionActions extends ConsumerStatefulWidget {
  const LibrarySelectionActions({
    required this.selection,
    required this.query,
    required this.catalogRevision,
    required this.enabled,
    required this.onCommitted,
    super.key,
  });
  final GallerySelection selection;
  final LibraryGalleryQuery query;
  final BigInt catalogRevision;
  final bool enabled;
  final VoidCallback onCommitted;

  @override
  ConsumerState<LibrarySelectionActions> createState() =>
      _LibrarySelectionActionsState();
}

class _LibrarySelectionActionsState
    extends ConsumerState<LibrarySelectionActions> {
  LibrarySelectionSession? _session;
  LibraryBatchFavoriteRequest? _favorite;
  late LibraryOrganizationActions _actions;
  bool _working = false;
  bool _dialogOwnsSession = false;
  Timer? _timer;
  bool _readingProgress = false;
  LibraryBatchProgress? _progress;
  String? _error;

  @override
  void initState() {
    super.initState();
    _actions = ref.read(libraryOrganizationActionsProvider.notifier);
  }

  @override
  void didUpdateWidget(LibrarySelectionActions oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selection != widget.selection &&
        !_dialogOwnsSession &&
        _session?.hasCommitted != true) {
      _retire();
    }
  }

  void _retire() {
    _timer?.cancel();
    _session?.dispose();
    _session = null;
    final request = _favorite;
    if (request != null) {
      request.cancel();
      scheduleMicrotask(() => _actions.retire(request));
    }
    _favorite = null;
    _working = false;
    _progress = null;
  }

  @override
  void dispose() {
    _timer?.cancel();
    if (!_dialogOwnsSession) _retire();
    super.dispose();
  }

  LibrarySelectionSession _start() {
    _retire();
    final selection = widget.selection;
    final session = LibrarySelectionSession(
      ref.read(libraryOrganizationBatchesProvider),
      LibraryOrganizationSelection(
        query: widget.query,
        catalogRevision: widget.catalogRevision,
        allMatching: selection.isAllMatching,
        assetIds: selection.isAllMatching
            ? selection.excludedAssetIds
            : selection.includedAssetIds,
      ),
    );
    setState(() {
      _session = session;
      _working = true;
      _error = null;
    });
    _timer = Timer.periodic(
      const Duration(milliseconds: 250),
      (_) => _poll(session),
    );
    return session;
  }

  bool _owns(LibrarySelectionSession session) =>
      mounted && identical(_session, session);

  Future<void> _poll(LibrarySelectionSession session) async {
    if (_readingProgress || !_owns(session)) return;
    _readingProgress = true;
    try {
      final progress = await session.progress();
      if (_owns(session)) setState(() => _progress = progress);
    } on Object {
      // The command result owns failures; a late progress read cannot replace it.
    } finally {
      _readingProgress = false;
    }
  }

  Future<void> _setFavorite(bool favorite) async {
    if (!mounted || _working || !widget.enabled) return;
    final onCommitted = widget.onCommitted;
    final session = _start();
    final request = LibraryBatchFavoriteRequest(session, favorite);
    _favorite = request;
    final result = await _actions.submit(request);
    if (result is LibraryOrganizationSaved) {
      session.dispose();
      onCommitted();
    }
    if (!_owns(session)) return;
    _timer?.cancel();
    setState(() => _working = false);
    if (result is LibraryOrganizationSaved) {
      _retire();
    }
  }

  Future<void> _album() async {
    if (!mounted || _working || !widget.enabled) return;
    final onCommitted = widget.onCommitted;
    final session = _start();
    final membership = LibraryBatchAlbumMembership(session);
    try {
      final summary = await session.prepare();
      if (!_owns(session)) return;
      final proposal = await resolveLibraryAlbumProposal(
        service: ref.read(libraryOrganizationProvider),
        preferences: ref.read(amePreferencesControllerProvider).albums,
        current: summary.commonAlbum,
        hasConflict: summary.conflicts > 0,
      );
      if (!_owns(session)) return;
      String? initialError;
      if (!proposal.shouldAsk && proposal.destination != null) {
        final result = await _actions.submit(
          membership.request(proposal.destination),
        );
        if (result is LibraryOrganizationSaved) {
          onCommitted();
          return;
        }
        if (!_owns(session)) return;
        if (result case LibraryOrganizationRejected(:final failure)) {
          initialError = libraryOrganizationErrorMessage(failure.code);
        }
      }
      if (!mounted || !_owns(session)) return;
      _dialogOwnsSession = true;
      _timer?.cancel();
      final choice = await showLibraryAlbumPicker(
        context: context,
        membership: membership,
        actionLabel: summary.assigned > 0 ? "移至相册" : "加入相册",
        initialAlbum: proposal.destination,
        clearLabel: summary.assigned > 0 && summary.conflicts == 0
            ? "移出相册"
            : null,
        explanation: summary.conflicts > 0
            ? "所选图片中有相册归属冲突，请选择一个相册。"
            : "所选图片及其重复图片会一起加入所选相册。",
        initialError: initialError,
      );
      if (choice != null) onCommitted();
    } on Object catch (error) {
      if (_owns(session)) {
        setState(
          () => _error = libraryOrganizationErrorMessage(
            error is LibraryOrganizationFailure
                ? error.code
                : "unexpected_error",
          ),
        );
      }
    } finally {
      membership.dispose();
      if (_owns(session)) {
        _dialogOwnsSession = false;
        _retire();
        setState(() {});
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final busy = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    final enabled = widget.enabled && !busy && !_working;
    final progress = _progress;
    var progressText = progress?.phase == LibraryBatchPhase.saving
        ? "正在保存"
        : "正在准备";
    if (progress != null && progress.total > 0) {
      progressText += "（${progress.completed}/${progress.total}）";
    }
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        TextButton.icon(
          key: const Key("library-selection-album"),
          onPressed: enabled ? _album : null,
          icon: const Icon(Symbols.photo_album_rounded),
          label: const Text("加入相册"),
        ),
        AmePopupMenuButton<bool>(
          labels: const ["加入收藏夹", "取消收藏"],
          onSelected: _setFavorite,
          items: const [
            PopupMenuItem(
              value: true,
              child: AmeMenuItemContent(
                label: "加入收藏夹",
                icon: Symbols.favorite_rounded,
              ),
            ),
            PopupMenuItem(
              value: false,
              child: AmeMenuItemContent(
                label: "取消收藏",
                icon: Symbols.favorite_rounded,
              ),
            ),
          ],
          builder: (context, openMenu) => TextButton.icon(
            key: const Key("library-selection-favorite"),
            onPressed: enabled ? openMenu : null,
            icon: const Icon(Symbols.favorite_rounded),
            label: const Text("收藏"),
          ),
        ),
        if (_working) Text(progressText, semanticsLabel: progressText),
        if (_error case final error?)
          Text(
            error,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
      ],
    );
  }
}
