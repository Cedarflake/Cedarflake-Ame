import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "../domain/library_organization.dart";
import "../domain/library_organization_conflicts.dart";
import "library_controller.dart";
import "library_conflict_status_reader.dart";
import "library_organization_actions.dart";
import "ports/library_organization_conflicts.dart";

final libraryOrganizationConflictsProvider =
    Provider<LibraryOrganizationConflicts?>(
      (ref) => throw StateError(
        "LibraryOrganizationConflicts must be bound during application startup",
      ),
    );

final libraryConflictStatusProvider =
    StreamProvider.autoDispose<LibraryConflictStatus>((ref) {
      final service = ref.watch(libraryOrganizationConflictsProvider);
      if (service == null) return const Stream.empty();
      final reader = LibraryConflictStatusReader(service.status);
      ref.onDispose(reader.close);
      ref.listen(
        libraryControllerProvider.select((state) => state.catalogRevision),
        (_, next) => reader.refresh(next),
      );
      ref.listen(
        libraryOrganizationActionsProvider.select((state) => state.sequence),
        (_, _) =>
            reader.refresh(ref.read(libraryControllerProvider).catalogRevision),
      );
      reader.refresh(ref.read(libraryControllerProvider).catalogRevision);
      return reader.changes;
    });

final libraryConflictAlbumsProvider = FutureProvider.autoDispose
    .family<
      LibraryConflictAlbums,
      ({LibraryOrganizationTarget target, String? after})
    >((ref, request) {
      ref.watch(
        libraryControllerProvider.select((state) => state.catalogRevision),
      );
      ref.watch(
        libraryOrganizationActionsProvider.select((state) => state.sequence),
      );
      final service = ref.watch(libraryOrganizationConflictsProvider);
      if (service == null) {
        throw StateError("Conflict inspection is unavailable");
      }
      return service.albums(request.target, after: request.after);
    });

final libraryConflictAnnotationsProvider =
    FutureProvider.autoDispose<Set<String>>((ref) async {
      final service = ref.watch(libraryOrganizationConflictsProvider);
      if (service == null) return const {};
      final window = ref.watch(
        libraryControllerProvider.select(
          (state) => _ConflictWindow(
            state.catalogRevision,
            state.assets
                .map((asset) => asset.locationId)
                .toList(growable: false),
          ),
        ),
      );
      ref.watch(
        libraryOrganizationActionsProvider.select((state) => state.sequence),
      );
      final revision = window.revision;
      if (revision == null) return const {};
      var retired = false;
      ref.onDispose(() => retired = true);
      final locations = <String>{};
      for (
        var offset = 0;
        offset < window.locationIds.length && !retired;
        offset += 256
      ) {
        final end = (offset + 256).clamp(0, window.locationIds.length);
        locations.addAll(
          await service.annotations(
            revision,
            window.locationIds.sublist(offset, end),
          ),
        );
      }
      return retired ? const {} : Set.unmodifiable(locations);
    });

class _ConflictWindow {
  const _ConflictWindow(this.revision, this.locationIds);
  final BigInt? revision;
  final List<String> locationIds;
  @override
  bool operator ==(Object other) =>
      other is _ConflictWindow &&
      revision == other.revision &&
      listEquals(locationIds, other.locationIds);
  @override
  int get hashCode => Object.hash(revision, Object.hashAll(locationIds));
}
