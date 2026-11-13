import "dart:async";
import "dart:math" as math;

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_controller.dart";
import "../../application/library_duplicate_review_window.dart";
import "../../application/library_group_members.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_group_members.dart";
import "../../domain/library_gallery_group.dart";
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_organization_batch.dart";
import "../../domain/library_file_operations.dart";
import "../../../../app/notifications/ame_notification_controller.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_location_tile.dart";
import "library_duplicate_review_viewer.dart";
import "library_duplicate_update_status.dart";
import "library_file_action_dialog.dart";
import "library_viewer_fade.dart";

class LibraryDuplicateLocationsSheet extends ConsumerStatefulWidget {
  const LibraryDuplicateLocationsSheet({
    this.group,
    this.galleryGroup,
    this.query,
    required this.currentLocationId,
    super.key,
  }) : assert(group != null || (galleryGroup != null && query != null));

  final LibraryDuplicateGroup? group;
  final LibraryGalleryGroup? galleryGroup;
  final LibraryGalleryQuery? query;
  final String currentLocationId;

  @override
  ConsumerState<LibraryDuplicateLocationsSheet> createState() =>
      _LibraryDuplicateLocationsSheetState();
}

class _LibraryDuplicateLocationsSheetState
    extends ConsumerState<LibraryDuplicateLocationsSheet> {
  late LibraryDuplicateReviewWindow<LibraryGroupMember, String> _locations;
  int? _firstBuilt;
  int _endBuilt = 0;
  int _pendingCount = 0;
  BigInt _decisionRevision = BigInt.zero;
  bool _saving = false;
  final Map<String, LibraryOrganizationTarget> _selected = {};

  LibraryGalleryGroupKey get _key =>
      widget.galleryGroup?.key ??
      LibraryExactGalleryGroupKey(
        widget.group!.digest,
        BigInt.from(widget.group!.byteLength),
      );

  @override
  void initState() {
    super.initState();
    _createWindow();
    ref.listenManual(
      libraryControllerProvider.select((state) => state.catalogRevision),
      (_, _) => unawaited(_locations.revalidate()),
    );
  }

  void _createWindow() {
    final service = ref.read(libraryGroupMembersProvider);
    final group = widget.group;
    _pendingCount = group?.pendingLocationCount ?? 0;
    _locations = LibraryDuplicateReviewWindow(
      (after) async {
        final page = await service.load(
          key: _key,
          query: widget.query,
          after: after,
        );
        return _LocationSlice(page);
      },
      onAdmitted: (slice) {
        if (slice is _LocationSlice) {
          _decisionRevision = slice.revision;
          _pendingCount = slice.pending;
        }
      },
    )..total = widget.galleryGroup?.memberCount ?? group!.locationCount;
    unawaited(_locations.ensureRange(0, 1));
  }

  @override
  void didUpdateWidget(covariant LibraryDuplicateLocationsSheet oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.group?.digest != oldWidget.group?.digest ||
        widget.group?.byteLength != oldWidget.group?.byteLength ||
        widget.galleryGroup?.key != oldWidget.galleryGroup?.key ||
        widget.query != oldWidget.query) {
      _selected.clear();
      _locations.dispose();
      _createWindow();
    }
  }

  // Variable-height paths let the framework's lazy list report its demand.
  void _requestBuiltRange(int index) {
    final first = _firstBuilt;
    _firstBuilt = first == null ? index : math.min(first, index);
    _endBuilt = math.max(_endBuilt, index + 1);
    if (first != null) return;
    final window = _locations;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final start = _firstBuilt;
      final end = _endBuilt;
      _firstBuilt = null;
      _endBuilt = 0;
      if (mounted && identical(window, _locations) && start != null) {
        unawaited(_locations.ensureRange(start, end));
      }
    });
  }

  @override
  void dispose() {
    _locations.dispose();
    super.dispose();
  }

  Future<void> _prefer(LibraryGroupMember member) async {
    setState(() => _saving = true);
    try {
      await ref
          .read(libraryGroupMembersProvider)
          .prefer(key: _key, member: member, revision: _decisionRevision);
      await _locations.revalidate();
      if (mounted) {
        unawaited(
          ref.read(libraryControllerProvider.notifier).refreshCurrentQuery(),
        );
      }
    } on Object {
      if (mounted) {
        ref
            .read(ameNotificationControllerProvider.notifier)
            .publish(
              const AmeNotificationDraft(
                title: "未能保存选择",
                message: "请重试。",
                severity: AmeNotificationSeverity.error,
                isPersistent: true,
              ),
            );
      }
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Future<void> _deleteSelected() async {
    await _delete(LibraryFilesAtLocations(_selected.values));
  }

  Future<void> _open(LibraryGroupMember member, int index) async {
    final window = _locations;
    await Navigator.of(context).push<void>(
      LibraryViewerRoute(
        context: context,
        builder: (context) => Scaffold(
          backgroundColor: Colors.transparent,
          body: LibraryDuplicateReviewViewer(
            initialAsset: member.asset,
            initialIndex: index,
            totalCount: window.total,
            loadAsset: (index) async {
              await window.retryRange(index, index + 1);
              return mounted && identical(window, _locations)
                  ? window.itemAt(index)?.asset
                  : null;
            },
            onBack: () => Navigator.of(context).pop(),
          ),
        ),
      ),
    );
  }

  Future<void> _cleanup() async {
    final query = widget.query;
    final key = _key;
    final selection = query != null
        ? LibraryCleanupInGallery(
            LibraryOrganizationSelection(
              query: query,
              catalogRevision:
                  ref.read(libraryControllerProvider).catalogRevision ??
                  BigInt.zero,
              allMatching: false,
              assetIds: [key.entryId],
            ),
          )
        : LibraryDuplicateCleanup(
            kind: LibraryCleanupKind.exact,
            allGroups: false,
            groups: [
              LibraryExactCleanupGroup(
                widget.group!.digest,
                widget.group!.byteLength,
              ),
            ],
          );
    await _delete(selection);
  }

  Future<void> _delete(LibraryFileSelection selection) async {
    final submission = await showLibraryFileAction(
      context: context,
      ref: ref,
      selection: selection,
      action: LibraryFileAction.recycle,
    );
    final result = await submission?.completion;
    if (mounted && (result?.published ?? 0) > 0) {
      setState(_selected.clear);
      unawaited(_locations.revalidate());
    }
  }

  @override
  Widget build(BuildContext context) => SafeArea(
    top: false,
    child: SizedBox(
      key: const Key("duplicate-locations-sheet"),
      height: MediaQuery.sizeOf(context).height * 0.72,
      child: ListenableBuilder(
        listenable: _locations,
        builder: (context, _) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 8, 24, 16),
              child: LibraryDuplicateUpdateStatus(
                hasPending: _pendingCount > 0,
                builder: (context, label) => Wrap(
                  alignment: WrapAlignment.spaceBetween,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  spacing: 16,
                  runSpacing: 4,
                  children: [
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          _key is LibrarySimilarGalleryGroupKey
                              ? "${_locations.total} 张相似图片"
                              : LibraryDuplicateStrings.duplicateImages(
                                  _locations.total,
                                ),
                          style: Theme.of(context).textTheme.titleLarge,
                        ),
                        const SizedBox(width: 12),
                        AmeLoadingProgress(
                          active:
                              _locations.loading &&
                              _locations.cachedItemCount > 0,
                          size: 24,
                          semanticsLabel: "正在更新文件位置",
                        ),
                      ],
                    ),
                    if (label != null)
                      Text(
                        label,
                        style: Theme.of(context).textTheme.bodyMedium,
                      ),
                    Wrap(
                      spacing: 4,
                      crossAxisAlignment: WrapCrossAlignment.center,
                      children: [
                        TextButton.icon(
                          onPressed:
                              _saving ||
                                  _locations.total < 2 ||
                                  ref.watch(libraryFileActionsProvider).busy
                              ? null
                              : _cleanup,
                          icon: const Icon(Symbols.delete_rounded),
                          label: Text(
                            _key is LibrarySimilarGalleryGroupKey
                                ? "删除相似图片"
                                : "删除重复图片",
                          ),
                        ),
                        TextButton.icon(
                          onPressed:
                              _saving ||
                                  _selected.isEmpty ||
                                  ref.watch(libraryFileActionsProvider).busy
                              ? null
                              : _deleteSelected,
                          icon: const Icon(Symbols.delete_rounded),
                          label: const Text("删除所选"),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
            if (_locations.failed)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 24),
                child: Row(
                  children: [
                    const Expanded(
                      child: Text(LibraryDuplicateStrings.loadFailed),
                    ),
                    TextButton(
                      onPressed: _locations.revalidate,
                      child: const Text(LibraryDuplicateStrings.retry),
                    ),
                  ],
                ),
              ),
            Expanded(
              child: AmeLoadingContent(
                isLoading:
                    _locations.loading && _locations.cachedItemCount == 0,
                semanticsLabel: "正在读取文件位置",
                child: _buildLocations(),
              ),
            ),
          ],
        ),
      ),
    ),
  );

  Widget _buildLocations() {
    if (_locations.total == 0 && !_locations.loading && !_locations.failed) {
      return const Center(child: Text(LibraryDuplicateStrings.emptyLocations));
    }
    return AmeScrollMotion(
      child: ListView.builder(
        key: ValueKey(_key),
        itemCount: _locations.total,
        padding: const EdgeInsets.fromLTRB(24, 0, 24, 16),
        itemBuilder: (context, index) {
          _requestBuiltRange(index);
          final location = _locations.itemAt(index);
          if (location == null) {
            return SizedBox(
              height: 32 + MediaQuery.textScalerOf(context).scale(56),
            );
          }
          return LibraryDuplicateLocationTile(
            key: ValueKey(location.asset.locationId),
            location: location.location,
            onOpen: () => unawaited(_open(location, index)),
            selected: _selected.containsKey(location.asset.locationId),
            onSelect: () => setState(() {
              final id = location.asset.locationId;
              if (_selected.containsKey(id)) {
                _selected.remove(id);
              } else {
                _selected[id] = LibraryOrganizationTarget.fromAsset(
                  location.asset,
                );
              }
            }),
            preferred: location.preferred,
            onPreference: _saving ? null : () => _prefer(location),
            preferenceLabel: _key is LibrarySimilarGalleryGroupKey
                ? (location.preferred
                      ? LibraryDuplicateStrings.deselectSimilarImage
                      : LibraryDuplicateStrings.selectSimilarImage)
                : (location.preferred ? "取消首选" : "设为首选"),
            wrapPath: true,
          );
        },
      ),
    );
  }
}

class _LocationSlice
    extends LibraryDuplicateReviewSlice<LibraryGroupMember, String> {
  _LocationSlice(LibraryGroupMemberPage page)
    : revision = page.revision,
      pending = page.pending,
      super(items: page.members, total: page.total, next: page.next);

  final BigInt revision;
  final int pending;
}
