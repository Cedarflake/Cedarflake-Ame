import "package:cedarflake_ame/features/library/application/library_query_projection.dart";
import "package:cedarflake_ame/features/library/application/library_query_refresh.dart";
import "package:cedarflake_ame/features/library/domain/library_query_snapshot.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("later input retires only this registry's older read authority", () {
    final first = LibraryQueryProjections();
    final other = LibraryQueryProjections();
    final old = first.beginRead();
    expect(first.accepts(old), isTrue);
    expect(other.accepts(old), isFalse);
    first.invalidatePosition();
    expect(first.accepts(old), isFalse);
    final current = first.beginRead();
    expect(first.accepts(current), isTrue);
    first.dispose();
    expect(first.accepts(current), isFalse);
  });

  test(
    "a detached gallery leaves committed catalog refresh available",
    () async {
      final projections = LibraryQueryProjections();
      final registration = projections.attach(_Projection("old"));
      projections.detach(registration);
      var calls = 0;
      final outcome = await projections.publish((anchor) async {
        calls += 1;
        expect(anchor, isNull);
        return LibraryQueryUpdateOutcome.applied;
      });
      expect(calls, 1);
      expect(outcome, LibraryQueryUpdateOutcome.applied);
    },
  );

  test("old gallery teardown cannot detach its replacement", () async {
    final projections = LibraryQueryProjections();
    final old = projections.attach(_Projection("old"));
    projections.attach(_Projection("current"));
    projections.detach(old);
    await projections.publish((anchor) async {
      expect(anchor?.requestedLocationId, "current");
      return LibraryQueryUpdateOutcome.applied;
    });
  });

  test(
    "disposal cannot admit another read even after a late attachment",
    () async {
      final projections = LibraryQueryProjections();
      projections.dispose();
      projections.attach(_Projection("late"));
      final outcome = await projections.publish((_) async {
        fail("Disposed projection registry admitted a catalog read");
      });
      expect(outcome, LibraryQueryUpdateOutcome.superseded);
    },
  );
}

class _Projection implements LibraryQueryProjection {
  const _Projection(this.location);

  final String location;

  @override
  Future<LibraryQueryUpdateOutcome> publish(
    LibraryQueryPublication publication,
  ) => publication(LibraryQueryAnchor(requestedLocationId: location));
}
