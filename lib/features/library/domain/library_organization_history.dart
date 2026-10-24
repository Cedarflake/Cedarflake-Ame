import "library_organization.dart";

enum LibraryOrganizationHistoryDirection { undo, redo }

class LibraryOrganizationHistory {
  const LibraryOrganizationHistory({
    required this.revision,
    this.undoOperationId,
    this.redoOperationId,
  });

  final BigInt revision;
  final String? undoOperationId;
  final String? redoOperationId;

  String? operationFor(LibraryOrganizationHistoryDirection direction) =>
      switch (direction) {
        LibraryOrganizationHistoryDirection.undo => undoOperationId,
        LibraryOrganizationHistoryDirection.redo => redoOperationId,
      };
}

class LibraryOrganizationHistoryReceipt {
  const LibraryOrganizationHistoryReceipt({
    required this.operation,
    this.retiredAlbumId,
  });

  final LibraryOrganizationReceipt operation;
  final String? retiredAlbumId;
}
