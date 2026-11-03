import "boundary_rules.dart";
import "dart_graph.dart";

class DartBoundaryPolicy {
  const DartBoundaryPolicy({
    required this.activeExceptions,
    required this.publicFeatureEdges,
  });

  final List<String> activeExceptions;
  final Set<String> publicFeatureEdges;

  factory DartBoundaryPolicy.parse(
    Map<String, Object?> document,
    Map<String, DartModule> modules,
  ) {
    final active = document["activeExceptions"];
    final entries = document["publicFeatureEdges"];
    if (active is! List<Object?> ||
        active.any((entry) => entry is! String) ||
        entries is! List<Object?>) {
      throw const FormatException(
        "Boundary policy requires activeExceptions and publicFeatureEdges lists",
      );
    }
    final edges = <String>{};
    for (final entry in entries) {
      if (entry case {
        "source": final String source,
        "target": final String target,
        "contract": final String contract,
      }) {
        final origin = modules[source];
        final owner = moduleOwner(source);
        final destination = moduleOwner(target);
        final isContract = {
          ModuleLayer.domain,
          ModuleLayer.port,
          ModuleLayer.application,
        }.contains(destination.layer);
        final present =
            origin != null &&
            exposedDependencies(
              origin,
              modules,
            ).any((edge) => edge.target == target);
        if (!present ||
            !modules.containsKey(target) ||
            !isContract ||
            owner.feature == null ||
            destination.feature == null ||
            owner.feature == destination.feature ||
            contract.trim().isEmpty ||
            source.contains("*") ||
            target.contains("*")) {
          throw FormatException(
            "Invalid or stale public contract: $source -> $target",
          );
        }
        if (!edges.add("$source|$target")) {
          throw FormatException(
            "Duplicate public contract: $source -> $target",
          );
        }
      } else {
        throw const FormatException(
          "Public feature edges require exact source, target and named contract",
        );
      }
    }
    return DartBoundaryPolicy(
      activeExceptions: active.cast<String>(),
      publicFeatureEdges: edges,
    );
  }
}
