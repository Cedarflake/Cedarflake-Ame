import "dart:async";

import "package:cedarflake_ame/features/library/application/library_query_update.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_synchronization_refresh.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("queued revisions coalesce to the highest unpublished value", (
    tester,
  ) async {
    final fixture = _RefreshFixture();
    addTearDown(fixture.refresh.dispose);
    fixture.refresh.queueRevision(BigInt.two);
    fixture.refresh.queueRevision(BigInt.from(5));
    fixture.refresh.queueRevision(BigInt.from(3));
    await tester.pump(Duration.zero);
    expect(fixture.revisions, [BigInt.from(5)]);

    fixture.complete(
      0,
      LibraryQueryUpdateOutcome.applied,
      publishedRevision: 5,
    );
    await tester.pump(Duration.zero);
    fixture.refresh.queueRevision(BigInt.from(4));
    await tester.pump(Duration.zero);
    expect(fixture.revisions, [BigInt.from(5)]);
    expect(fixture.failures, isEmpty);
  });

  testWidgets("a newer revision queued during failure continues once", (
    tester,
  ) async {
    final fixture = _RefreshFixture();
    addTearDown(fixture.refresh.dispose);
    fixture.refresh.queueRevision(BigInt.two);
    await tester.pump(Duration.zero);
    fixture.refresh.queueRevision(BigInt.from(4));
    fixture.refresh.queueRevision(BigInt.from(3));
    fixture.complete(0, LibraryQueryUpdateOutcome.failed);
    await tester.pump(Duration.zero);
    await tester.pump(Duration.zero);
    expect(fixture.revisions, [BigInt.two, BigInt.from(4)]);
    expect(fixture.failures, [true]);

    fixture.complete(
      1,
      LibraryQueryUpdateOutcome.applied,
      publishedRevision: 4,
    );
    await tester.pump(Duration.zero);
    await tester.pump(const Duration(seconds: 1));
    expect(fixture.revisions, hasLength(2));
    expect(fixture.failures, [true, false]);
  });

  testWidgets("failure waits for retry and reports only feedback changes", (
    tester,
  ) async {
    final fixture = _RefreshFixture();
    addTearDown(fixture.refresh.dispose);
    fixture.refresh.queueRevision(BigInt.two);
    await tester.pump(Duration.zero);
    fixture.complete(0, LibraryQueryUpdateOutcome.failed);
    await tester.pump(Duration.zero);
    await tester.pump(const Duration(seconds: 3));
    expect(fixture.revisions, hasLength(1));
    fixture.refresh.retry();
    await tester.pump(Duration.zero);
    fixture.complete(1, LibraryQueryUpdateOutcome.failed);
    await tester.pump(Duration.zero);
    expect(fixture.failures, [true]);

    fixture.refresh.retry();
    await tester.pump(Duration.zero);
    fixture.complete(
      2,
      LibraryQueryUpdateOutcome.applied,
      publishedRevision: 2,
    );
    await tester.pump(Duration.zero);
    expect(fixture.failures, [true, false]);
  });

  for (final outcome in [
    LibraryQueryUpdateOutcome.busy,
    LibraryQueryUpdateOutcome.superseded,
  ]) {
    testWidgets("$outcome retains the revision for a delayed retry", (
      tester,
    ) async {
      final fixture = _RefreshFixture();
      addTearDown(fixture.refresh.dispose);
      fixture.refresh.queueRevision(BigInt.two);
      await tester.pump(Duration.zero);
      fixture.complete(0, outcome);
      await tester.pump(Duration.zero);
      await tester.pump(const Duration(milliseconds: 249));
      expect(fixture.revisions, hasLength(1));
      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump(Duration.zero);
      expect(fixture.revisions, [BigInt.two, BigInt.two]);
      expect(fixture.failures, isEmpty);
      fixture.complete(
        1,
        LibraryQueryUpdateOutcome.applied,
        publishedRevision: 2,
      );
      await tester.pump(Duration.zero);
    });
  }

  testWidgets("browse admission defers reads without dropping the revision", (
    tester,
  ) async {
    final fixture = _RefreshFixture();
    addTearDown(fixture.refresh.dispose);
    fixture.state = fixture.state.copyWith(isLoadingTimeAnchor: true);
    fixture.refresh.queueRevision(BigInt.two);
    await tester.pump(Duration.zero);
    expect(fixture.revisions, isEmpty);
    fixture.state = fixture.state.copyWith(isLoadingTimeAnchor: false);
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pump(Duration.zero);
    expect(fixture.revisions, [BigInt.two]);
    fixture.complete(
      0,
      LibraryQueryUpdateOutcome.applied,
      publishedRevision: 2,
    );
    await tester.pump(Duration.zero);
  });

  testWidgets(
    "applied outcome still requires the requested published revision",
    (tester) async {
      final fixture = _RefreshFixture();
      addTearDown(fixture.refresh.dispose);
      fixture.refresh.queueRevision(BigInt.two);
      await tester.pump(Duration.zero);
      fixture.complete(0, LibraryQueryUpdateOutcome.applied);
      await tester.pump(Duration.zero);
      await tester.pump(const Duration(seconds: 1));
      expect(fixture.revisions, hasLength(1));
      expect(fixture.failures, [true]);
      fixture.refresh.retry();
      await tester.pump(Duration.zero);
      expect(fixture.revisions, [BigInt.two, BigInt.two]);
      fixture.complete(
        1,
        LibraryQueryUpdateOutcome.applied,
        publishedRevision: 2,
      );
      await tester.pump(Duration.zero);
    },
  );

  for (final inFlight in [false, true]) {
    testWidgets("disposal retires pending work, in flight: $inFlight", (
      tester,
    ) async {
      final fixture = _RefreshFixture();
      fixture.refresh.queueRevision(BigInt.two);
      if (inFlight) {
        await tester.pump(Duration.zero);
        fixture.refresh.queueRevision(BigInt.from(3));
      }
      fixture.refresh.dispose();
      fixture.isRetired = true;
      fixture.refresh.retry();
      fixture.refresh.queueRevision(BigInt.from(4));
      if (inFlight) {
        fixture.complete(0, LibraryQueryUpdateOutcome.failed);
      }
      await tester.pump(Duration.zero);
      await tester.pump(const Duration(seconds: 1));
      expect(fixture.revisions, hasLength(inFlight ? 1 : 0));
      expect(fixture.failures, isEmpty);
    });
  }
}

class _RefreshFixture {
  _RefreshFixture() {
    refresh = LibrarySynchronizationRefresh(
      readState: () {
        expectSync(isRetired, isFalse);
        return state;
      },
      refresh: (revision) {
        revisions.add(revision);
        final result = Completer<LibraryQueryUpdateOutcome>();
        results.add(result);
        return result.future;
      },
      onFailureChanged: failures.add,
    );
  }

  late final LibrarySynchronizationRefresh refresh;
  LibraryState state = LibraryState(catalogRevision: BigInt.one);
  final List<BigInt> revisions = [];
  final List<Completer<LibraryQueryUpdateOutcome>> results = [];
  final List<bool> failures = [];
  bool isRetired = false;

  void complete(
    int attempt,
    LibraryQueryUpdateOutcome outcome, {
    int? publishedRevision,
  }) {
    if (publishedRevision != null) {
      state = state.copyWith(catalogRevision: BigInt.from(publishedRevision));
    }
    results[attempt].complete(outcome);
  }
}
