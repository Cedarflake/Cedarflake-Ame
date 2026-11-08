import "dart_graph.dart";

enum ModuleLayer {
  domain,
  port,
  application,
  adapter,
  presentation,
  composition,
  platform,
  bridge,
  unknown,
}

class ModuleOwner {
  const ModuleOwner(this.layer, {this.feature, this.isSharedView = false});

  final ModuleLayer layer;
  final String? feature;
  final bool isSharedView;
}

ModuleOwner moduleOwner(String path) {
  if (path == "lib/main.dart" || path.startsWith("lib/app/bootstrap/")) {
    return const ModuleOwner(ModuleLayer.composition);
  }
  if (path == "lib/app/ame_app.dart") {
    return const ModuleOwner(ModuleLayer.presentation);
  }
  if (path.startsWith("lib/app/presentation/")) {
    return const ModuleOwner(ModuleLayer.presentation, isSharedView: true);
  }
  if (path.startsWith("lib/app/notifications/")) {
    return const ModuleOwner(ModuleLayer.application);
  }
  if (path.startsWith("lib/app/window/")) {
    return const ModuleOwner(ModuleLayer.platform);
  }
  if (path.startsWith("lib/src/rust/")) {
    return const ModuleOwner(ModuleLayer.bridge);
  }
  final parts = path.split("/");
  if (parts.length >= 5 && parts[0] == "lib" && parts[1] == "features") {
    final layer = switch (parts[3]) {
      "domain" => ModuleLayer.domain,
      "application" when parts[4] == "ports" => ModuleLayer.port,
      "application" => ModuleLayer.application,
      "adapters" => ModuleLayer.adapter,
      "presentation" => ModuleLayer.presentation,
      _ => ModuleLayer.unknown,
    };
    return ModuleOwner(layer, feature: parts[2]);
  }
  return const ModuleOwner(ModuleLayer.unknown);
}

class BoundaryViolation {
  const BoundaryViolation(
    this.rule,
    this.source,
    this.target, {
    this.line = 1,
    this.via = const [],
  });

  final String rule;
  final String source;
  final String target;
  final int line;
  final List<String> via;

  String get key => "$rule|$source|$target";

  Map<String, Object> toJson() => {
    "rule": rule,
    "source": source,
    "target": target,
    "line": line,
    if (via.isNotEmpty) "via": via,
  };
}

const _valueLibraries = {
  "dart:async",
  "dart:collection",
  "dart:convert",
  "dart:core",
  "dart:math",
  "dart:typed_data",
};
const _stateLibraries = {
  "package:flutter/foundation.dart",
  "package:flutter_riverpod/flutter_riverpod.dart",
  "package:flutter_riverpod/legacy.dart",
};
const _designPackages = {"material_ui", "cupertino_ui"};
const _presentationPackages = {
  ..._designPackages,
  "flutter",
  "flutter_localizations",
  "flutter_riverpod",
  "material_symbols_icons",
};

// ADR 0012 and ADR 0037 retain these Ame-owned desktop contracts at their platform boundary.
const _windowContracts = {
  "lib/app/window/ame_runtime_platform.dart": {ModuleLayer.presentation},
  "lib/app/window/ame_window_chrome.dart": {ModuleLayer.presentation},
  "lib/app/window/ame_window_fullscreen.dart": {ModuleLayer.presentation},
  "lib/app/window/ame_window_placement.dart": {
    ModuleLayer.adapter,
    ModuleLayer.application,
  },
};

bool _allowsExternal(ModuleLayer layer, String target) {
  if (layer == ModuleLayer.composition || layer == ModuleLayer.platform) {
    return true;
  }
  if (layer == ModuleLayer.domain || layer == ModuleLayer.port) {
    return _valueLibraries.contains(target);
  }
  if (layer == ModuleLayer.application) {
    return _valueLibraries.contains(target) || _stateLibraries.contains(target);
  }
  if (layer == ModuleLayer.adapter) {
    final uri = Uri.parse(target);
    if (uri.scheme == "package" &&
        uri.pathSegments.isNotEmpty &&
        _designPackages.contains(uri.pathSegments.first)) {
      return false;
    }
    return !{
      "package:flutter/material.dart",
      "package:flutter/widgets.dart",
      "package:flutter/cupertino.dart",
      "package:flutter/rendering.dart",
    }.contains(target);
  }
  if (layer == ModuleLayer.presentation) {
    if (_valueLibraries.contains(target) || target == "dart:ui") return true;
    final uri = Uri.parse(target);
    return uri.scheme == "package" &&
        _presentationPackages.contains(uri.pathSegments.first);
  }
  return false;
}

