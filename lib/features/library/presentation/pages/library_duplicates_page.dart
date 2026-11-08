import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../application/library_controller.dart";
import "../../application/library_duplicate_review.dart";
import "../../application/library_duplicate_review_controller.dart";
import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_gallery_duplicates.dart";
import "../../application/library_similar_analysis_controller.dart";
import "../../application/library_similar_review_controller.dart";
import "../../application/library_workbench_controller.dart";
import "../../domain/library_models.dart";
import "../../domain/library_similar_review.dart";
import "../coordination/library_duplicate_workspace.dart";
import "../coordination/library_similar_results_refresh.dart";
import "../library_duplicate_strings.dart";
import "../widgets/library_duplicate_group_menus.dart";
import "../widgets/library_duplicate_locations_sheet.dart";
import "../widgets/library_duplicate_results.dart";
import "../widgets/library_duplicate_review_header.dart";
import "../widgets/library_duplicate_review_viewer.dart";
import "../widgets/library_gallery_loading_region.dart";
import "../widgets/library_gallery_message.dart";
import "../widgets/library_image_comparison.dart";
import "../widgets/library_path_text.dart";
import "../widgets/library_viewer_fade.dart";

class LibraryDuplicatesPage extends ConsumerStatefulWidget {
  const LibraryDuplicatesPage({super.key});
  @override
  ConsumerState<LibraryDuplicatesPage> createState() =>
      _LibraryDuplicatesPageState();
}

class _LibraryDuplicatesPageState extends ConsumerState<LibraryDuplicatesPage> {
  late final LibraryExactAnalysisController _exactAnalysis;
  late final LibrarySimilarAnalysisController _similarAnalysis;
  late final LibraryDuplicateWorkspace _workspace;
  late final LibrarySimilarResultsRefresh _similarResultsRefresh;
  bool _exactRunning = false;
  bool _similarRunning = false;
  bool _viewerOpen = false;
  bool _catalogPending = false;

  @override
  void initState() {
    super.initState();
    _exactAnalysis = ref.read(libraryExactAnalysisControllerProvider);
    _similarAnalysis = ref.read(librarySimilarAnalysisControllerProvider);
    _workspace = LibraryDuplicateWorkspace(
      exact: LibraryDuplicateReviewController(
        ref.read(libraryDuplicateReviewProvider),
      ),
      similar: LibrarySimilarReviewController(
        ref.read(librarySimilarReviewProvider),
      ),
    );
    _exactRunning = _exactAnalysis.isRunning;
    _similarRunning = _similarAnalysis.isRunning;
    _exactAnalysis.addListener(_analysisChanged);
    _similarAnalysis.addListener(_analysisChanged);
    _similarResultsRefresh = LibrarySimilarResultsRefresh(
      analysis: _similarAnalysis,
      refresh: _refreshSimilarResults,
    );
    ref.listenManual(
      libraryControllerProvider.select((value) => value.catalogRevision),
      (_, _) => _catalogChanged(),
    );
    ref.listenManual(libraryWorkbenchControllerProvider, (previous, next) {
      if (previous == LibraryWorkspace.duplicates &&
          next != LibraryWorkspace.duplicates) {
        unawaited(_workspace.showView(_workspace.view));
      }
    });
    unawaited(_workspace.showView(LibraryDuplicateView.exact));
  }

  void _analysisChanged() {
    if ((_exactRunning && !_exactAnalysis.isRunning) ||
        (_similarRunning && !_similarAnalysis.isRunning)) {
      _catalogChanged();
    }
    _exactRunning = _exactAnalysis.isRunning;
    _similarRunning = _similarAnalysis.isRunning;
  }

  void _catalogChanged() {
    if (_viewerOpen) {
      _catalogPending = true;
    } else {
      unawaited(_workspace.catalogChanged());
    }
  }

