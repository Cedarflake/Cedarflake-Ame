import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/organization_batch.dart" as api;
import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/organization.dart" as organization;
import "../../../src/rust/domain/organization_batch.dart" as dto;
import "../domain/library_organization.dart";
import "../domain/library_organization_batch.dart";
import "library_catalog.dart";
import "library_review.dart";

abstract interface class LibraryOrganizationBatches {
  Future<LibraryOrganizationBatch> open(LibraryOrganizationSelection selection);
}

abstract interface class LibraryOrganizationBatch {
  Future<void> append(List<String> assetIds);
  Future<LibraryBatchSummary> prepare();
  Future<LibraryBatchSummary> refresh();
  Future<LibraryOrganizationReceipt> apply(
    String operationId,
    LibraryBatchAction action,
  );
  Future<LibraryBatchProgress> progress();
  Future<void> close();
}

final libraryOrganizationBatchesProvider = Provider<LibraryOrganizationBatches>(
  (ref) => const RustLibraryOrganizationBatches(),
);

class RustLibraryOrganizationBatches implements LibraryOrganizationBatches {
  const RustLibraryOrganizationBatches();

  @override
  Future<LibraryOrganizationBatch> open(
    LibraryOrganizationSelection selection,
  ) => _call(() async {
    final draft = await api.openGalleryOrganizationSelection(
      query: mapLibraryGalleryQueryToRust(selection.query),
      catalogRevision: selection.catalogRevision,
      allMatching: selection.allMatching,
    );
    return _RustBatch(draft);
  });
}

Future<T> _call<T>(Future<T> Function() operation) async {
  try {
    return await operation();
  } on rust.ScanError catch (error) {
    throw LibraryOrganizationFailure(error.code, error.message);
  }
}

class _RustBatch implements LibraryOrganizationBatch {
  _RustBatch(this.draft);
  final api.OrganizationSelectionDraft draft;
  Future<void>? _closing;

  @override
  Future<void> append(List<String> assetIds) =>
      _call(() => draft.appendIds(assetIds: assetIds));
  @override
  Future<LibraryBatchSummary> prepare() =>
      _call(() async => _summary(await draft.prepare()));
  @override
  Future<LibraryBatchSummary> refresh() =>
      _call(() async => _summary(await draft.refresh()));

  @override
  Future<LibraryOrganizationReceipt> apply(
    String operationId,
    LibraryBatchAction action,
  ) => _call(() async {
    final result = await draft.apply(
      operationId: operationId,
      action: switch (action) {
        LibraryBatchFavorite(:final favorite) =>
          dto.OrganizationBatchAction.favorite(favorite: favorite),
        LibraryBatchAlbum(:final albumId) =>
          dto.OrganizationBatchAction.organize(
            action: albumId == null
                ? const organization.OrganizationAction.removeAlbum()
                : organization.OrganizationAction.assignAlbum(albumId: albumId),
          ),
        LibraryBatchProgressChange(:final action) =>
          dto.OrganizationBatchAction.organize(
            action: mapLibraryProgressAction(action),
          ),
      },
    );
    return LibraryOrganizationReceipt(
      operationId: result.operationId,
      revision: result.organizationRevision,
      changedGroups: result.changedGroups.toInt(),
    );
  });

  @override
  Future<LibraryBatchProgress> progress() => _call(() async {
    final value = await draft.progress();
    return LibraryBatchProgress(
      switch (value.phase) {
        dto.OrganizationBatchPhase.collecting => LibraryBatchPhase.collecting,
        dto.OrganizationBatchPhase.preparing => LibraryBatchPhase.preparing,
        dto.OrganizationBatchPhase.ready => LibraryBatchPhase.ready,
        dto.OrganizationBatchPhase.saving => LibraryBatchPhase.saving,
        dto.OrganizationBatchPhase.completed => LibraryBatchPhase.completed,
        dto.OrganizationBatchPhase.failed => LibraryBatchPhase.failed,
        dto.OrganizationBatchPhase.cancelled => LibraryBatchPhase.cancelled,
      },
      value.completedGroups.toInt(),
      value.totalGroups.toInt(),
    );
  });

  @override
  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    try {
      await _call(draft.close);
    } finally {
      draft.dispose();
    }
  }

  LibraryBatchSummary _summary(dto.OrganizationBatchSummary value) =>
      LibraryBatchSummary(
        groups: value.groupCount.toInt(),
        favorites: value.favoriteCount.toInt(),
        assigned: value.assignedCount.toInt(),
        conflicts: value.conflictingCount.toInt(),
        commonAlbum: value.commonAlbum == null
            ? null
            : LibraryOrganizationAlbum(
                value.commonAlbum!.albumId,
                value.commonAlbum!.name,
              ),
      );
}