bool _allowsLayer(ModuleLayer from, ModuleLayer to) => switch (from) {
  ModuleLayer.domain => to == ModuleLayer.domain,
  ModuleLayer.port => {ModuleLayer.domain, ModuleLayer.port}.contains(to),
  ModuleLayer.application => {
    ModuleLayer.domain,
    ModuleLayer.port,
    ModuleLayer.application,
  }.contains(to),
  ModuleLayer.adapter => {
    ModuleLayer.domain,
    ModuleLayer.port,
    ModuleLayer.adapter,
    ModuleLayer.bridge,
  }.contains(to),
  ModuleLayer.presentation => {
    ModuleLayer.domain,
    ModuleLayer.port,
    ModuleLayer.application,
    ModuleLayer.presentation,
  }.contains(to),
  ModuleLayer.platform => {
    ModuleLayer.domain,
    ModuleLayer.port,
    ModuleLayer.application,
    ModuleLayer.presentation,
    ModuleLayer.platform,
  }.contains(to),
  ModuleLayer.composition => to != ModuleLayer.unknown,
  ModuleLayer.bridge || ModuleLayer.unknown => false,
};

List<BoundaryViolation> checkDartBoundaries(
  Map<String, DartModule> modules, {
  Set<String> publicFeatureEdges = const {},
}) {
  final violations = <String, BoundaryViolation>{};
  void add(String rule, DartDependency edge) {
    final violation = BoundaryViolation(
      rule,
      edge.source,
      edge.target,
      line: edge.line,
      via: edge.via,
    );
    violations.putIfAbsent(violation.key, () => violation);
  }

  for (final module in modules.values) {
    if (module.isGeneratedBridge) continue;
    final owner = moduleOwner(module.path);
    if (owner.layer == ModuleLayer.unknown ||
        owner.layer == ModuleLayer.bridge) {
      final violation = BoundaryViolation(
        "unclassified",
        module.path,
        module.path,
      );
      violations[violation.key] = violation;
    }
    for (final edge in exposedDependencies(module, modules)) {
      if (Uri.parse(edge.target).hasScheme) {
        if (!_allowsExternal(owner.layer, edge.target)) {
          add("external-layer", edge);
        }
        continue;
      }
      if (!modules.containsKey(edge.target)) {
        add("unresolved", edge);
        continue;
      }
      final target = moduleOwner(edge.target);
      final platformContract =
          _windowContracts[edge.target]?.contains(owner.layer) ?? false;
      if (!_allowsLayer(owner.layer, target.layer) && !platformContract) {
        add("layer", edge);
      }
      if (owner.isSharedView && target.feature != null) {
        add("shared-feature", edge);
      }
      if (owner.feature != null &&
          target.feature != null &&
          owner.feature != target.feature) {
        final isContract = {
          ModuleLayer.domain,
          ModuleLayer.port,
          ModuleLayer.application,
        }.contains(target.layer);
        if (!isContract ||
            !publicFeatureEdges.contains("${edge.source}|${edge.target}")) {
          add("cross-feature", edge);
        }
      }
      final isPage = edge.target.contains("/presentation/pages/");
      final fromLeaf =
          edge.source.contains("/presentation/widgets/") ||
          edge.source.contains("/presentation/coordination/");
      if (isPage && fromLeaf) add("page-dependency", edge);
    }
  }
  for (final component in dependencyCycles(modules)) {
    final violation = BoundaryViolation(
      "cycle",
      component.first,
      component.join(","),
    );
    violations[violation.key] = violation;
  }
  return violations.values.toList()..sort((a, b) => a.key.compareTo(b.key));
}
