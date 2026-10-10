import "package:cedarflake_ame/features/library/application/library_preview_order.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("priority outranks demand rank and arrival sequence", () {
    const priorityOrder = [
      LibraryPreviewPriority.viewer,
      LibraryPreviewPriority.visible,
      LibraryPreviewPriority.nearDirection,
      LibraryPreviewPriority.guard,
      LibraryPreviewPriority.idle,
    ];
    for (var higher = 0; higher < priorityOrder.length; higher += 1) {
      for (var lower = higher + 1; lower < priorityOrder.length; lower += 1) {
        final preferred = _Order(
          priorityOrder[higher],
          rank: null,
          sequence: 9,
        );
        final other = _Order(priorityOrder[lower], rank: 0, sequence: 0);
        expect(_isPreferred(preferred, other), isTrue);
        expect(_isPreferred(other, preferred), isFalse);
      }
    }
  });

  test("a ranked request precedes an earlier unranked request", () {
    const ranked = _Order(
      LibraryPreviewPriority.visible,
      rank: 99,
      sequence: 9,
    );
    const unranked = _Order(LibraryPreviewPriority.visible, sequence: 0);
    expect(_isPreferred(ranked, unranked), isTrue);
    expect(_isPreferred(unranked, ranked), isFalse);
  });

  test("a lower demand rank precedes an earlier arrival", () {
    const center = _Order(LibraryPreviewPriority.visible, rank: 0, sequence: 9);
    const edge = _Order(LibraryPreviewPriority.visible, rank: 1, sequence: 0);
    expect(_isPreferred(center, edge), isTrue);
    expect(_isPreferred(edge, center), isFalse);
  });

  for (final rank in <int?>[null, 0, 1]) {
    test("matching rank $rank uses arrival order and preserves exact ties", () {
      final first = _Order(
        LibraryPreviewPriority.visible,
        rank: rank,
        sequence: 0,
      );
      final later = _Order(
        LibraryPreviewPriority.visible,
        rank: rank,
        sequence: 1,
      );
      expect(_isPreferred(first, later), isTrue);
      expect(_isPreferred(later, first), isFalse);
      expect(_isPreferred(first, first), isFalse);
    });
  }

  test("3600 boundary combinations preserve the original selection policy", () {
    final orders = [
      for (final priority in LibraryPreviewPriority.values)
        for (final rank in <int?>[null, 0, 1, 2])
          for (final sequence in [0, 1, 2])
            _Order(priority, rank: rank, sequence: sequence),
    ];
    for (final candidate in orders) {
      for (final current in orders) {
        expect(
          _isPreferred(candidate, current),
          _originalSelection(candidate, current),
          reason: "candidate=$candidate, current=$current",
        );
      }
    }
  });
}

bool _isPreferred(_Order candidate, _Order current) =>
    isPreviewRequestPreferred(
      priority: candidate.priority,
      demandRank: candidate.rank,
      sequence: candidate.sequence,
      currentPriority: current.priority,
      currentDemandRank: current.rank,
      currentSequence: current.sequence,
    );

// Frozen policy from c358661; the boundary examples above independently specify its intent.
bool _originalSelection(_Order request, _Order current) =>
    request.priority.index > current.priority.index ||
    (request.priority == current.priority &&
        request.rank != null &&
        (current.rank == null || request.rank! < current.rank!)) ||
    (request.priority == current.priority &&
        request.rank == current.rank &&
        request.sequence < current.sequence);

class _Order {
  const _Order(this.priority, {this.rank, required this.sequence});

  final LibraryPreviewPriority priority;
  final int? rank;
  final int sequence;

  @override
  String toString() => "${priority.name}/$rank/$sequence";
}
