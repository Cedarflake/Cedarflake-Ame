import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_controller.dart";
import "../../application/library_duplicate_review.dart";
import "../../application/library_duplicate_review_controller.dart";
import "../../application/library_exact_analysis_controller.dart";
import "../../application/library_exact_analysis_scheduler.dart";
import "../../domain/library_exact_duplicates.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";
import "../library_workbench_strings.dart";
import "library_duplicate_review_list.dart";
import "library_duplicate_review_cards.dart";
import "library_duplicate_review_viewer.dart";
import "library_exact_analysis_status.dart";

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
    if (_wasRunning && !_analysis.isRunning) unawaited(_review.refresh());
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
    if (await _review.setIgnored(!_review.members.decision.ignored) &&
        mounted &&
        identical(_review.selected, selected)) {
      await _review.showGroups();
    }
  }

  Future<void> _open(LibraryAsset asset) async {
    if (_viewerOpen) return;
    final assets = _review.members.locations
        .map((member) => member.asset)
        .toList(growable: false);
    final index = assets.indexWhere(
      (item) => item.locationId == asset.locationId,
    );
    if (index < 0) return;
    setState(() => _viewerOpen = true);
    try {
      await Navigator.of(context).push<void>(
        MaterialPageRoute(
          builder: (context) => Scaffold(
            body: LibraryDuplicateReviewViewer(
              assets: assets,
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
  Widget build(BuildContext context) {
    final roots = ref.watch(
      libraryControllerProvider.select((value) => value.roots.length),
    );
    return ListenableBuilder(
      listenable: Listenable.merge([_analysis, _review, _scheduler]),
      builder: (context, child) {
        return TickerMode(
          enabled: !_viewerOpen,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(28, 24, 28, 16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Row(
                  children: [
                    if (_review.selected != null)
                      AmeTooltip(
                        message: LibraryDuplicateStrings.back,
                        child: IconButton(
                          onPressed: () => unawaited(_review.showGroups()),
                          icon: const Icon(Symbols.arrow_back_rounded),
                        ),
                      ),
                    Expanded(
                      child: Text(
                        _review.selected == null
                            ? LibraryWorkbenchStrings.duplicates
                            : LibraryDuplicateStrings.review,
                        style: Theme.of(context).textTheme.headlineSmall,
                      ),
                    ),
                    if (_review.selected != null)
                      TextButton(
                        onPressed:
                            _review.saving ||
                                _review.loading ||
                                _review.loadFailed
                            ? null
                            : () => unawaited(_ignore()),
                        child: Text(
                          _review.members.decision.ignored
                              ? LibraryDuplicateStrings.restoreGroup
                              : LibraryDuplicateStrings.ignore,
                        ),
                      ),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  _review.selected == null
                      ? LibraryDuplicateStrings.exactDescription
                      : LibraryDuplicateStrings.preferDescription,
                ),
                Text(
                  LibraryDuplicateStrings.scope(roots),
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 12),
                LibraryExactAnalysisStatus(controller: _analysis),
                const SizedBox(height: 8),
                if (_review.selected == null)
                  Align(
                    alignment: Alignment.centerLeft,
                    child: SegmentedButton<bool>(
                      segments: const [
                        ButtonSegment(
                          value: false,
                          label: Text(LibraryDuplicateStrings.pendingGroups),
                        ),
                        ButtonSegment(
                          value: true,
                          label: Text(LibraryDuplicateStrings.ignoredGroups),
                        ),
                      ],
                      selected: {_review.ignored},
                      onSelectionChanged: (value) => unawaited(
                        _review.showGroups(ignoredGroups: value.single),
                      ),
                    ),
                  ),
                if (_review.saveFailed)
                  Text(
                    LibraryDuplicateStrings.saveFailed,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                SizedBox(
                  height: 6,
                  child: _review.loading || _review.saving
                      ? const LinearProgressIndicator()
                      : null,
                ),
                Expanded(
                  child: _review.loadFailed
                      ? Center(
                          child: Column(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              const Text(LibraryDuplicateStrings.loadFailed),
                              TextButton(
                                onPressed: () => unawaited(_review.refresh()),
                                child: const Text(
                                  LibraryDuplicateStrings.retry,
                                ),
                              ),
                            ],
                          ),
                        )
                      : _results(),
                ),
                Row(
                  children: [
                    TextButton(
                      onPressed: _review.loading
                          ? null
                          : () => unawaited(_review.refresh()),
                      child: const Text(LibraryDuplicateStrings.refresh),
                    ),
                    const Spacer(),
                    TextButton(
                      onPressed:
                          _review.loading ||
                              _review.saving ||
                              !_review.canPrevious
                          ? null
                          : () => unawaited(_review.previous()),
                      child: const Text(LibraryDuplicateStrings.previous),
                    ),
                    TextButton(
                      onPressed:
                          _review.loading || _review.saving || !_review.canNext
                          ? null
                          : () => unawaited(_review.next()),
                      child: const Text(LibraryDuplicateStrings.next),
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  Widget _results() {
    if (_review.selected != null) {
      final members = _review.members.locations;
      if (members.isEmpty) {
        return Center(
          child: Text(
            _review.loading ? "" : LibraryDuplicateStrings.emptyLocations,
          ),
        );
      }
      return LibraryDuplicateReviewList(
        key: ValueKey(_review.pageKey),
        assets: members.map((member) => member.asset).toList(growable: false),
        itemBuilder: (context, index) {
          final member = members[index];
          final preferred = _review.members.decision.prefers(member.asset);
          return LibraryDuplicateMemberCard(
            member: member,
            preferred: preferred,
            onOpen: () => unawaited(_open(member.asset)),
            onPrefer:
                _review.saving ||
                    _review.loading ||
                    (!preferred &&
                        member.availability !=
                            LibraryDuplicateAvailability.local)
                ? null
                : () => unawaited(_prefer(preferred ? null : member.asset)),
          );
        },
      );
    }
    final groups = _review.groups.groups;
    if (groups.isEmpty) {
      final String message;
      if (_review.ignored) {
        message = LibraryDuplicateStrings.noIgnoredGroups;
      } else if (_analysis.isRunning) {
        message = LibraryDuplicateStrings.waitingForResults;
      } else {
        message = LibraryDuplicateStrings.emptyAfterAnalysis;
      }
      return Center(child: Text(_review.loading ? "" : message));
    }
    return LibraryDuplicateReviewList(
      key: ValueKey(_review.pageKey),
      assets: groups
          .map((group) => group.representative)
          .toList(growable: false),
      itemBuilder: (context, index) {
        final entry = groups[index];
        return LibraryDuplicateGroupCard(
          entry: entry,
          onOpen: () => unawaited(_review.showGroup(entry)),
        );
      },
    );
  }
}
