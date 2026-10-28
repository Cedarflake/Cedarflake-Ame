import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_actions.dart";
import "package:cedarflake_ame/features/library/application/library_sidebar_album_removal.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  for (final stale in [false, true]) {
    test(
      "album retry preserves prior commits and ${stale ? 'renews stale authority' : 'reuses unknown command identity'}",
      () async {
        final service = OrganizationServiceFixture();
        final container = ProviderContainer(
          overrides: [
            libraryOrganizationProvider.overrideWithValue(service),
            libraryControllerProvider.overrideWith(
              OrganizationLibraryFixture.new,
            ),
          ],
        );
        addTearDown(container.dispose);
        var fail = true;
        service.saveAlbum = (id) async {
          final write = service.albumWrites.last;
          if (write.albumId == "two" && fail) {
            throw LibraryOrganizationFailure(
              stale ? "organization_selection_changed" : "database_error",
              "write failed",
            );
          }
          return LibraryOrganizationReceipt(
            operationId: id,
            revision: write.revision + BigInt.one,
            changedGroups: 1,
          );
        };
        final removal = createSidebarAlbumRemoval(
          albums: const [
            LibraryOrganizationAlbum("one", "First"),
            LibraryOrganizationAlbum("two", "Second"),
          ],
          revision: BigInt.one,
          actions: container.read(libraryOrganizationActionsProvider.notifier),
        );
        addTearDown(removal.dispose);
        await removal.run();
        expect(removal.completed, {"one"});
        expect(service.albumWrites.map((w) => w.revision), [
          BigInt.one,
          BigInt.two,
        ]);
        fail = false;
        await removal.run();
        expect(service.albumWrites.map((w) => w.albumId), [
          "one",
          "two",
          "two",
        ]);
        expect(
          service.albumWrites.last.id == service.albumWrites[1].id,
          !stale,
        );
        expect(removal.isComplete, isTrue);
      },
    );
  }
}
