import "dart:async";

import "package:cedarflake_ame/features/library/application/library_review_requests.dart";
import "package:cedarflake_ame/features/library/application/library_review_session.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_review.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";
import "../support/library_review_fixture.dart";

void main() {
  late ReviewServiceFixture service;
  late OrganizationServiceFixture organization;
  late LibraryReviewSession session;
  const query = LibraryGalleryQuery(
    organizationQueue: LibraryOrganizationQueue.pending,
  );

  setUp(() {
    service = ReviewServiceFixture();
    organization = OrganizationServiceFixture();
    session = LibraryReviewSession(service, query);
  });
  tearDown(() => session.close());

  test(
    "mode entry alone is inert and late start checkpoints only the latest image",
    () async {
      expect(service.starts, isEmpty);
      final started = Completer<LibraryReviewPosition>();
      service.starting = () => started.future;
      final first = session.open(organizationTarget("first"));
      final stale = expectLater(
        session.checkpoint(first),
        throwsA(isA<LibraryOrganizationFailure>()),
      );
      final second = session.open(organizationTarget("second"));
      final current = session.checkpoint(second);
      started.complete(LibraryReviewPosition("started", BigInt.zero));
      await stale;
      await current;
      expect(service.starts, [query]);
      expect(service.checkpoints.single.target, second.target);
    },
  );

  test(
    "unknown checkpoint retries its identity rather than creating a later position",
    () async {
      final entry = session.open(organizationTarget("one"));
      service.checkpointing = (_) async =>
          throw StateError("Receipt unavailable");
      await expectLater(session.checkpoint(entry), throwsStateError);
      service.checkpointing = null;
      await session.checkpoint(entry);
      expect(service.starts, hasLength(1));
      expect(
        service.checkpoints[1].position,
        same(service.checkpoints[0].position),
      );
    },
  );

  test(
    "unknown save retains the operation, prepared target and saved position",
    () async {
      final entry = session.open(organizationTarget("one"));
      final request = LibraryReviewRequest(
        session,
        entry,
        const LibraryReviewProgress(LibraryProgressAction.defer),
      );
      service.writing = (_) async => throw StateError("Receipt unavailable");
      await expectLater(request.execute(organization), throwsStateError);
      expect(entry.committed, isFalse);
      service.writing = null;
      await request.execute(organization);
      expect(entry.committed, isTrue);
      expect(service.writes[1].id, service.writes[0].id);
      expect(service.writes[1].position, same(service.writes[0].position));
      expect(service.writes[1].prepared, same(service.writes[0].prepared));
      expect(service.checkpoints, hasLength(1));
      expect(organization.preparationCount, 1);
      expect(organization.writes, isEmpty);
    },
  );

  test(
    "closing a pending start retires it without a checkpoint or later write",
    () async {
      final started = Completer<LibraryReviewPosition>();
      service.starting = () => started.future;
      final entry = session.open(organizationTarget("one"));
      final rejected = expectLater(
        session.checkpoint(entry),
        throwsA(isA<LibraryOrganizationFailure>()),
      );
      final closing = session.close();
      started.complete(LibraryReviewPosition("old", BigInt.zero));
      await rejected;
      await closing;
      expect(service.ends, ["old"]);
      expect(service.checkpoints, isEmpty);
    },
  );

  test(
    "changing images during preparation cannot write the previous selection",
    () async {
      organization.preparing = Completer<LibraryOrganizationPrepared>();
      final entry = session.open(organizationTarget("old"));
      final request = LibraryReviewRequest(
        session,
        entry,
        const LibraryReviewAlbum(LibraryOrganizationAlbum("a", "A")),
      );
      final rejected = expectLater(
        request.execute(organization),
        throwsA(isA<LibraryOrganizationFailure>()),
      );
      await Future<void>.delayed(Duration.zero);
      session.open(organizationTarget("new"));
      organization.preparing!.complete(const OrganizationPreparedFixture());
      await rejected;
      expect(service.writes, isEmpty);
    },
  );

  test(
    "an in-flight commit cannot collide with or retire a newer checkpoint",
    () async {
      final committed = Completer<LibraryReviewReceipt>();
      service.writing = (_) => committed.future;
      final old = session.open(organizationTarget("old"));
      final request = LibraryReviewRequest(
        session,
        old,
        const LibraryReviewProgress(LibraryProgressAction.ignore),
      );
      final saving = request.execute(organization);
      await Future<void>.delayed(Duration.zero);
      final newer = session.open(organizationTarget("new"));
      final newerPosition = await session.checkpoint(newer);
      final receipt = service.receipt(service.writes.single);
      expect(newerPosition.sequence, greaterThan(receipt.position.sequence));
      committed.complete(receipt);
      await saving;
      expect(session.current, same(newer));
      expect(newer.committed, isFalse);
      final next = session.open(organizationTarget("third"));
      expect(
        (await session.checkpoint(next)).sequence,
        greaterThan(newerPosition.sequence),
      );
    },
  );

  test(
    "a confirmed stale selection prepares again without replacing its checkpoint",
    () async {
      final entry = session.open(organizationTarget("one"));
      final request = LibraryReviewRequest(
        session,
        entry,
        const LibraryReviewProgress(LibraryProgressAction.restore),
      );
      service.writing = (_) async => throw const LibraryOrganizationFailure(
        "organization_selection_changed",
        "Changed",
      );
      await expectLater(
        request.execute(organization),
        throwsA(isA<LibraryOrganizationFailure>()),
      );
      request.refreshAuthority();
      service.writing = null;
      await request.execute(organization);
      expect(service.writes[1].id, isNot(service.writes[0].id));
      expect(organization.preparationCount, 2);
      expect(service.checkpoints, hasLength(1));
    },
  );
}
