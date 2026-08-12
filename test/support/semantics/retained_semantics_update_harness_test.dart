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
