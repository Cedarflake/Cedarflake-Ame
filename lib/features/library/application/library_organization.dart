import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/organization.dart" as api;
import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/organization.dart" as dto;
import "../domain/library_organization.dart";
import "../domain/library_organization_history.dart";
import "../domain/library_review.dart";

abstract interface class LibraryOrganization {
  Future<LibraryOrganizationHistory> history();
  Future<LibraryOrganizationHistoryReceipt> stepHistory(
    String operationId,
    LibraryOrganizationHistoryDirection direction,
    String originalId,
    BigInt revision,
  );
  Future<LibraryOrganizationInspection> inspect(
    List<LibraryOrganizationTarget> targets,
  );
  Future<LibraryOrganizationPrepared> prepare(
    List<LibraryOrganizationTarget> targets,
  );
  Future<LibraryOrganizationReceipt> favorite(
    String operationId,
    LibraryOrganizationPrepared prepared,
    bool favorite,
  );
  Future<BigInt> revision();
  Future<LibraryOrganizationReceipt> progress(
    String operationId,
    LibraryOrganizationPrepared prepared,
    LibraryProgressAction action,
  );
  Future<LibraryOrganizationAlbumPage> albums({String? after});
  Future<LibraryOrganizationAlbum?> findAlbum(String id);
  Future<LibraryOrganizationReceipt> createAlbum(
    String operationId,
    BigInt revision,
    String name,
  );
  Future<LibraryOrganizationReceipt> renameAlbum(
    String operationId,
    BigInt revision,
    String albumId,
    String name,
  );
  Future<LibraryOrganizationReceipt> deleteAlbum(
    String operationId,
    BigInt revision,
    String albumId,
  );
  Future<LibraryOrganizationReceipt> assignAlbum(
    String operationId,
    LibraryOrganizationPrepared prepared,
    String albumId,
  );
  Future<LibraryOrganizationReceipt> removeAlbum(
    String operationId,
    LibraryOrganizationPrepared prepared,
  );
  Future<LibraryOrganizationHistoryReceipt> undo(
    String operationId,
    String originalId,
    BigInt revision,
  );
}

class RustLibraryOrganization implements LibraryOrganization {
  const RustLibraryOrganization();

  @override
  Future<LibraryOrganizationHistory> history() => _call(() async {
    final value = await api.loadOrganizationHistory();
    return LibraryOrganizationHistory(
      revision: value.organizationRevision,
      undoOperationId: value.undoOperationId,
      redoOperationId: value.redoOperationId,
    );
  });

  @override
  Future<LibraryOrganizationHistoryReceipt> stepHistory(
    String operationId,
    LibraryOrganizationHistoryDirection direction,
    String originalId,
    BigInt revision,
  ) => _call(() async {
    final value = await api.stepOrganizationHistory(
      operationId: operationId,
      direction: switch (direction) {
        LibraryOrganizationHistoryDirection.undo =>
          dto.OrganizationHistoryDirection.undo,
        LibraryOrganizationHistoryDirection.redo =>
          dto.OrganizationHistoryDirection.redo,
      },
      originalId: originalId,
      expectedRevision: revision,
    );
    return LibraryOrganizationHistoryReceipt(
      operation: _receipt(value.operation),
      retiredAlbumId: value.retiredAlbumId,
    );
  });

  List<dto.OrganizationSelection> _selections(
    List<LibraryOrganizationTarget> targets,
  ) => targets
      .map(
        (target) => dto.OrganizationSelection(
          assetId: target.assetId,
          locationId: target.locationId,
          sourceGeneration: target.sourceGeneration,
        ),
      )
      .toList(growable: false);

  Future<T> _call<T>(Future<T> Function() operation) async {
    try {
      return await operation();
    } on rust.ScanError catch (error) {
      throw LibraryOrganizationFailure(error.code, error.message);
    }
  }

  @override
  Future<LibraryOrganizationInspection> inspect(
    List<LibraryOrganizationTarget> targets,
  ) => _call(() async {
    final value = await api.inspectOrganizationSelection(
      selections: _selections(targets),
    );
    return LibraryOrganizationInspection(
      revision: value.organizationRevision,
      decisions: value.decisions
          .map(
            (decision) => LibraryOrganizationDecision(
              favorite: decision.favorite,
              disposition: switch (decision.disposition) {
                dto.OrganizationDisposition.pending =>
                  LibraryOrganizationDisposition.pending,
                dto.OrganizationDisposition.deferred_ =>
                  LibraryOrganizationDisposition.deferred,
                dto.OrganizationDisposition.organized =>
                  LibraryOrganizationDisposition.organized,
                dto.OrganizationDisposition.ignored =>
                  LibraryOrganizationDisposition.ignored,
              },
              album: decision.album == null
                  ? null
                  : LibraryOrganizationAlbum(
                      decision.album!.albumId,
                      decision.album!.name,
                    ),
              hasConflict: decision.hasConflict,
            ),
          )
          .toList(growable: false),
    );
  });

  @override
  Future<LibraryOrganizationPrepared> prepare(
    List<LibraryOrganizationTarget> targets,
  ) => _call(
    () async => RustOrganizationPrepared(
      await api.prepareOrganizationSelection(selections: _selections(targets)),
    ),
  );

