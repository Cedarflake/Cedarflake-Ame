import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../application/library_controller.dart";
import "../../application/library_duplicate_review.dart";
import "../../application/library_duplicate_review_controller.dart";
import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../../domain/library_exact_duplicates.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_review_cards.dart";
import "library_duplicate_review_grid.dart";
import "library_duplicate_review_header.dart";
import "library_duplicate_review_viewer.dart";
import "library_exact_analysis_actions.dart";
import "library_gallery_loading_region.dart";
import "library_gallery_message.dart";
import "library_path_text.dart";
import "library_viewer_fade.dart";

class LibraryExactDuplicatesPage extends ConsumerStatefulWidget {
  const LibraryExactDuplicatesPage({super.key});
  @override
  ConsumerState<LibraryExactDuplicatesPage> createState() =>
      _LibraryExactDuplicatesPageState();
}

class _LibraryExactDuplicatesPageState
    extends ConsumerState<LibraryExactDuplicatesPage> {
  late final LibraryExactAnalysisController _analysis;
  late final LibraryDuplicateReviewController _review;
  LibraryExactAnalysisScheduler? _scheduler;
  late bool _wasRunning;
  bool _viewerOpen = false;

  @override
  void initState() {
    super.initState();
    _analysis = ref.read(libraryExactAnalysisControllerProvider);
    _scheduler = ref.read(libraryExactAnalysisSchedulerProvider);
    _review = LibraryDuplicateReviewController(
      ref.read(libraryDuplicateReviewProvider),
    );
    _wasRunning = _analysis.isRunning;
    _analysis.addListener(_analysisChanged);
    ref.listenManual(
      libraryControllerProvider.select((value) => value.catalogRevision),
      (_, _) {
        unawaited(_review.catalogChanged());
      },
    );
    unawaited(_review.refresh());
  }

  void _analysisChanged() {
    if (_wasRunning && !_analysis.isRunning) {
      unawaited(_review.catalogChanged());
    }
    _wasRunning = _analysis.isRunning;
  }

  @override
  void dispose() {
    _analysis.removeListener(_analysisChanged);
    _review.dispose();
    super.dispose();
  }

  Future<void> _prefer(LibraryAsset? asset) async {
    if (await _review.prefer(asset) && mounted) {
      await ref.read(libraryControllerProvider.notifier).refreshCurrentQuery();
    }
  }

  Future<void> _ignore() async {
    final selected = _review.selected;
    if (await _review.setIgnored(!_review.decision.ignored) &&
        mounted &&
        identical(_review.selected, selected)) {
      await _review.showGroups();
    }
  }

  Future<void> _open(LibraryAsset asset, int index) async {
    if (_viewerOpen) return;
    final group = _review.selected;
    setState(() => _viewerOpen = true);
    try {
      await Navigator.of(context).push<void>(
        LibraryViewerRoute(
          context: context,
          builder: (context) => Scaffold(
            body: LibraryDuplicateReviewViewer(
              initialAsset: asset,
              totalCount: _review.itemCount,
              loadAsset: (index) async {
                await _review.memberWindow.retryRange(index, index + 1);
                if (!identical(_review.selected, group)) return null;
                return _review.memberWindow.itemAt(index)?.asset;
              },
              initialIndex: index,
              onBack: () => Navigator.of(context).pop(),
            ),
          ),
        ),
      );
    } finally {
      if (mounted) setState(() => _viewerOpen = false);
    }
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: Listenable.merge([_analysis, _review, _scheduler]),
    builder: (context, child) {
      final selected = _review.selected;
      final title = selected == null
          ? LibraryDuplicateStrings.workspaceTitle
          : displayLibraryFileName(
              selected.representative.displayPath.replaceAll("/", "\\"),
            );
      final countVisible =
          !_review.loadFailed &&
          (!_review.loading || _review.groupWindow.total > 0);
      return TickerMode(
        enabled: !_viewerOpen,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(28, 24, 28, 16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              LibraryDuplicateReviewHeader(
                title: title,
                totalGroups: countVisible ? _review.groupWindow.total : null,
                ignored: _review.ignored,
                onCategoryChanged: (value) =>
                    unawaited(_review.showGroups(ignoredGroups: value)),
                onBack: selected == null
                    ? null
                    : () => unawaited(_review.showGroups()),
                actions: LibraryExactAnalysisActions(
                  controller: _analysis,
                  actions: [
                    TextButton(
                      onPressed: _review.loading || _review.saving
                          ? null
                          : () => unawaited(_review.refresh()),
                      child: const Text(LibraryDuplicateStrings.refresh),
                    ),
                    if (selected != null)
                      TextButton(
                        onPressed:
                            _review.saving ||
                                _review.loading ||
                                _review.loadFailed
                            ? null
                            : () => unawaited(_ignore()),
                        child: Text(
                          _review.decision.ignored
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
                    left: selected == null
                        ? 0
                        : LibraryDuplicateReviewHeader.backInset,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (_review.saveFailed)
                        Text(
                          LibraryDuplicateStrings.saveFailed,
                          style: TextStyle(
                            color: Theme.of(context).colorScheme.error,
                          ),
                        ),
                      Expanded(
                        child: LibraryGalleryLoadingRegion(
                          isLoading: _review.loading,
                          child: Padding(
                            padding: const EdgeInsets.only(top: 12),
                            child: _review.loadFailed
                                ? LibraryGalleryMessage(
                                    icon: Symbols.error_outline_rounded,
                                    title: LibraryDuplicateStrings.loadFailed,
                                    action: TextButton(
                                      onPressed: () =>
                                          unawaited(_review.refresh()),
                                      child: const Text(
                                        LibraryDuplicateStrings.retry,
                                      ),
                                    ),
                                  )
                                : _results(),
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

  Widget _results() {
    final selected = _review.selected;
    if (_review.itemCount == 0) {
      if (_review.loading) return const SizedBox.shrink();
      final String message;
      if (selected != null) {
        message = LibraryDuplicateStrings.emptyLocations;
      } else if (_review.ignored) {
        message = LibraryDuplicateStrings.noIgnoredGroups;
      } else if (_analysis.isRunning) {
        message = LibraryDuplicateStrings.waitingForResults;
      } else {
        message = LibraryDuplicateStrings.emptyAfterAnalysis;
      }
      return LibraryGalleryMessage(
        icon: Symbols.photo_library_rounded,
        title: message,
      );
    }
    return LibraryDuplicateReviewGrid(
      key: ValueKey(_review.pageKey),
      itemCount: _review.itemCount,
      singleColumn: selected != null,
      onVisibleRange: _review.ensureRange,
      assetAt: (index) => selected == null
          ? _review.groupWindow.itemAt(index)?.representative
          : _review.memberWindow.itemAt(index)?.asset,
      itemBuilder: (context, index) {
        if (selected == null) {
          final entry = _review.groupWindow.itemAt(index);
          if (entry == null) return const Card(margin: EdgeInsets.zero);
          return LibraryDuplicateGroupCard(
            entry: entry,
            onOpen: () => unawaited(_review.showGroup(entry)),
          );
        }
        final member = _review.memberWindow.itemAt(index);
        if (member == null) return const Card(margin: EdgeInsets.zero);
        final preferred = _review.decision.prefers(member.asset);
        return LibraryDuplicateMemberCard(
          member: member,
          preferred: preferred,
          onOpen: () => unawaited(_open(member.asset, index)),
          onPrefer:
              _review.saving ||
                  _review.loading ||
                  (!preferred &&
                      member.availability != LibraryDuplicateAvailability.local)
              ? null
              : () => unawaited(_prefer(preferred ? null : member.asset)),
        );
      },
    );
  }
}
