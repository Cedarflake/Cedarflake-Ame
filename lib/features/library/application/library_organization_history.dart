import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_organization.dart";
import "../domain/library_organization_history.dart";
import "library_controller.dart";
import "library_organization.dart";
import "library_organization_actions.dart";
import "library_organization_publication.dart";
import "library_organization_requests.dart";

final libraryOrganizationHistoryProvider =
    FutureProvider.autoDispose<LibraryOrganizationHistory?>((ref) {
      final activity = ref.watch(libraryOrganizationActionsProvider);
      ref.watch(
        libraryControllerProvider.select((state) => state.catalogRevision),
      );
      if (activity.saving) return null;
      return ref.watch(libraryOrganizationProvider).history();
    });

class LibraryOrganizationHistoryRequest extends LibraryOrganizationRequest {
  LibraryOrganizationHistoryRequest({
    required this.direction,
    required this.originalId,
    required BigInt revision,
  }) : _revision = revision;

  final LibraryOrganizationHistoryDirection direction;
  final String originalId;
  BigInt? _revision;
  String? _retiredAlbumId;

  @override
  String get successMessage => switch (direction) {
    LibraryOrganizationHistoryDirection.undo => "已撤销",
    LibraryOrganizationHistoryDirection.redo => "已重做",
  };

  @override
  String get failureTitle => switch (direction) {
    LibraryOrganizationHistoryDirection.undo => "无法撤销",
    LibraryOrganizationHistoryDirection.redo => "无法重做",
  };

  @override
  LibraryOrganizationChange get change {
    final albumId = _retiredAlbumId;
    return albumId == null
        ? const LibraryOrganizationChange.records()
        : LibraryOrganizationChange.albumRetired(albumId);
  }

  @override
  bool canUndo(LibraryOrganizationReceipt receipt) => false;

  @override
  Future<LibraryOrganizationReceipt> execute(
    LibraryOrganization service,
  ) async {
    var revision = _revision;
    if (revision == null) {
      final history = await service.history();
      requireActive();
      if (history.operationFor(direction) != originalId) {
        throw const LibraryOrganizationFailure(
          "organization_history_changed",
          "The next organization history operation has changed",
        );
      }
      revision = _revision = history.revision;
    }
    requireActive();
    final result = await service.stepHistory(
      id,
      direction,
      originalId,
      revision,
    );
    _retiredAlbumId = result.retiredAlbumId;
    return result.operation;
  }

  @override
  void refreshAuthority() {
    super.refreshAuthority();
    _revision = null;
  }
}
