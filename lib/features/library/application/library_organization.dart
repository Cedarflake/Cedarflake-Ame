import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/organization.dart" as api;
import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/organization.dart" as dto;
import "../domain/library_organization.dart";

abstract interface class LibraryOrganization {
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
  Future<LibraryOrganizationReceipt> undo(
    String operationId,
    String originalId,
    BigInt revision,
  );
}

class RustLibraryOrganization implements LibraryOrganization {
  const RustLibraryOrganization();

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
    () async => _RustOrganizationPrepared(
      await api.prepareOrganizationSelection(selections: _selections(targets)),
    ),
  );

  @override
  Future<LibraryOrganizationReceipt> favorite(
    String operationId,
    LibraryOrganizationPrepared prepared,
    bool favorite,
  ) => _call(() async {
    if (prepared is! _RustOrganizationPrepared) {
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
  Future<LibraryOrganizationReceipt> undo(
    String operationId,
    String originalId,
    BigInt revision,
  ) => _call(
    () async => _receipt(
      await api.undoOrganizationOperation(
        operationId: operationId,
        originalId: originalId,
        expectedRevision: revision,
      ),
    ),
  );

  LibraryOrganizationReceipt _receipt(dto.OrganizationWriteReceipt value) =>
      LibraryOrganizationReceipt(
        operationId: value.operationId,
        revision: value.organizationRevision,
        changedGroups: value.changedGroups.toInt(),
      );
}

class _RustOrganizationPrepared extends LibraryOrganizationPrepared {
  const _RustOrganizationPrepared(this.value);
  final dto.OrganizationPreparedSelection value;
}

final libraryOrganizationProvider = Provider<LibraryOrganization>(
  (ref) => const RustLibraryOrganization(),
);