  @override
  Future<LibraryOrganizationReceipt> favorite(
    String operationId,
    LibraryOrganizationPrepared prepared,
    bool favorite,
  ) => _call(() async {
    if (prepared is! RustOrganizationPrepared) {
      throw StateError(
        "The prepared selection belongs to another organization adapter",
      );
    }
    return _receipt(
      await api.setOrganizationFavorite(
        operationId: operationId,
        prepared: prepared.value,
        favorite: favorite,
      ),
    );
  });

  @override
  Future<BigInt> revision() => _call(
    () async => (await api.loadOrganizationAlbums()).organizationRevision,
  );

  @override
  Future<LibraryOrganizationReceipt> progress(
    String operationId,
    LibraryOrganizationPrepared prepared,
    LibraryProgressAction action,
  ) => _change(operationId, prepared, switch (action) {
    LibraryProgressAction.defer => const dto.OrganizationAction.defer(),
    LibraryProgressAction.ignore => const dto.OrganizationAction.ignore(),
    LibraryProgressAction.restore =>
      const dto.OrganizationAction.restorePending(),
  });

  @override
  Future<LibraryOrganizationAlbumPage> albums({String? after}) =>
      _call(() async {
        final page = await api.loadOrganizationAlbums(after: after);
        return LibraryOrganizationAlbumPage(
          albums: [
            for (final album in page.albums)
              LibraryOrganizationAlbum(album.albumId, album.name),
          ],
          revision: page.organizationRevision,
          total: page.totalAlbums.toInt(),
          nextAlbumId: page.nextAlbumId,
        );
      });

  @override
  Future<LibraryOrganizationAlbum?> findAlbum(String id) => _call(() async {
    final value = await api.findOrganizationAlbum(albumId: id);
    return value == null
        ? null
        : LibraryOrganizationAlbum(value.albumId, value.name);
  });

  @override
  Future<LibraryOrganizationReceipt> createAlbum(
    String operationId,
    BigInt revision,
    String name,
  ) => _call(() async {
    final result = await api.createOrganizationAlbum(
      operationId: operationId,
      expectedRevision: revision,
      name: name,
    );
    return _receipt(
      result.operation,
      album: LibraryOrganizationAlbum(result.albumId, name.trim()),
    );
  });

  @override
  Future<LibraryOrganizationReceipt> renameAlbum(
    String operationId,
    BigInt revision,
    String albumId,
    String name,
  ) => _call(() async {
    final result = await api.renameOrganizationAlbum(
      operationId: operationId,
      expectedRevision: revision,
      albumId: albumId,
      name: name,
    );
    return _receipt(
      result.operation,
      album: LibraryOrganizationAlbum(result.albumId, name.trim()),
    );
  });

  @override
  Future<LibraryOrganizationReceipt> deleteAlbum(
    String operationId,
    BigInt revision,
    String albumId,
  ) => _call(
    () async => _receipt(
      await api.deleteOrganizationAlbum(
        operationId: operationId,
        expectedRevision: revision,
        albumId: albumId,
      ),
    ),
  );

  @override
  Future<LibraryOrganizationReceipt> assignAlbum(
    String operationId,
    LibraryOrganizationPrepared prepared,
    String albumId,
  ) => _change(
    operationId,
    prepared,
    dto.OrganizationAction.assignAlbum(albumId: albumId),
  );

  @override
  Future<LibraryOrganizationReceipt> removeAlbum(
    String operationId,
    LibraryOrganizationPrepared prepared,
  ) => _change(
    operationId,
    prepared,
    const dto.OrganizationAction.removeAlbum(),
  );

  Future<LibraryOrganizationReceipt> _change(
    String operationId,
    LibraryOrganizationPrepared prepared,
    dto.OrganizationAction action,
  ) => _call(() async {
    if (prepared is! RustOrganizationPrepared) {
      throw StateError(
        "The prepared selection belongs to another organization adapter",
      );
    }
    return _receipt(
      await api.changeOrganization(
        operationId: operationId,
        prepared: prepared.value,
        action: action,
      ),
    );
  });

  @override
  Future<LibraryOrganizationHistoryReceipt> undo(
    String operationId,
    String originalId,
    BigInt revision,
  ) => _call(() async {
    final value = await api.undoOrganizationOperation(
      operationId: operationId,
      originalId: originalId,
      expectedRevision: revision,
    );
    return LibraryOrganizationHistoryReceipt(
      operation: _receipt(value.operation),
      retiredAlbumId: value.retiredAlbumId,
    );
  });

  LibraryOrganizationReceipt _receipt(
    dto.OrganizationWriteReceipt value, {
    LibraryOrganizationAlbum? album,
  }) => LibraryOrganizationReceipt(
    operationId: value.operationId,
    revision: value.organizationRevision,
    changedGroups: value.changedGroups.toInt(),
    album: album,
  );
}

class RustOrganizationPrepared extends LibraryOrganizationPrepared {
  const RustOrganizationPrepared(this.value);
  final dto.OrganizationPreparedSelection value;
}

final libraryOrganizationProvider = Provider<LibraryOrganization>(
  (ref) => const RustLibraryOrganization(),
);
