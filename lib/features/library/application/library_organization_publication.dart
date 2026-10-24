import "dart:async";

import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "library_controller.dart";

sealed class LibraryOrganizationChange {
  const LibraryOrganizationChange();
  const factory LibraryOrganizationChange.records() =
      LibraryOrganizationRecordsChanged;
  const factory LibraryOrganizationChange.albumRetired(String albumId) =
      LibraryOrganizationAlbumRetired;
}

final class LibraryOrganizationRecordsChanged
    extends LibraryOrganizationChange {
  const LibraryOrganizationRecordsChanged();
}

final class LibraryOrganizationAlbumRetired extends LibraryOrganizationChange {
  const LibraryOrganizationAlbumRetired(this.albumId);
  final String albumId;
}

abstract interface class LibraryOrganizationProjection {
  Future<bool> retireAlbum(String albumId);
}

class LibraryOrganizationProjectionRegistration {
  LibraryOrganizationProjectionRegistration._(this.projection);
  final LibraryOrganizationProjection projection;
}

/// Committed collection retirement precedes reads and survives a failed refresh.
class LibraryOrganizationPublication {
  LibraryOrganizationPublication(this._ref) {
    _ref.onDispose(() {
      _disposed = true;
      _projection = null;
      _retiredAlbums.clear();
    });
  }

  final Ref _ref;
  final Set<String> _retiredAlbums = {};
  LibraryOrganizationProjectionRegistration? _projection;
  Future<void> _publicationTail = Future<void>.value();
  bool _disposed = false;

  LibraryOrganizationProjectionRegistration attach(
    LibraryOrganizationProjection projection,
  ) {
    final registration = LibraryOrganizationProjectionRegistration._(
      projection,
    );
    if (!_disposed) _projection = registration;
    return registration;
  }

  void detach(LibraryOrganizationProjectionRegistration registration) {
    if (identical(_projection, registration)) _projection = null;
  }

  Future<void> refresh([
    LibraryOrganizationChange change =
        const LibraryOrganizationChange.records(),
  ]) async {
    if (_disposed) return;
    if (change case LibraryOrganizationAlbumRetired(:final albumId)) {
      _retiredAlbums.add(albumId);
    }
    final previous = _publicationTail;
    final released = Completer<void>();
    _publicationTail = released.future;
    try {
      await previous;
      if (!_disposed) await _refresh();
    } finally {
      released.complete();
    }
  }

  Future<void> _refresh() async {
    final retired = Set<String>.of(_retiredAlbums);
    var refreshed = false;
    try {
      var scopesReady = true;
      for (final albumId in retired) {
        final projection = _projection;
        if (projection == null ||
            !await projection.projection.retireAlbum(albumId) ||
            !identical(projection, _projection)) {
          scopesReady = false;
          break;
        }
        if (_disposed) return;
      }
      if (scopesReady && !_disposed) {
        refreshed = await _ref
            .read(libraryControllerProvider.notifier)
            .refreshCurrentQuery();
      }
    } on Object {
      refreshed = false;
    }
    if (_disposed) return;
    final notifications = _ref.read(ameNotificationControllerProvider.notifier);
    if (refreshed) {
      _retiredAlbums.removeAll(retired);
      notifications.resolve("organization-gallery-refresh");
    } else {
      notifications.publish(
        const AmeNotificationDraft(
          title: "更改已保存",
          message: "图库尚未刷新，请重试。",
          severity: AmeNotificationSeverity.warning,
          dedupeKey: "organization-gallery-refresh",
          actionId: "organization.refresh",
          actionLabel: "重试",
          isPersistent: true,
        ),
      );
    }
  }
}

final libraryOrganizationPublicationProvider = Provider(
  LibraryOrganizationPublication.new,
);
