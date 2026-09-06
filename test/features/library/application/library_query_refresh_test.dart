import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog_publication.dart";
import "package:cedarflake_ame/features/library/application/library_query_refresh.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "committed refresh waits for an active passive query and admits no competing refresh",
    () async {
      final owner = LibraryQueryRefreshCoordinator(
        LibraryCatalogPublicationCoordinator(),
      );
      addTearDown(owner.dispose);
      final passiveResult = Completer<LibraryQueryUpdateOutcome>();
      final passive = owner.runPassive(() => passiveResult.future);
      var reads = 0;
      final refreshed = owner.refreshCommitted(() async {
        reads += 1;
        return LibraryQueryUpdateOutcome.applied;
      });
      await Future<void>.delayed(Duration.zero);
      expect(reads, 0);
      expect(
        await owner.runPassive(() => throw StateError("competing read")),
        LibraryQueryUpdateOutcome.busy,
      );
      passiveResult.complete(LibraryQueryUpdateOutcome.applied);
      await passive;
      expect(await refreshed, isTrue);
      expect(reads, 1);
    },
  );

  test(
    "exclusive publication replacement is awaited before a fresh attempt",
    () async {
      final publications = LibraryCatalogPublicationCoordinator();
      final owner = LibraryQueryRefreshCoordinator(publications);
      addTearDown(owner.dispose);
      addTearDown(publications.dispose);
      final first = Completer<LibraryQueryUpdateOutcome>();
      var reads = 0;
      final refreshed = owner.refreshCommitted(() {
        reads += 1;
        return reads == 1
            ? first.future
            : Future.value(LibraryQueryUpdateOutcome.applied);
      });
      await Future<void>.delayed(Duration.zero);
      final publication = publications.reserve()!;
      owner.invalidate();
      first.complete(LibraryQueryUpdateOutcome.superseded);
      await Future<void>.delayed(Duration.zero);
      expect(reads, 1);
      publications.release(publication);
      expect(await refreshed, isTrue);
      expect(reads, 2);
    },
  );

  test(
    "a non-replaced failure or superseded attempt is not retried or accepted",
    () async {
      final owner = LibraryQueryRefreshCoordinator(
        LibraryCatalogPublicationCoordinator(),
      );
      addTearDown(owner.dispose);
      var reads = 0;
      for (final outcome in [
        LibraryQueryUpdateOutcome.failed,
        LibraryQueryUpdateOutcome.superseded,
        LibraryQueryUpdateOutcome.busy,
      ]) {
        expect(
          await owner.refreshCommitted(() async {
            reads += 1;
            return outcome;
          }),
          isFalse,
        );
      }
      expect(reads, 3);
    },
  );

  test(
    "an exception releases committed admission for the next request",
    () async {
      final owner = LibraryQueryRefreshCoordinator(
        LibraryCatalogPublicationCoordinator(),
      );
      addTearDown(owner.dispose);
      await expectLater(
        owner.refreshCommitted(() => throw StateError("query failure")),
        throwsStateError,
      );
      expect(
        await owner.refreshCommitted(
          () async => LibraryQueryUpdateOutcome.applied,
        ),
        isTrue,
      );
      expect(
        await owner.runPassive(() async => LibraryQueryUpdateOutcome.applied),
        LibraryQueryUpdateOutcome.applied,
      );
    },
  );

  test(
    "disposal resolves retired and current attempts and queued refresh obligations",
    () async {
      final owner = LibraryQueryRefreshCoordinator(
        LibraryCatalogPublicationCoordinator(),
      );
      final retiredResult = Completer<LibraryQueryUpdateOutcome>();
      final currentResult = Completer<LibraryQueryUpdateOutcome>();
      final first = owner.refreshCommitted(() => retiredResult.future);
      await Future<void>.delayed(Duration.zero);
      final user = owner.runUser(() => currentResult.future);
      final queued = owner.refreshCommitted(
        () => throw StateError("disposed query"),
      );
      owner.dispose();
      expect(await first, isFalse);
      expect(await queued, isFalse);
      expect(await user, LibraryQueryUpdateOutcome.superseded);
      retiredResult.completeError(StateError("late retired failure"));
      currentResult.complete(LibraryQueryUpdateOutcome.applied);
      await Future<void>.delayed(Duration.zero);
    },
  );
}
