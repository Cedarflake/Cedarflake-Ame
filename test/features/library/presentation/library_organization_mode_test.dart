import "dart:async";

import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_review.dart";
import "package:cedarflake_ame/features/library/presentation/library_organization_mode.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_review_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  late ReviewServiceFixture service;
  late ReviewSurfaceFixture surface;
  late LibraryOrganizationMode mode;
  setUp(() {
    service = ReviewServiceFixture();
    surface = ReviewSurfaceFixture();
    mode = LibraryOrganizationMode(service, surface);
  });
  tearDown(() => mode.dispose());

  test(
    "viewer-admitted representative replacement renews authority without duplicate checkpoints",
    () async {
      await mode.enter();
      mode.open(surface.library.assets.single);
      await Future<void>.delayed(Duration.zero);
      final previous = mode.current;
      final replacement = galleryAsset(
        id: "replacement",
        captureLocalTime: "2020-01-01T00:00:00",
      );
      mode.reconcileDisplayedAsset(replacement);
      mode.adoptDisplayedAsset(replacement);
      await Future<void>.delayed(Duration.zero);
      expect(mode.current, isNot(same(previous)));
      expect(service.checkpoints, hasLength(2));
      expect(service.checkpoints.last.target.locationId, "replacement");
      expect(mode.session!.owns(previous!), isFalse);
    },
  );

  test(
    "catalog publications coalesce continuation reads and retire old results",
    () async {
      final old = Completer<LibraryReviewContinuation>();
      var reads = 0;
      service.saved = const LibraryReviewContinuation(
        LibraryContinuationStatus.emptyScope,
      );
      service.reading = () async => ++reads == 1 ? old.future : service.saved;
      final entering = mode.enter();
      await Future<void>.delayed(Duration.zero);
      final updates = List.generate(20, (_) => mode.refreshContinuation());
      expect(reads, 1);
      old.complete(
        const LibraryReviewContinuation(LibraryContinuationStatus.noActivity),
      );
      await entering;
      await Future.wait(updates);
      expect(reads, 2);
      expect(mode.continuation?.status, LibraryContinuationStatus.emptyScope);
    },
  );

  test(
    "entry and queue switching create no activity and exit restores ordinary context",
    () async {
      final query = surface.library.query.copyWith(
        sortKey: LibraryGallerySortKey.fileName,
        sortDirection: LibraryGallerySortDirection.ascending,
      );
      surface.library = surface.library.copyWith(query: query);
      await mode.enter();
      expect(mode.queue, LibraryOrganizationQueue.pending);
      expect(mode.continuation?.canResume, isFalse);
      await mode.selectQueue(LibraryOrganizationQueue.ignored);
      expect(mode.queue, LibraryOrganizationQueue.ignored);
      expect(service.starts, isEmpty);
      await mode.exit();
      expect(mode.isActive, isFalse);
      expect(surface.library.query, query);
      expect(surface.restored!.selection.includedAssetIds, {
        surface.library.assets.first.assetId,
      });
      expect(surface.positions.last!.globalItemIndex, 23);
      expect(surface.positions.last!.itemFraction, .3);
      expect(surface.positions.last!.viewportFraction, .4);
    },
  );

  test(
    "resume uses the saved scope and identity, never the queue head",
    () async {
      final target = LibraryOrganizationTarget.fromAsset(
        surface.library.assets.single,
      );
      final savedScope = surface.library.query.copyWith(
        organizationQueue: LibraryOrganizationQueue.deferred,
        searchText: "saved",
      );
      service.saved = LibraryReviewContinuation(
        LibraryContinuationStatus.ready,
        query: savedScope,
        target: target,
      );
      await mode.enter();
      await mode.resume();
      await Future<void>.delayed(Duration.zero);
      expect(surface.targets.last, target);
      expect(surface.library.query, savedScope);
      expect(surface.opened.single.assetId, target.assetId);
      expect(service.checkpoints.single.target, target);
    },
  );

  test(
    "new arrivals update continuation availability without moving the open image",
    () async {
      await mode.enter();
      mode.open(surface.library.assets.single);
      await Future<void>.delayed(Duration.zero);
      final entry = mode.current;
      service.saved = LibraryReviewContinuation(
        LibraryContinuationStatus.ready,
        query: surface.library.query,
        target: LibraryOrganizationTarget.fromAsset(
          surface.library.assets.single,
        ),
      );
      await mode.refreshContinuation();
      expect(mode.current, same(entry));
      expect(surface.opened, isEmpty);
      expect(service.checkpoints, hasLength(1));
    },
  );

  test("save completion advances only its current committed entry", () async {
    await mode.enter();
    mode.open(surface.library.assets.single);
    await Future<void>.delayed(Duration.zero);
    final entry = mode.current!;
    await mode.afterSave(entry);
    expect(surface.closes, 0);
    mode.session!.accept(
      entry,
      LibraryReviewReceipt(
        LibraryOrganizationReceipt(
          operationId: "saved",
          revision: BigInt.two,
          changedGroups: 1,
        ),
        service.checkpoints.last.position.next(),
      ),
    );
    service.saved = const LibraryReviewContinuation(
      LibraryContinuationStatus.emptyScope,
    );
    await mode.afterSave(entry);
    expect(surface.closes, 1);
    expect(mode.current, isNull);
  });

  test(
    "closing the viewer during continuation loading cannot reopen it",
    () async {
      await mode.enter();
      mode.open(surface.library.assets.single);
      await Future<void>.delayed(Duration.zero);
      final entry = mode.current!;
      mode.session!.accept(
        entry,
        LibraryReviewReceipt(
          LibraryOrganizationReceipt(
            operationId: "saved",
            revision: BigInt.two,
            changedGroups: 1,
          ),
          service.checkpoints.last.position.next(),
        ),
      );
      service.saved = LibraryReviewContinuation(
        LibraryContinuationStatus.ready,
        query: surface.library.query,
        target: entry.target,
      );
      final publish = Completer<bool>();
      surface.publishing = () => publish.future;
      final advance = mode.afterSave(entry);
      await Future<void>.delayed(Duration.zero);
      mode.closeCurrent();
      publish.complete(true);
      await advance;
      expect(surface.opened, isEmpty);
    },
  );

  test(
    "a failed resume can retry the same workflow and keeps ordinary context",
    () async {
      await mode.enter();
      service.saved = LibraryReviewContinuation(
        LibraryContinuationStatus.ready,
        query: surface.library.query,
        target: LibraryOrganizationTarget.fromAsset(
          surface.library.assets.single,
        ),
      );
      surface.publishing = () async => false;
      await mode.resume();
      expect(mode.problem, isNotNull);
      expect(surface.opened, isEmpty);
      surface.publishing = null;
      await mode.retry();
      expect(surface.opened, hasLength(1));
      expect(mode.problem, isNull);
    },
  );
}
