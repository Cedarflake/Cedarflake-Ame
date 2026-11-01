import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_review_session.dart";
import "package:cedarflake_ame/features/library/application/library_classification_selection.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_progress.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_classification_progress_fixture.dart";
import "../support/library_classification_review_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  test(
    "preparation is passive and a refreshed committed image does not overwrite continuation",
    () async {
      final progress = ClassificationProgressFixture();
      final service = ClassificationReviewFixture();
      final scope = classificationScope(ids: ["a"]);
      final session = LibraryClassificationReviewSession(progress, scope.query);
      final entry = session.open(organizationTarget("a"));
      final selection = LibraryClassificationSelection(
        service,
        scope,
        reviewEntry: entry,
      );
      await selection.prepare();
      expect(progress.scopes, isEmpty);
      await selection.apply("defer", const LibraryDeferClassification());
      expect(progress.scopes, [scope.query]);
      expect(progress.targets, [entry.target]);
      expect(service.drafts.first.reviewPositions.single.sequence, BigInt.two);
      selection.dispose();
      final refreshed = LibraryClassificationSelection(
        service,
        scope,
        reviewEntry: entry,
      );
      await refreshed.prepare();
      await refreshed.apply(
        "correct",
        const LibraryCorrectClassification("photo"),
      );
      expect(progress.targets, hasLength(1));
      expect(
        service.drafts.last.reviewPositions.single.sequence,
        BigInt.from(3),
      );
      refreshed.dispose();
      await session.close();
      expect(progress.ended, ["review"]);
    },
  );

  test(
    "a late session opening cannot checkpoint the image already left",
    () async {
      final progress = ClassificationProgressFixture()
        ..pendingBegin = Completer<LibraryClassificationPosition>();
      final session = LibraryClassificationReviewSession(
        progress,
        classificationScope().query,
      );
      final first = session.open(organizationTarget("a"));
      final pending = session.checkpoint(first);
      final rejected = expectLater(
        pending,
        throwsA(isA<LibraryClassificationFailure>()),
      );
      final second = session.open(organizationTarget("b"));
      progress.pendingBegin!.complete(
        LibraryClassificationPosition("review", BigInt.zero),
      );
      await rejected;
      await session.checkpoint(second);
      expect(progress.targets, [second.target]);
      expect(progress.positions.single.sequence, BigInt.two);
      await session.close();
    },
  );

  test(
    "navigation reserves later authority while an older save completes",
    () async {
      final progress = ClassificationProgressFixture();
      final session = LibraryClassificationReviewSession(
        progress,
        classificationScope().query,
      );
      final first = session.open(organizationTarget("a"));
      final firstPosition = await session.checkpoint(first);
      final second = session.open(organizationTarget("b"));
      final secondPosition = await session.checkpoint(second);
      session.accept(
        first,
        LibraryClassificationProgressReceipt(
          const LibraryClassificationReceipt("first", 1),
          LibraryClassificationPosition(
            "review",
            firstPosition.sequence + BigInt.one,
          ),
        ),
      );
      expect(secondPosition.sequence, BigInt.from(4));
      expect(second.committed, isFalse);
      final third = session.open(organizationTarget("c"));
      expect((await session.checkpoint(third)).sequence, BigInt.from(6));
      await session.close();
    },
  );

  test(
    "disposing during begin joins its retirement without a late checkpoint",
    () async {
      final progress = ClassificationProgressFixture()
        ..pendingBegin = Completer<LibraryClassificationPosition>();
      final session = LibraryClassificationReviewSession(
        progress,
        classificationScope().query,
      );
      final entry = session.open(organizationTarget("a"));
      final rejected = expectLater(
        session.checkpoint(entry),
        throwsA(isA<LibraryClassificationFailure>()),
      );
      final closed = session.close();
      progress.pendingBegin!.complete(
        LibraryClassificationPosition("old", BigInt.zero),
      );
      await rejected;
      await closed;
      await session.close();
      expect(progress.ended, ["old"]);
      expect(progress.targets, isEmpty);
    },
  );

  test(
    "checkpoint retry retains the same requested position after failure",
    () async {
      final progress = ClassificationProgressFixture()
        ..pendingCheckpoint = Completer<void>();
      final session = LibraryClassificationReviewSession(
        progress,
        classificationScope().query,
      );
      final entry = session.open(organizationTarget("a"));
      final rejected = expectLater(session.checkpoint(entry), throwsStateError);
      await Future<void>.delayed(Duration.zero);
      progress.pendingCheckpoint!.completeError(
        StateError("temporarily unavailable"),
      );
      await rejected;
      progress.pendingCheckpoint = null;
      final retried = await session.checkpoint(entry);
      expect(progress.positions.first, same(retried));
      expect(progress.positions.last, same(retried));
      await session.close();
    },
  );
}
