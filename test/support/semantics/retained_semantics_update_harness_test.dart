import "package:flutter_test/flutter_test.dart";

import "retained_semantics_update_harness.dart";

void main() {
  final validator = RetainedSemanticsUpdateValidator.instance;

  setUp(validator.reset);

  test("accepts complete and partial retained tree updates", () {
    validator.apply({
      0: _node([1, 2]),
      1: _node(const []),
      2: _node([3]),
      3: _node(const []),
    });
    validator.apply({2: _node(const [])});

    validator.verifyLatestUpdate();
  });

  test("rejects a disconnected initial batch without root zero", () {
    expect(
      () => validator.apply({4: _node(const [])}),
      throwsA(_failureContaining("does not contain root 0")),
    );
  });

  test("rejects a child missing from the retained tree", () {
    expect(
      () => validator.apply({
        0: _node([1]),
      }),
      throwsA(_failureContaining("child 1 is missing")),
    );
  });

  test("rejects a child with multiple parents", () {
    expect(
      () => validator.apply({
        0: _node([1, 2]),
        1: _node([3]),
        2: _node([3]),
        3: _node(const []),
      }),
      throwsA(_failureContaining("child 3 has multiple parents")),
    );
  });

  test("rejects a retained cycle introduced by a partial update", () {
    validator.apply({
      0: _node([1]),
      1: _node([2]),
      2: _node(const []),
    });

    expect(
      () => validator.apply({
        1: _node([2]),
        2: _node([1]),
      }),
      throwsA(_failureContaining("cycle detected")),
    );
  });

  test("rejects an updated orphan", () {
    expect(
      () => validator.apply({0: _node(const []), 2: _node(const [])}),
      throwsA(_failureContaining("unreachable from root 0: [2]")),
    );
  });

  test("rejects a Windows reparent without a matching child update", () {
    validator.apply({
      0: _node([1, 2]),
      1: _node([3]),
      2: _node(const []),
      3: _node(const []),
    });

    expect(
      () => validator.apply({
        1: _node(const []),
        2: _node([3]),
      }),
      throwsA(_failureContaining("reparented child 3")),
    );
  });

  test("accepts a Windows reparent with matching parent and child updates", () {
    validator.apply({
      0: _node([1, 2]),
      1: _node([3]),
      2: _node(const []),
      3: _node(const []),
    });
    validator.apply({
      1: _node(const []),
      2: _node([3]),
      3: _node(const []),
    });

    validator.verifyLatestUpdate();
  });

  test("removes a reparented child from its retained old parent", () {
    validator.apply({
      0: _node([1, 2]),
      1: _node([3]),
      2: _node(const []),
      3: _node(const []),
    });
    validator.apply({
      2: _node([3]),
      3: _node(const []),
    });

    validator.verifyLatestUpdate();
  });

  test("prunes detached retained children before identifiers are reused", () {
    validator.apply({
      0: _node([1]),
      1: _node([2]),
      2: _node(const []),
    });
    validator.apply({0: _node(const [])});
    validator.apply({
      0: _node([1]),
      1: _node(const []),
    });

    validator.verifyLatestUpdate();
  });

  test("treats traversal parent as diagnostics rather than an AXTree edge", () {
    validator.apply({
      0: _node([1, 2]),
      1: _node(const [], traversalParent: 7),
      2: _node(const [], traversalParent: 2),
    });

    validator.verifyLatestUpdate();
  });
}

RetainedSemanticsNodeUpdate _node(
  List<int> traversalChildren, {
  int? traversalParent,
}) {
  return RetainedSemanticsNodeUpdate(
    traversalChildren: traversalChildren,
    traversalParent: traversalParent,
  );
}

Matcher _failureContaining(String text) {
  return isA<TestFailure>().having(
    (failure) => failure.message,
    "message",
    contains(text),
  );
}