  Future<void> _refreshSimilarResults() {
    if (_viewerOpen ||
        _workspace.hasMembers ||
        _workspace.view == LibraryDuplicateView.exact ||
        _workspace.similar.groupWindow.isReading ||
        ref.read(libraryWorkbenchControllerProvider) !=
            LibraryWorkspace.duplicates) {
      return Future.value();
    }
    return _workspace.similar.catalogChanged();
  }

  @override
  void dispose() {
    _similarResultsRefresh.dispose();
    _exactAnalysis.removeListener(_analysisChanged);
    _similarAnalysis.removeListener(_analysisChanged);
    _workspace.dispose();
    super.dispose();
  }

  Future<void> _preferExact(LibraryAsset? asset) async {
    if (await _workspace.exact.prefer(asset) && mounted) {
      await ref.read(libraryControllerProvider.notifier).refreshCurrentQuery();
    }
  }

  Future<void> _ignore() => _workspace.isSimilar
      ? _workspace.similar.setIgnored(!_workspace.ignored)
      : _workspace.exact.setIgnored(!_workspace.ignored);

  Future<void> _locations(LibrarySimilarMember member) async {
    try {
      final revision = ref.read(libraryControllerProvider).catalogRevision;
      final service = ref.read(libraryGalleryDuplicatesProvider);
      if (revision == null || service == null) {
        throw StateError("Catalog unavailable");
      }
      final groups = await service.load(revision, [member.asset.locationId]);
      if (!mounted) return;
      final group = groups[member.asset.locationId];
      if (group == null) throw StateError("Duplicate locations changed");
      await showModalBottomSheet<void>(
        context: context,
        showDragHandle: true,
        isScrollControlled: true,
        useSafeArea: true,
        builder: (_) => LibraryDuplicateLocationsSheet(
          group: group,
          currentLocationId: member.asset.locationId,
        ),
      );
    } on Object {
      if (mounted) {
        showAmeTransientFeedback(
          context,
          message: LibraryDuplicateStrings.loadFailed,
          severity: AmeNotificationSeverity.error,
          action: (
            label: LibraryDuplicateStrings.retry,
            invoke: () => unawaited(_locations(member)),
          ),
        );
      }
    }
  }

  Future<void> _view(WidgetBuilder builder) async {
    if (_viewerOpen) return;
    _workspace.exact.memberWindow.cancelRead();
    _workspace.similar.memberWindow.cancelRead();
    setState(() => _viewerOpen = true);
    try {
      await Navigator.of(
        context,
      ).push<void>(LibraryViewerRoute(context: context, builder: builder));
    } finally {
      if (mounted) {
        setState(() => _viewerOpen = false);
        final changed = _catalogPending;
        _catalogPending = false;
        await _workspace.catalogChanged(keepComparison: !changed);
      }
    }
  }

  Future<void> _open(LibraryAsset asset, int index) {
    final pageKey = _workspace.pageKey;
    return _view(
      (context) => Scaffold(
        body: LibraryDuplicateReviewViewer(
          initialAsset: asset,
          initialIndex: index,
          totalCount: _workspace.itemCount,
          loadAsset: (index) async {
            if (_workspace.isSimilar) {
              await _workspace.similar.memberWindow.retryRange(
                index,
                index + 1,
              );
            } else {
              await _workspace.exact.memberWindow.retryRange(index, index + 1);
            }
            return _workspace.pageKey == pageKey
                ? _workspace.assetAt(index)
                : null;
          },
          onBack: () => Navigator.of(context).pop(),
        ),
      ),
    );
  }

