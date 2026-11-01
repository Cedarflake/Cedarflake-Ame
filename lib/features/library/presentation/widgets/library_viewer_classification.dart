import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_classification_settings.dart";
import "../../application/library_classification_progress.dart";
import "../../application/library_classification_review_session.dart";
import "../../application/library_controller.dart";
import "../../domain/library_classification_review.dart";
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "library_classification_controls.dart";

class LibraryViewerClassification extends ConsumerStatefulWidget {
  const LibraryViewerClassification({
    required this.target,
    required this.query,
    super.key,
  });
  final LibraryOrganizationTarget target;
  final LibraryGalleryQuery query;

  @override
  ConsumerState<LibraryViewerClassification> createState() =>
      _LibraryViewerClassificationState();
}

class _LibraryViewerClassificationState
    extends ConsumerState<LibraryViewerClassification> {
  LibraryClassificationReviewSession? _session;
  LibraryClassificationReviewEntry? _entry;

  @override
  void initState() {
    super.initState();
    _openScope();
  }

  void _openScope() {
    if (widget.query.collection is LibrarySmartAlbumCollection) {
      _session = LibraryClassificationReviewSession(
        ref.read(libraryClassificationProgressProvider),
        widget.query,
      );
      _entry = _session!.open(widget.target);
    }
  }

  Future<void> _closeSession(
    LibraryClassificationReviewSession? session,
  ) async {
    if (session == null) return;
    try {
      await session.close();
    } on Object catch (error) {
      debugPrint("[Ame classification] review close failed: $error");
    }
  }

  @override
  void didUpdateWidget(LibraryViewerClassification oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.query != widget.query) {
      unawaited(_closeSession(_session));
      _session = null;
      _entry = null;
      _openScope();
    } else if (oldWidget.target != widget.target) {
      _entry = _session?.open(widget.target);
    }
  }

  @override
  void dispose() {
    unawaited(_closeSession(_session));
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final settings = ref.watch(libraryClassificationSettingsProvider).value;
    final revision = ref.watch(
      libraryControllerProvider.select((state) => state.catalogRevision),
    );
    if (revision == null || settings == null || settings.categories.isEmpty) {
      return const SizedBox.shrink();
    }
    return LibraryClassificationControls(
      key: ValueKey((widget.target, _entry)),
      reviewEntry: _entry,
      viewer: true,
      enabled: true,
      scope: LibraryClassificationScope(
        query: const LibraryGalleryQuery(),
        catalogRevision: revision,
        allMatching: false,
        assetIds: [widget.target.assetId],
        targets: [widget.target],
      ),
    );
  }
}
