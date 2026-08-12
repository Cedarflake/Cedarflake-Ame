import "dart:typed_data";
import "dart:ui" as ui;

import "package:flutter/semantics.dart";
import "package:flutter_test/flutter_test.dart";

final class RetainedSemanticsUpdateBinding
    extends AutomatedTestWidgetsFlutterBinding {
  @override
  ui.SemanticsUpdateBuilder createSemanticsUpdateBuilder() {
    return RetainedSemanticsUpdateBuilder();
  }
}

final class RetainedSemanticsUpdateBuilder extends Fake
    implements ui.SemanticsUpdateBuilder {
  final ui.SemanticsUpdateBuilder _delegate = ui.SemanticsUpdateBuilder();
  final Map<int, RetainedSemanticsNodeUpdate> _updates = {};

  @override
  void updateNode({
    required int id,
    required SemanticsFlags flags,
    required int actions,
    required int maxValueLength,
    required int currentValueLength,
    required int textSelectionBase,
    required int textSelectionExtent,
    required int platformViewId,
    required int scrollChildren,
    required int scrollIndex,
    required int traversalParent,
    required double scrollPosition,
    required double scrollExtentMax,
    required double scrollExtentMin,
    required Rect rect,
    required String identifier,
    required String label,
    required List<StringAttribute> labelAttributes,
    required String value,
    required List<StringAttribute> valueAttributes,
    required String increasedValue,
    required List<StringAttribute> increasedValueAttributes,
    required String decreasedValue,
    required List<StringAttribute> decreasedValueAttributes,
    required String hint,
    required List<StringAttribute> hintAttributes,
    required String tooltip,
    required TextDirection? textDirection,
    required Float64List transform,
    required Float64List hitTestTransform,
    required Int32List childrenInTraversalOrder,
    required Int32List childrenInHitTestOrder,
    required Int32List additionalActions,
    int headingLevel = 0,
    String linkUrl = "",
    SemanticsRole role = SemanticsRole.none,
    required List<String>? controlsNodes,
    SemanticsValidationResult validationResult = SemanticsValidationResult.none,
    ui.SemanticsHitTestBehavior hitTestBehavior =
        ui.SemanticsHitTestBehavior.defer,
    required ui.SemanticsInputType inputType,
    required ui.Locale? locale,
    required String minValue,
    required String maxValue,
  }) {
    _updates[id] = RetainedSemanticsNodeUpdate(
      traversalChildren: childrenInTraversalOrder.toList(growable: false),
      hitTestChildren: childrenInHitTestOrder.toList(growable: false),
      traversalParent: traversalParent,
      identifier: identifier,
      label: label,
      tooltip: tooltip,
    );
    _delegate.updateNode(
      id: id,
      flags: flags,
      actions: actions,
      maxValueLength: maxValueLength,
      currentValueLength: currentValueLength,
      textSelectionBase: textSelectionBase,
      textSelectionExtent: textSelectionExtent,
      platformViewId: platformViewId,
      scrollChildren: scrollChildren,
      scrollIndex: scrollIndex,
      traversalParent: traversalParent,
      scrollPosition: scrollPosition,
      scrollExtentMax: scrollExtentMax,
      scrollExtentMin: scrollExtentMin,
      rect: rect,
      identifier: identifier,
      label: label,
      labelAttributes: labelAttributes,
      value: value,
      valueAttributes: valueAttributes,
      increasedValue: increasedValue,
      increasedValueAttributes: increasedValueAttributes,
      decreasedValue: decreasedValue,
      decreasedValueAttributes: decreasedValueAttributes,
      hint: hint,
      hintAttributes: hintAttributes,
      tooltip: tooltip,
      textDirection: textDirection,
      transform: transform,
      hitTestTransform: hitTestTransform,
      childrenInTraversalOrder: childrenInTraversalOrder,
      childrenInHitTestOrder: childrenInHitTestOrder,
      additionalActions: additionalActions,
      headingLevel: headingLevel,
      linkUrl: linkUrl,
      role: role,
      controlsNodes: controlsNodes,
      validationResult: validationResult,
      hitTestBehavior: hitTestBehavior,
      inputType: inputType,
      locale: locale,
      minValue: minValue,
      maxValue: maxValue,
    );
  }

  @override
  void updateCustomAction({
    required int id,
    String? label,
    String? hint,
    int overrideId = -1,
  }) {
    _delegate.updateCustomAction(
      id: id,
      label: label,
      hint: hint,
      overrideId: overrideId,
    );
  }

  @override
  ui.SemanticsUpdate build() {
    RetainedSemanticsUpdateValidator.instance.apply(_updates);
    return _delegate.build();
  }
}