  Future<void> _compare() {
    final selected = _workspace.comparison;
    if (selected.length != 2) return Future.value();
    return _view(
      (context) => LibraryImageComparison(
        left: selected.first,
        right: selected.last,
        onBack: () => Navigator.of(context).pop(),
      ),
    );
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: Listenable.merge([
      _workspace,
      _exactAnalysis,
      _similarAnalysis,
    ]),
    builder: (context, child) {
      final representative = _workspace.representative;
      final title = representative == null
          ? switch (_workspace.view) {
              LibraryDuplicateView.exact =>
                LibraryDuplicateStrings.workspaceTitle,
              LibraryDuplicateView.ignored =>
                LibraryDuplicateStrings.ignoredGroups,
              LibraryDuplicateView.similar =>
                LibraryDuplicateStrings.similarGroups,
            }
          : displayLibraryFileName(representative.displayPath);
      return TickerMode(
        enabled: !_viewerOpen,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(28, 24, 28, 16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              LibraryDuplicateReviewHeader(
                title: title,
                totalGroups:
                    _workspace.loadFailed ||
                        (_workspace.loading && _workspace.groupCount == 0)
                    ? null
                    : _workspace.groupCount,
                onBack: !_workspace.hasMembers
                    ? null
                    : () => unawaited(_workspace.showView(_workspace.view)),
                actions: !_workspace.hasMembers
                    ? LibraryDuplicateGroupMenus(
                        key: const Key("duplicate-review-actions"),
                        view: _workspace.view,
                        degree: _workspace.similar.degree,
                        onViewChanged: (view) =>
                            unawaited(_workspace.showView(view)),
                        onDegreeChanged: (degree) => unawaited(
                          _workspace.showView(
                            LibraryDuplicateView.similar,
                            degree: degree,
                          ),
                        ),
                      )
                    : Wrap(
                        key: const Key("duplicate-review-actions"),
                        alignment: WrapAlignment.end,
                        spacing: 8,
                        children: [
                          TextButton.icon(
                            onPressed: _workspace.comparison.length == 2
                                ? () => unawaited(_compare())
                                : null,
                            icon: const Icon(Symbols.compare_rounded),
                            label: const Text(LibraryDuplicateStrings.compare),
                          ),
                          TextButton.icon(
                            onPressed:
                                _workspace.saving ||
                                    _workspace.loading ||
                                    _workspace.loadFailed
                                ? null
                                : () => unawaited(_ignore()),
                            icon: Icon(
                              _workspace.ignored
                                  ? Symbols.visibility_rounded
                                  : Symbols.visibility_off_rounded,
                            ),
                            label: Text(
                              _workspace.ignored
                                  ? LibraryDuplicateStrings.restoreGroup
                                  : LibraryDuplicateStrings.ignore,
                            ),
                          ),
                        ],
                      ),
              ),
              const SizedBox(height: 12),
              Expanded(
                child: Padding(
                  padding: EdgeInsets.only(
                    left: _workspace.hasMembers
                        ? LibraryDuplicateReviewHeader.backInset
                        : 0,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (_workspace.saveFailed)
                        Text(
                          LibraryDuplicateStrings.saveFailed,
                          style: TextStyle(
                            color: Theme.of(context).colorScheme.error,
                          ),
                        ),
                      Expanded(
                        child: LibraryGalleryLoadingRegion(
                          isLoading: _workspace.loading,
                          child: Padding(
                            padding: const EdgeInsets.only(top: 12),
                            child: _workspace.loadFailed
                                ? LibraryGalleryMessage(
                                    icon: Symbols.error_outline_rounded,
                                    title: LibraryDuplicateStrings.loadFailed,
                                    action: TextButton(
                                      onPressed: () =>
                                          unawaited(_workspace.refresh()),
                                      child: const Text(
                                        LibraryDuplicateStrings.retry,
                                      ),
                                    ),
                                  )
                                : LibraryDuplicateResults(
                                    workspace: _workspace,
                                    analyzing:
                                        _workspace.view ==
                                            LibraryDuplicateView.similar
                                        ? _similarAnalysis.isRunning
                                        : _exactAnalysis.isRunning,
                                    onOpen: (asset, index) =>
                                        unawaited(_open(asset, index)),
                                    onPreferExact: (asset) =>
                                        unawaited(_preferExact(asset)),
                                    onLocations: (member) =>
                                        unawaited(_locations(member)),
                                  ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      );
    },
  );
}
