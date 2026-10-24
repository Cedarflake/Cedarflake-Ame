import "dart:async";

import "package:cedarflake_ame/features/library/application/library_batch_requests.dart";
import "package:cedarflake_ame/features/library/application/library_selection_session.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_batch.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_batch_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  LibraryOrganizationSelection selection({bool all = false, int ids = 0}) =>
      LibraryOrganizationSelection(
        query: const LibraryGalleryQuery(
          collection: LibraryGalleryCollection.favorites(),
          rootId: "root",
        ),
        catalogRevision: BigInt.from(42),
        allMatching: all,
        assetIds: List.generate(ids, (index) => "asset-$index"),
      );

  test(
    "explicit and excluded selections cross the bridge in bounded chunks",
    () async {
      for (final all in [false, true]) {
        final service = OrganizationBatchesFixture();
        final session = LibrarySelectionSession(
          service,
          selection(all: all, ids: 600),
        );
        await session.prepare();
        final batch = service.batches.single;
        expect(service.selections.single.allMatching, all);
        expect(
          service.selections.single.query.collection,
          const LibraryGalleryCollection.favorites(),
        );
        expect(service.selections.single.catalogRevision, BigInt.from(42));
        expect(batch.chunks.map((chunk) => chunk.length), [256, 256, 88]);
        expect(batch.chunks.expand((chunk) => chunk).toSet(), hasLength(600));
        await session.apply("save", const LibraryBatchFavorite(true));
        expect(batch.preparations, 1);
        session.dispose();
        session.dispose();
        await Future<void>.delayed(Duration.zero);
        expect(batch.closes, 1);
      }
    },
  );

  test(
    "disposing an unopened selection closes its late handle without writing",
    () async {
      final opened = Completer<OrganizationBatchFixture>();
      final service = OrganizationBatchesFixture()
        ..opening = () => opened.future;
      final session = LibrarySelectionSession(service, selection(all: true));
      final pending = session.apply(
        "cancelled",
        const LibraryBatchFavorite(true),
      );
      final expectation = expectLater(
        pending,
        throwsA(isA<LibraryOrganizationFailure>()),
      );
      session.dispose();
      final batch = OrganizationBatchFixture();
      opened.complete(batch);
      await expectation;
      expect(batch.closes, 1);
      expect(batch.preparations, 0);
      expect(batch.writes, isEmpty);
    },
  );

  test(
    "disposing preparation suppresses its late write and releases staging",
    () async {
      final batch = OrganizationBatchFixture()
        ..preparing = Completer<LibraryBatchSummary>();
      final service = OrganizationBatchesFixture()..create = () => batch;
      final session = LibrarySelectionSession(service, selection(ids: 1));
      final pending = session.apply(
        "cancelled",
        const LibraryBatchFavorite(true),
      );
      final expectation = expectLater(
        pending,
        throwsA(isA<LibraryOrganizationFailure>()),
      );
      await Future<void>.delayed(Duration.zero);
      session.dispose();
      batch.preparing!.complete(batch.summary);
      await expectation;
      expect(batch.writes, isEmpty);
      expect(batch.closes, 1);
    },
  );

  test(
    "a failed preparation releases its handle and permits a fresh guarded attempt",
    () async {
      final failed = OrganizationBatchFixture()
        ..preparing = Completer<LibraryBatchSummary>();
      final service = OrganizationBatchesFixture()..create = () => failed;
      final session = LibrarySelectionSession(service, selection(ids: 1));
      final pending = session.prepare();
      final expectation = expectLater(pending, throwsStateError);
      await Future<void>.delayed(Duration.zero);
      expect(failed.preparations, 1);
      failed.preparing!.completeError(StateError("read failed"));
      await expectation;
      service.create = OrganizationBatchFixture.new;
      await session.prepare();
      expect(service.selections, hasLength(2));
      expect(failed.closes, 1);
      expect(service.selections[1].catalogRevision, BigInt.from(42));
      session.dispose();
    },
  );

  test(
    "unknown writes retain operation and snapshot while stale batches require reselection",
    () async {
      final service = OrganizationBatchesFixture();
      final session = LibrarySelectionSession(service, selection(all: true));
      await session.prepare();
      final batch = service.batches.single;
      final request = LibraryBatchFavoriteRequest(session, true);
      batch.saving = (_) async => throw StateError("receipt lost");
      await expectLater(
        request.execute(OrganizationServiceFixture()),
        throwsStateError,
      );
      batch.saving = null;
      await request.execute(OrganizationServiceFixture());
      expect(batch.writes.map((write) => write.id).toSet(), hasLength(1));
      expect(batch.preparations, 1);
      expect(batch.refreshes, 0);
      request.cancel();

      final staleSession = LibrarySelectionSession(
        service,
        selection(all: true),
      );
      await staleSession.prepare();
      service.batches.last.saving = (_) async =>
          throw const LibraryOrganizationFailure(
            "organization_selection_changed",
            "stale",
          );
      final stale = LibraryBatchFavoriteRequest(staleSession, false);
      await expectLater(
        stale.execute(OrganizationServiceFixture()),
        throwsA(
          isA<LibraryOrganizationFailure>().having(
            (failure) => failure.code,
            "recovery",
            "organization_selection_expired",
          ),
        ),
      );
      stale.cancel();
    },
  );

  test(
    "create-in-picker refreshes the same selection without sending IDs again",
    () async {
      final service = OrganizationBatchesFixture();
      final session = LibrarySelectionSession(
        service,
        selection(all: true, ids: 1),
      );
      await session.prepare();
      final membership = LibraryBatchAlbumMembership(session);
      await membership.albumCreated();
      final destination = const LibraryOrganizationAlbum("new", "New");
      final request = membership.request(destination);
      expect(membership.request(destination), same(request));
      await request.execute(OrganizationServiceFixture());
      final batch = service.batches.single;
      expect(batch.refreshes, 1);
      expect(batch.preparations, 1);
      expect(batch.chunks, hasLength(1));
      expect((batch.writes.single.action as LibraryBatchAlbum).albumId, "new");
      membership.dispose();
    },
  );
}