final class RetainedSemanticsNodeUpdate {
  const RetainedSemanticsNodeUpdate({
    required this.traversalChildren,
    this.hitTestChildren = const [],
    this.traversalParent,
    this.identifier = "",
    this.label = "",
    this.tooltip = "",
  });

  final List<int> traversalChildren;
  final List<int> hitTestChildren;
  final int? traversalParent;
  final String identifier;
  final String label;
  final String tooltip;

  RetainedSemanticsNodeUpdate withTraversalChildren(List<int> children) {
    return RetainedSemanticsNodeUpdate(
      traversalChildren: children,
      hitTestChildren: hitTestChildren,
      traversalParent: traversalParent,
      identifier: identifier,
      label: label,
      tooltip: tooltip,
    );
  }
}

final class RetainedSemanticsUpdateValidator {
  RetainedSemanticsUpdateValidator._();

  static final instance = RetainedSemanticsUpdateValidator._();

  final Map<int, RetainedSemanticsNodeUpdate> _nodes = {};
  final Map<int, RetainedSemanticsNodeUpdate> _lastSeenNodes = {};
  final Map<int, List<String>> _nodeEvents = {};
  final List<String> _checkpoints = [];
  var _updateSequence = 0;
  String? _latestFailure;

  void reset() {
    _nodes.clear();
    _lastSeenNodes.clear();
    _nodeEvents.clear();
    _checkpoints.clear();
    _latestFailure = null;
    _updateSequence = 0;
  }

  void apply(Map<int, RetainedSemanticsNodeUpdate> updates) {
    if (_latestFailure != null) {
      return;
    }
    _updateSequence += 1;
    final previousParents = _traversalParentsFor(_nodes);
    final candidateNodes = Map<int, RetainedSemanticsNodeUpdate>.of(_nodes)
      ..addAll(updates);
    _lastSeenNodes.addAll(updates);
    for (final entry in updates.entries) {
      _recordNodeEvent(
        entry.key,
        "$_updateSequence updated ${_describeNode(entry.value)}",
      );
    }
    try {
      if (_nodes.isEmpty && !updates.containsKey(0)) {
        throw _failure(
          "Initial Windows semantics update does not contain root 0",
          updates,
          updates.keys.isEmpty ? const [0] : [updates.keys.first],
        );
      }
      _validateWindowsReparentPreconditions(updates, previousParents);
      _applyEngineReparents(candidateNodes, updates, previousParents);
      _validateCandidateCycles(candidateNodes, updates);
      final reachable = _validateTraversalTree(candidateNodes, updates);
      final detachedIds = candidateNodes.keys
          .where((id) => !reachable.contains(id))
          .toList(growable: false);
      for (final id in detachedIds) {
        _recordNodeEvent(
          id,
          "$_updateSequence pruned ${_describeNode(candidateNodes[id])}",
        );
        candidateNodes.remove(id);
      }
      _nodes
        ..clear()
        ..addAll(candidateNodes);
    } on TestFailure catch (failure) {
      _latestFailure = failure.message;
      rethrow;
    }
  }

  void _applyEngineReparents(
    Map<int, RetainedSemanticsNodeUpdate> candidateNodes,
    Map<int, RetainedSemanticsNodeUpdate> updates,
    Map<int, int> previousParents,
  ) {
    for (final parentEntry in updates.entries) {
      for (final childId in parentEntry.value.traversalChildren) {
        final previousParentId = previousParents[childId];
        if (previousParentId == null || previousParentId == parentEntry.key) {
          continue;
        }
        final previousParent = candidateNodes[previousParentId];
        if (previousParent == null) {
          continue;
        }
        candidateNodes[previousParentId] = previousParent.withTraversalChildren(
          previousParent.traversalChildren
              .where((candidateId) => candidateId != childId)
              .toList(growable: false),
        );
      }
    }
  }

  void _validateWindowsReparentPreconditions(
    Map<int, RetainedSemanticsNodeUpdate> updates,
    Map<int, int> previousParents,
  ) {
    for (final parentEntry in updates.entries) {
      for (final childId in parentEntry.value.traversalChildren) {
        final previousParent = previousParents[childId];
        if (previousParent == null || previousParent == parentEntry.key) {
          continue;
        }
        if (!updates.containsKey(childId)) {
          throw _failure(
            "Windows reparented child $childId from $previousParent to "
            "${parentEntry.key} without a matching child update",
            updates,
            [previousParent, parentEntry.key, childId],
          );
        }
      }
    }
  }

  Map<int, int> _traversalParentsFor(
    Map<int, RetainedSemanticsNodeUpdate> nodes,
  ) {
    final parents = <int, int>{};
    for (final parentEntry in nodes.entries) {
      for (final childId in parentEntry.value.traversalChildren) {
        parents[childId] = parentEntry.key;
      }
    }
    return parents;
  }

