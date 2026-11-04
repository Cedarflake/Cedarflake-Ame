import "dart:async";

import "package:cedarflake_ame/features/library/application/library_similar_review_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_similar_review.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/similar_review_fixtures.dart";

void main() {
  test("degree and ignored transitions reject old query publication", () async {
    final service = SimilarReviewFixture();
    final review = LibrarySimilarReviewController(service);
    addTearDown(review.dispose);
    final held = Completer<LibrarySimilarGroups>();
    service.pendingGroups = held;
    final old = review.refresh();
    await Future<void>.value();
    expect(service.queries.single.degree, LibrarySimilarityDegree.standard);
    service.pendingGroups = null;
    await review.showGroups(degree: LibrarySimilarityDegree.strict);
    held.complete(
      LibrarySimilarGroups(groups: [service.group("old")], total: 1),
    );
    await old;
    expect(review.groupWindow.itemAt(0)?.id, "similar-one");
    expect(review.degree, LibrarySimilarityDegree.strict);
    service.decision = LibrarySimilarDecision(
      revision: BigInt.one,
      ignored: true,
    );
    await review.showGroups(ignoredGroups: true);
    expect(service.queries.last, (
      degree: LibrarySimilarityDegree.broad,
      ignored: true,
    ));
    await review.showGroup(service.group());
    expect(service.memberQueries.last.degree, LibrarySimilarityDegree.broad);
    expect(review.memberWindow.itemAt(0)?.locationCount, 3);
    await review.showGroups(ignoredGroups: false);
    expect(review.degree, LibrarySimilarityDegree.strict);
    expect(review.effectiveDegree, LibrarySimilarityDegree.strict);
  });

  test(
    "pending writes keep their context and cannot replace a later member view",
    () async {
      final service = SimilarReviewFixture();
      final review = LibrarySimilarReviewController(service);
      addTearDown(review.dispose);
      await review.showGroup(service.group());
      final held = Completer<LibrarySimilarDecision>();
      service.pendingWrite = held;
      final save = review.prefer(service.members.first);
      expect(review.choices.saving, isTrue);
      expect(await review.setIgnored(true), isFalse);
      expect(service.writes, 1);
      expect(service.lastContext?.groupId, "similar-one");
      expect(service.lastContext?.revision, BigInt.zero);
      await review.catalogChanged();
      expect(service.memberQueries, hasLength(1));
      service.decision = LibrarySimilarDecision(revision: BigInt.from(7));
      await review.showGroup(service.group("new"));
      held.complete(
        LibrarySimilarDecision(
          revision: BigInt.one,
          preferred: service.members.first.content,
        ),
      );
      expect(await save, isTrue);
      expect(review.selected?.id, "new");
      expect(review.decision.revision, BigInt.from(7));
      expect(review.decision.preferred, isNull);
      expect(review.choices.saving, isFalse);
      service.pendingWrite = null;
      service.failWrite = true;
      expect(await review.prefer(service.members.first), isFalse);
      expect(review.choices.error, isNotNull);
      expect(review.decision.revision, BigInt.from(7));
      service.failWrite = false;
      expect(await review.prefer(service.members.last), isTrue);
      expect(review.choices.error, isNull);
      expect(service.lastContext?.revision, BigInt.from(7));
      expect(review.decision.preferred, service.members.last.content);
    },
  );

  test(
    "exclusion refreshes membership and its undo remains scoped after returning to groups",
    () async {
      final service = SimilarReviewFixture();
      final review = LibrarySimilarReviewController(service);
      addTearDown(review.dispose);
      await review.showGroup(service.group());
      final member = service.members.first;
      expect(await review.prefer(member), isTrue);
      final undo = await review.exclude(member);
      expect(undo, isNotNull);
      expect(undo!.content, member.content);
      expect(review.itemCount, 1);
      expect(review.decision.preferred, member.content);
      expect(review.memberWindow.itemAt(0)?.content, isNot(member.content));
      await review.showGroups();
      expect(review.itemCount, 0);
      expect((await review.undo(undo))?.restored, isTrue);
      expect(service.lastUndo, same(undo));
      expect(review.itemCount, 1);
      await review.showGroup(service.group());
      expect(await review.setIgnored(true), isTrue);
      expect(review.selected?.id, "similar-one");
      expect(review.decision.ignored, isTrue);
      await review.showGroups(ignoredGroups: true);
      expect(review.itemCount, 1);
      service.restoreUndo = false;
      expect((await review.undo(undo))?.restored, isFalse);
      expect(review.itemCount, 1);
      expect(service.members, hasLength(2));
    },
  );
}
