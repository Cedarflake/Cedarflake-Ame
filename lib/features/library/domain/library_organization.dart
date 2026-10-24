import "library_models.dart";

class LibraryOrganizationTarget {
  const LibraryOrganizationTarget({
    required this.assetId,
    required this.locationId,
    required this.sourceGeneration,
  });

  factory LibraryOrganizationTarget.fromAsset(LibraryAsset asset) =>
      LibraryOrganizationTarget(
        assetId: asset.assetId,
        locationId: asset.locationId,
        sourceGeneration: asset.sourceGeneration,
      );

  final String assetId;
  final String locationId;
  final BigInt sourceGeneration;

  @override
  int get hashCode => Object.hash(assetId, locationId, sourceGeneration);

  @override
  bool operator ==(Object other) =>
      other is LibraryOrganizationTarget &&
      assetId == other.assetId &&
      locationId == other.locationId &&
      sourceGeneration == other.sourceGeneration;
}

enum LibraryOrganizationDisposition { pending, deferred, organized, ignored }

class LibraryOrganizationAlbum {
  const LibraryOrganizationAlbum(this.id, this.name);
  final String id;
  final String name;
}

class LibraryOrganizationDecision {
  const LibraryOrganizationDecision({
    required this.favorite,
    required this.disposition,
    this.album,
    this.hasConflict = false,
  });
  final bool favorite;
  final LibraryOrganizationDisposition disposition;
  final LibraryOrganizationAlbum? album;
  final bool hasConflict;
}

class LibraryOrganizationInspection {
  const LibraryOrganizationInspection({
    required this.revision,
    required this.decisions,
  });
  final BigInt revision;
  final List<LibraryOrganizationDecision> decisions;
}

abstract class LibraryOrganizationPrepared {
  const LibraryOrganizationPrepared();
}

class LibraryOrganizationReceipt {
  const LibraryOrganizationReceipt({
    required this.operationId,
    required this.revision,
    required this.changedGroups,
  });
  final String operationId;
  final BigInt revision;
  final int changedGroups;
}

class LibraryOrganizationFailure implements Exception {
  const LibraryOrganizationFailure(this.code, this.message);
  final String code;
  final String message;
  bool get isStale => code == "organization_selection_changed";
  @override
  String toString() => message;
}