  void _validateCandidateCycles(
    Map<int, RetainedSemanticsNodeUpdate> candidateNodes,
    Map<int, RetainedSemanticsNodeUpdate> updates,
  ) {
    final active = <int>{};
    final completed = <int>{};

    void visit(int id, List<int> path) {
      if (completed.contains(id)) {
        return;
      }
      active.add(id);
      final childIds = candidateNodes[id]!.traversalChildren;
      for (final childId in childIds) {
        if (!candidateNodes.containsKey(childId)) {
          continue;
        }
        if (active.contains(childId)) {
          throw _failure("Traversal cycle detected", updates, [
            ...path,
            id,
            childId,
          ]);
        }
        visit(childId, [...path, id]);
      }
      active.remove(id);
      completed.add(id);
    }

    final candidateIds = candidateNodes.keys.toList(growable: false)..sort();
    for (final id in candidateIds) {
      visit(id, const []);
    }
  }

  void verifyLatestUpdate({String? trace}) {
    if (trace != null) {
      _checkpoints.add("$_updateSequence $trace");
      if (_checkpoints.length > 12) {
        _checkpoints.removeAt(0);
      }
    }
    if (_latestFailure case final failure?) {
      throw TestFailure(
        trace == null
            ? "$failure; checkpoints=$_checkpoints"
            : "$failure; trace=$trace; checkpoints=$_checkpoints",
      );
    }
  }

  Set<int> _validateTraversalTree(
    Map<int, RetainedSemanticsNodeUpdate> candidateNodes,
    Map<int, RetainedSemanticsNodeUpdate> updates,
  ) {
    final parents = <int, int>{};
    final reachable = <int>{};
    final active = <int>{};

    void visit(int id, List<int> path) {
      final node = candidateNodes[id];
      if (node == null) {
        throw _failure(
          "Traversal child $id is missing from the retained tree; "
          "events=${_nodeEvents[id] ?? const []}",
          updates,
          [...path, id],
        );
      }
      if (!active.add(id)) {
        throw _failure("Traversal cycle detected", updates, [...path, id]);
      }
      reachable.add(id);
      final childIds = node.traversalChildren;
      final distinctChildren = <int>{};
      for (final childId in childIds) {
        if (!distinctChildren.add(childId)) {
          throw _failure(
            "Traversal parent $id lists child $childId more than once",
            updates,
            [...path, id, childId],
          );
        }
        if (active.contains(childId)) {
          throw _failure("Traversal cycle detected", updates, [
            ...path,
            id,
            childId,
          ]);
        }
        final previousParent = parents.putIfAbsent(childId, () => id);
        if (previousParent != id) {
          throw _failure(
            "Traversal child $childId has multiple parents "
            "$previousParent and $id",
            updates,
            [...path, id, childId],
          );
        }
        visit(childId, [...path, id]);
      }
      active.remove(id);
    }

    visit(0, const []);
    final updatedOrphans =
        updates.keys
            .where((id) => !reachable.contains(id))
            .toList(growable: false)
          ..sort();
    if (updatedOrphans.isNotEmpty) {
      throw _failure(
        "Traversal update contains nodes unreachable from root 0: "
        "$updatedOrphans",
        updates,
        const [0],
      );
    }
    return reachable;
  }

  TestFailure _failure(
    String reason,
    Map<int, RetainedSemanticsNodeUpdate> updates,
    List<int> path,
  ) {
    return TestFailure(
      "$reason; path=${path.join(" -> ")}; "
      "lastSeen=${_describeNode(_lastSeenNodes[path.last])}; "
      "checkpoints=$_checkpoints; "
      "updates=${_describe(updates)}; retained=${_describe(_nodes)}",
    );
  }

  String _describe(Map<int, RetainedSemanticsNodeUpdate> nodes) {
    final ids = nodes.keys.toList(growable: false)..sort();
    return "{${ids.map((id) {
      final node = nodes[id]!;
      return "$id:t${node.traversalChildren}/h${node.hitTestChildren}"
          "/p${node.traversalParent}/${_describeNode(node)}";
    }).join(", ")}}";
  }

  String _describeNode(RetainedSemanticsNodeUpdate? node) {
    if (node == null) {
      return "never-seen";
    }
    return "id=${node.identifier},label=${node.label},tooltip=${node.tooltip}";
  }

  void _recordNodeEvent(int id, String event) {
    final events = _nodeEvents.putIfAbsent(id, () => []);
    events.add(event);
    if (events.length > 8) {
      events.removeAt(0);
    }
  }
}
