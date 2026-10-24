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
import "../../application/library_organization_requests.dart";
import "../../application/library_selection_session.dart";
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_organization_batch.dart";
import "../../domain/library_review.dart";
import "../../domain/library_selection_availability.dart";
import "../gallery_selection.dart";
import "../library_organization_messages.dart";
import "../library_progress_controls.dart";
import "library_album_picker.dart";
import "library_organization_menu.dart";
import "library_selection_progress.dart";

class LibrarySelectionActions extends ConsumerStatefulWidget {
  const LibrarySelectionActions({
    required this.selection,
    required this.query,
    required this.catalogRevision,
    required this.enabled,
    required this.onCommitted,
    required this.onCancel,
    super.key,
  });
  final GallerySelection selection;
  final LibraryGalleryQuery query;
  final BigInt catalogRevision;
  final bool enabled;
  final VoidCallback onCommitted;
  final VoidCallback onCancel;

  @override
  ConsumerState<LibrarySelectionActions> createState() =>
      _LibrarySelectionActionsState();
}

class _LibrarySelectionActionsState
    extends ConsumerState<LibrarySelectionActions> {
  LibrarySelectionSession? _session;
  LibraryOrganizationRequest? _pendingRequest;
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
    final request = _pendingRequest;
    if (request != null) {
      request.cancel();
      scheduleMicrotask(() => _actions.retire(request));
    }
    _pendingRequest = null;
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

  Future<void> _openFavorites(VoidCallback openMenu) async {
    if (_working || !widget.enabled) return;
    final session = _start();
    try {
      await session.prepare();
      if (!_owns(session)) return;
      _timer?.cancel();
      setState(() => _working = false);
      await WidgetsBinding.instance.endOfFrame;
      if (_owns(session) && widget.enabled) openMenu();
    } on Object catch (error) {
      if (!_owns(session)) return;
      _timer?.cancel();
      setState(() {
        _working = false;
        _error = libraryOrganizationErrorMessage(
          error is LibraryOrganizationFailure ? error.code : "unexpected_error",
        );
      });
    }
  }

  Future<void> _setFavorite(bool favorite) async {
    final session = _session;
    if (session?.summary == null) return;
    await _apply(
      (session) => LibraryBatchFavoriteRequest(session, favorite),
      prepared: session,
    );
  }

  Future<void> _setProgress(LibraryProgressAction action) =>
      _apply((session) => LibraryBatchProgressRequest(session, action));

  Future<void> _apply(
    LibraryOrganizationRequest Function(LibrarySelectionSession) create, {
    LibrarySelectionSession? prepared,
  }) async {
    if (!mounted || _working || !widget.enabled) return;
    final onCommitted = widget.onCommitted;
    final session = prepared ?? _start();
    if (prepared != null) setState(() => _working = true);
    final request = create(session);
    _pendingRequest = request;
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
      final availability = LibrarySelectionAvailability(summary);
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
      setState(() => _dialogOwnsSession = true);
      _timer?.cancel();
      final choice = await showLibraryAlbumPicker(
        context: context,
        membership: membership,
        progressRequestFactory: widget.query.organizationQueue == null
            ? null
            : (action) => LibraryBatchPickerProgressRequest(session, action),
        progressControls: widget.query.organizationQueue == null
            ? null
            : LibraryProgressControls.forQueue(
                widget.query.organizationQueue!,
                hasConflict: summary.conflicts > 0,
              ),
        actionLabel: availability.albumActionLabel,
        actionIcon: summary.assigned == 0
            ? Symbols.bookmark_add_rounded
            : Symbols.drive_file_move_rounded,
        initialAlbum: proposal.destination,
        currentAlbum: summary.commonAlbum,
        clearLabel: availability.canRemoveAlbum ? "移出相册" : null,
        explanation: summary.conflicts > 0 ? "所选图片中有相册归属冲突，请选择一个相册。" : null,
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
    final summary = _session?.summary;
    final availability = summary == null
        ? null
        : LibrarySelectionAvailability(summary);
    final queue = widget.query.organizationQueue;
    final controls = queue == null
        ? null
        : LibraryProgressControls.forQueue(
            queue,
            hasConflict: (summary?.conflicts ?? 0) > 0,
          );
    return Stack(
      alignment: Alignment.bottomCenter,
      children: [
        Row(
          key: const Key("library-selection-toolbar"),
          mainAxisSize: MainAxisSize.min,
          children: [
            AmePopupMenuButton<bool>(
              labels: const ["加入收藏夹", "取消收藏"],
              onSelected: _setFavorite,
              onOpenChanged: (open) {
                if (!open && !_working) _retire();
              },
              items: [
                if (availability?.canFavorite == true)
                  const PopupMenuItem(
                    value: true,
                    child: AmeMenuItemContent(
                      label: "加入收藏夹",
                      icon: Symbols.favorite_rounded,
                    ),
                  ),
                if (availability?.canUnfavorite == true)
                  const PopupMenuItem(
                    value: false,
                    child: AmeMenuItemContent(
                      label: "取消收藏",
                      icon: Symbols.heart_minus_rounded,
                    ),
                  ),
              ],
              builder: (context, openMenu) => TextButton.icon(
                key: const Key("library-selection-favorite"),
                onPressed: enabled ? () => _openFavorites(openMenu) : null,
                icon: const Icon(Symbols.favorite_rounded),
                label: const Text("收藏"),
              ),
            ),
            TextButton.icon(
              key: const Key("library-selection-album"),
              onPressed: enabled ? _album : null,
              icon: const Icon(Symbols.photo_album_rounded),
              label: const Text("相册"),
            ),
            if (controls case final controls?)
              TextButton.icon(
                key: const Key("library-selection-defer"),
                onPressed: enabled && controls.defer.enabled
                    ? () => _setProgress(controls.defer.action)
                    : null,
                icon: Icon(controls.defer.icon),
                label: Text(controls.defer.label),
              ),
            if (_error case final error?)
              Text(
                error,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            TextButton.icon(
              key: const Key("library-cancel-selection"),
              onPressed: widget.onCancel,
              icon: const Icon(Symbols.close_rounded),
              label: const Text("取消"),
            ),
            if (controls case final controls?)
              LibraryOrganizationMenu(
                key: ValueKey(widget.selection),
                enabled: enabled,
                progress: controls.ignore,
                onProgress: _setProgress,
              ),
          ],
        ),
        if (_working && !_dialogOwnsSession)
          Positioned(
            left: 0,
            right: 0,
            bottom: 0,
            child: LibrarySelectionProgress(
              key: ObjectKey(_session),
              progress: _progress,
            ),
          ),
      ],
    );
  }
}
