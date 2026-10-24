import "package:cedarflake_ame/features/library/application/library_album_proposal.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/settings/application/album_preferences.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  const current = LibraryOrganizationAlbum("current", "Current");
  const preferred = LibraryOrganizationAlbum("preferred", "Preferred");
  for (final ask in [true, false]) {
    test(
      "existing assignment uses the intended destination with prompting $ask",
      () async {
        final service = OrganizationServiceFixture()
          ..albumItems = [current, preferred];
        service.inspectTarget = (_) async => LibraryOrganizationInspection(
          revision: BigInt.one,
          decisions: const [
            LibraryOrganizationDecision(
              favorite: true,
              disposition: LibraryOrganizationDisposition.organized,
              album: current,
            ),
          ],
        );
        final plan = await prepareLibraryAlbumProposal(
          service: service,
          target: organizationTarget("one"),
          preferences: AlbumPreferences(
            askBeforeAdding: ask,
            defaultAlbumId: preferred.id,
          ),
        );
        expect(plan.shouldAsk, ask);
        expect(plan.current?.id, current.id);
        expect(plan.destination?.id, ask ? current.id : preferred.id);
        expect(service.writes, isEmpty);
        expect(service.memberships, isEmpty);
      },
    );
  }

  test(
    "missing defaults and unresolved conflicts require a destination choice",
    () async {
      final service = OrganizationServiceFixture();
      final missing = await prepareLibraryAlbumProposal(
        service: service,
        target: organizationTarget("one"),
        preferences: const AlbumPreferences(
          askBeforeAdding: false,
          defaultAlbumId: "deleted",
        ),
      );
      expect(missing.shouldAsk, isTrue);
      expect(missing.destination, isNull);
      service.albumItems = [preferred];
      service.inspectTarget = (_) async => LibraryOrganizationInspection(
        revision: BigInt.one,
        decisions: const [
          LibraryOrganizationDecision(
            favorite: false,
            disposition: LibraryOrganizationDisposition.deferred,
            hasConflict: true,
          ),
        ],
      );
      final conflict = await prepareLibraryAlbumProposal(
        service: service,
        target: organizationTarget("one"),
        preferences: const AlbumPreferences(
          askBeforeAdding: false,
          defaultAlbumId: "preferred",
        ),
      );
      expect(conflict.shouldAsk, isTrue);
      expect(conflict.hasConflict, isTrue);
      expect(conflict.destination, isNull);
      expect(service.memberships, isEmpty);
    },
  );
}
