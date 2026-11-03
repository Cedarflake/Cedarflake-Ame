import "boundary_rules.dart";

const legacyDartRevision = "f74146a4e88d72325c576504825399df109774aa";
const legacyDartInventoryBlob = "c508e7869d5954d4e6221ddfb1dafedccdf24107";

List<LegacyDependency> parseLegacyInventory(
  Map<String, Object?> document, {
  required String blobId,
}) {
  if (blobId != legacyDartInventoryBlob ||
      document["sourceRevision"] != legacyDartRevision) {
    throw const FormatException(
      "The frozen legacy inventory changed; it cannot be rebased onto current violations",
    );
  }
  final owners = document["owners"] as Map<String, Object?>;
  final entries = document["exceptions"] as List<Object?>;
  return [
    for (final value in entries)
      if (value case {"owner": final String owner})
        LegacyDependency.fromJson({
          ...owners[owner] as Map<String, Object?>,
          ...value as Map<String, Object?>,
        })
      else
        throw const FormatException(
          "Every frozen edge requires its named responsibility owner",
        ),
  ];
}

class LegacyDependency {
  const LegacyDependency({
    required this.id,
    required this.violation,
    required this.owner,
    required this.reason,
    required this.coverage,
    required this.removeIn,
  });

  final String id;
  final BoundaryViolation violation;
  final String owner;
  final String reason;
  final List<String> coverage;
  final String removeIn;

  factory LegacyDependency.fromJson(Map<String, Object?> record) {
    String field(String name) {
      final value = record[name];
      if (value is! String || value.trim().isEmpty || value.contains("*")) {
        throw FormatException("Legacy dependency requires an exact $name");
      }
      return value;
    }

    final coverage = record["coverage"];
    if (coverage is! List<Object?> ||
        coverage.isEmpty ||
        coverage.any(
          (item) => item is! String || item.isEmpty || item.contains("*"),
        )) {
      throw const FormatException(
        "Legacy dependency requires exact regression owners",
      );
    }
    final rule = field("rule");
    if (!{
      "layer",
      "external-layer",
      "shared-feature",
      "cross-feature",
      "page-dependency",
      "cycle",
    }.contains(rule)) {
      throw FormatException(
        "Unclassified or unresolved code cannot be exempted: $rule",
      );
    }
    final removeIn = field("removeIn");
    if (!{
      "O2",
      "O3",
      "O4",
      "O5.1",
      "O5.2",
      "O5.3",
      "O6",
      "O7",
    }.contains(removeIn)) {
      throw FormatException("Unknown removal slice: $removeIn");
    }
    return LegacyDependency(
      id: field("id"),
      violation: BoundaryViolation(rule, field("source"), field("target")),
      owner: field("owner"),
      reason: field("reason"),
      coverage: coverage.cast<String>(),
      removeIn: removeIn,
    );
  }
}

List<String> compareLegacyDependencies({
  required List<BoundaryViolation> violations,
  required List<LegacyDependency> inventory,
  required List<String> activeIds,
}) {
  final errors = <String>[];
  final byId = <String, LegacyDependency>{};
  final baselineKeys = <String>{};
  for (final entry in inventory) {
    if (byId.containsKey(entry.id) || !baselineKeys.add(entry.violation.key)) {
      throw FormatException("Duplicate frozen exception: ${entry.id}");
    }
    byId[entry.id] = entry;
  }
  final active = <String, String>{};
  for (final id in activeIds) {
    final entry = byId[id];
    if (entry == null) {
      errors.add(
        "Unknown exception $id; the frozen inventory cannot admit a new violation",
      );
    } else if (active.containsKey(entry.violation.key)) {
      errors.add("Duplicate active exception $id");
    } else {
      active[entry.violation.key] = id;
    }
  }
  for (final violation in violations) {
    if (active.remove(violation.key) == null) {
      final through = violation.via.isEmpty
          ? ""
          : " via ${violation.via.join(' -> ')}";
      errors.add(
        "${violation.source}:${violation.line} [${violation.rule}] -> ${violation.target}$through",
      );
    }
  }
  for (final id in active.values) {
    errors.add("Stale exception $id; remove it with its migrated boundary");
  }
  return errors;
}
