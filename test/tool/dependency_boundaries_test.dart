import "package:flutter_test/flutter_test.dart";

import "../../tool/dependency-boundaries/boundary_policy.dart";
import "../../tool/dependency-boundaries/boundary_rules.dart";
import "../../tool/dependency-boundaries/dart_graph.dart";
import "../../tool/dependency-boundaries/legacy_inventory.dart";

const _domain = "lib/features/library/domain/value.dart";
const _port = "lib/features/library/application/ports/catalog.dart";
const _application = "lib/features/library/application/query.dart";
const _adapter = "lib/features/library/adapters/rust/catalog.dart";
const _page = "lib/features/library/presentation/pages/library.dart";
const _widget = "lib/features/library/presentation/widgets/tile.dart";

Map<String, DartModule> _graph(
  Map<String, String> sources, {
  Set<String> generated = const {},
}) => {
  for (final entry in sources.entries)
    entry.key: parseDartModule(
      entry.key,
      entry.value,
      isGeneratedBridge: generated.contains(entry.key),
    ),
};

Map<String, Object?> _exceptionRecord() => {
  "id": "D001",
  "rule": "layer",
  "source": _application,
  "target": _adapter,
  "owner": "catalog read contract",
  "reason": "Native construction remains with the application declaration",
  "coverage": [
    "test/features/library/application/library_controller_test.dart",
  ],
  "removeIn": "O3",
};

void main() {
  test(
    "frozen source and inventory fingerprints cannot be replaced by a fresh baseline",
    () {
      final record = _exceptionRecord();
      final document = <String, Object?>{
        "sourceRevision": legacyDartRevision,
        "owners": {record["owner"] as String: record},
        "exceptions": [record],
      };
      expect(
        parseLegacyInventory(document, blobId: legacyDartInventoryBlob),
        hasLength(1),
      );
      expect(
        () => parseLegacyInventory(document, blobId: "current-violations"),
        throwsFormatException,
      );
      expect(
        () => parseLegacyInventory({
          ...document,
          "sourceRevision": "new-head",
        }, blobId: legacyDartInventoryBlob),
        throwsFormatException,
      );
    },
  );

  test(
    "parses multiline directives, raw URIs and every conditional branch",
    () {
      final module = parseDartModule(_application, """
// import '../adapters/ignored.dart';
import
  'package:cedarflake_ame/features/library/domain/value.dart';
export r'../domain/value.dart';
import '../domain/value.dart'
  if (dart.library.io) '../adapters/rust/catalog.dart';
const text = "export 'ignored.dart';";
""");
      expect(module.dependencies.map((edge) => edge.target), [
        _domain,
        _domain,
        _domain,
        _adapter,
      ]);
      expect(module.dependencies[1].kind, DependencyKind.export);
      expect(module.dependencies[0].line, 3);
      expect(module.declarations, ["text"]);
    },
  );

  test("resolves package and relative spellings to the same dependency", () {
    expect(
      resolveDartDependency(_application, "../domain/value.dart"),
      _domain,
    );
    expect(
      resolveDartDependency(_application, "../domain/../domain/value.dart"),
      _domain,
    );
    expect(
      resolveDartDependency(
        _application,
        "package:cedarflake_ame/features/library/domain/value.dart",
      ),
      _domain,
    );
    expect(
      () => resolveDartDependency(_application, "file:///outside.dart"),
      throwsFormatException,
    );
    expect(
      () => resolveDartDependency(_application, "../domain/value.dart#hidden"),
      throwsFormatException,
    );
  });

  test("admits feature contracts, use cases, adapters and presentation in order", () {
    final modules = _graph({
      _domain: "import 'dart:typed_data'; class Value {}",
      _port:
          "import '../../domain/value.dart'; abstract interface class Catalog {}",
      _application:
          "import 'ports/catalog.dart'; import 'package:flutter/foundation.dart';"
          "import 'package:flutter_riverpod/legacy.dart';",
      _adapter:
          "import '../../application/ports/catalog.dart'; import 'dart:io';",
      _page:
          "import '../../application/query.dart'; import '../widgets/tile.dart';",
      _widget:
          "import '../../domain/value.dart'; import 'package:flutter/material.dart';",
      "lib/app/bootstrap/library_bindings.dart":
          "import '../../features/library/adapters/rust/catalog.dart';",
    });
    expect(checkDartBoundaries(modules), isEmpty);
  });

  test(
    "rejects domain UI, Provider ports, application widgets and presentation I/O",
    () {
      final modules = _graph({
        _domain: "import 'package:flutter/material.dart';",
        _port: "import 'package:flutter_riverpod/flutter_riverpod.dart';",
        _application: "import 'package:flutter/widgets.dart';",
        _widget: "import 'dart:io';",
      });
      expect(
        checkDartBoundaries(modules).map((entry) => entry.rule),
        everyElement("external-layer"),
      );
      expect(checkDartBoundaries(modules), hasLength(4));
    },
  );

  test("an export chain cannot conceal a native implementation", () {
    const barrel = "lib/features/library/domain/barrel.dart";
    final modules = _graph({
      _application: "import '../domain/barrel.dart';",
      barrel:
          "export 'value.dart' if (dart.library.io) '../adapters/rust/catalog.dart';",
      _domain: "export '../adapters/rust/catalog.dart';",
      _adapter: "",
    });
    final violation = checkDartBoundaries(modules).singleWhere(
      (entry) => entry.source == _application && entry.target == _adapter,
    );
    expect(violation.rule, "layer");
    expect(violation.via.first, barrel);
  });

  test("rejects widgets and coordinators that import a page", () {
    final modules = _graph({
      _page: "",
      _widget: "import '../pages/library.dart';",
      "lib/features/library/presentation/coordination/selection.dart":
          "export '../pages/library.dart';",
    });
    expect(checkDartBoundaries(modules).map((entry) => entry.rule), [
      "page-dependency",
      "page-dependency",
    ]);
  });

  test(
    "finds complete conditional cycles and self-imports without flagging a DAG",
    () {
      final modules = _graph({
        _domain: "export 'barrel.dart';",
        "lib/features/library/domain/barrel.dart":
            "export 'tail.dart' if (dart.library.io) 'value.dart';",
        "lib/features/library/domain/tail.dart": "",
        _port: "import 'catalog.dart';",
        _application: "import '../domain/value.dart';",
      });
      final cycles = dependencyCycles(modules);
      expect(cycles, hasLength(2));
      expect(cycles, contains(equals([_port])));
      expect(
        cycles,
        contains(equals(["lib/features/library/domain/barrel.dart", _domain])),
      );
    },
  );

  test(
    "cross-feature entrypoints require an exact consumer and named contract",
    () {
      const consumer = "lib/features/settings/application/preferences.dart";
      final modules = _graph({
        consumer: "import '../../library/application/query.dart';",
        _application: "",
      });
      expect(checkDartBoundaries(modules).single.rule, "cross-feature");
      final policy = DartBoundaryPolicy.parse({
        "activeExceptions": <String>[],
        "publicFeatureEdges": [
          {
            "source": consumer,
            "target": _application,
            "contract": "LibraryQuery",
          },
        ],
      }, modules);
      expect(
        checkDartBoundaries(
          modules,
          publicFeatureEdges: policy.publicFeatureEdges,
        ),
        isEmpty,
      );
      expect(
        () => DartBoundaryPolicy.parse({
          "activeExceptions": <String>[],
          "publicFeatureEdges": [
            {"source": consumer, "target": _adapter, "contract": "RustCatalog"},
          ],
        }, modules),
        throwsFormatException,
      );
    },
  );

  test("shared visual primitives cannot acquire feature policy", () {
    final modules = _graph({
      "lib/app/presentation/card.dart":
          "import '../../features/library/domain/value.dart';",
      _domain: "",
    });
    expect(checkDartBoundaries(modules).single.rule, "shared-feature");
  });

  test(
    "retains declared desktop chrome but rejects concrete window commands",
    () {
      final modules = _graph({
        _page: "import '../../../../app/window/ame_window_chrome.dart';",
        _widget: "import '../../../../app/window/window_manager_actions.dart';",
        "lib/app/window/ame_window_chrome.dart": "",
        "lib/app/window/window_manager_actions.dart": "",
      });
      expect(checkDartBoundaries(modules).single.source, _widget);
    },
  );

  test("runtime host facts expose only their named platform contract", () {
    final modules = _graph({
      _page: "import '../../../../app/window/ame_runtime_platform.dart';",
      _widget: "import '../../../../app/window/ame_runtime_platform_io.dart';",
      "lib/app/window/ame_runtime_platform.dart": """
import 'ame_runtime_platform_stub.dart'
  if (dart.library.io) 'ame_runtime_platform_io.dart' as host;
bool get isAmeWindowsHost => host.isAmeWindowsHost;
""",
      "lib/app/window/ame_runtime_platform_stub.dart":
          "bool get isAmeWindowsHost => false;",
      "lib/app/window/ame_runtime_platform_io.dart":
          "import 'dart:io'; bool get isAmeWindowsHost => Platform.isWindows;",
    });
    final violation = checkDartBoundaries(modules).single;
    expect(violation.source, _widget);
    expect(violation.rule, "layer");
    expect(violation.target, "lib/app/window/ame_runtime_platform_io.dart");
  });

  test(
    "only generated bridge files are excluded; new handwritten owners fail closed",
    () {
      const marker = "// This file is automatically generated\n";
      final modules = _graph(
        {
          "lib/src/rust/generated.dart": "${marker}import 'dart:ffi';",
          "lib/src/rust/handwritten.dart": "${marker}class Unexpected {}",
          _domain: "${marker}import 'dart:io';",
          "lib/features/library/helpers.dart": "",
          _application: "import 'missing.dart';",
        },
        generated: {"lib/src/rust/generated.dart"},
      );
      expect(
        checkDartBoundaries(modules).map((entry) => entry.rule).toList()
          ..sort(),
        ["external-layer", "unclassified", "unclassified", "unresolved"],
      );
    },
  );

  test(
    "rejects malformed source rather than silently skipping its directives",
    () {
      expect(() => parseDartModule(_domain, "import ;"), throwsArgumentError);
    },
  );

  test(
    "parts cannot move domain implementation behind an adapter directory",
    () {
      final modules = _graph({
        _domain: "part '../adapters/rust/catalog.dart';",
        _adapter: "part of '../../domain/value.dart';",
      });
      expect(checkDartBoundaries(modules).single.rule, "layer");
      expect(dependencyCycles(modules), isEmpty);
    },
  );

  test(
    "generated admission also requires the expected path and generation marker",
    () {
      expect(
        () => parseDartModule(
          "lib/src/rust/generated.dart",
          "class Handwritten {}",
          isGeneratedBridge: true,
        ),
        throwsFormatException,
      );
      expect(
        () => parseDartModule(
          _domain,
          "// This file is automatically generated\n",
          isGeneratedBridge: true,
        ),
        throwsFormatException,
      );
    },
  );

  test(
    "frozen exceptions admit only their exact edge and removal must be explicit",
    () {
      final entry = LegacyDependency.fromJson(_exceptionRecord());
      final old = entry.violation;
      const fresh = BoundaryViolation("layer", _port, _adapter);
      expect(
        compareLegacyDependencies(
          violations: [old],
          inventory: [entry],
          activeIds: [entry.id],
        ),
        isEmpty,
      );
      expect(
        compareLegacyDependencies(
          violations: [old, fresh],
          inventory: [entry],
          activeIds: [entry.id],
        ),
        hasLength(1),
      );
      expect(
        compareLegacyDependencies(
          violations: [],
          inventory: [entry],
          activeIds: [entry.id],
        ).single,
        contains("Stale exception"),
      );
      expect(
        compareLegacyDependencies(
          violations: [old],
          inventory: [entry],
          activeIds: [],
        ),
        hasLength(1),
      );
      expect(
        compareLegacyDependencies(
          violations: [],
          inventory: [entry],
          activeIds: ["D999"],
        ).single,
        contains("Unknown exception"),
      );
      expect(
        compareLegacyDependencies(
          violations: [old],
          inventory: [entry],
          activeIds: [entry.id, entry.id],
        ).single,
        contains("Duplicate active"),
      );
    },
  );

  test(
    "exception metadata cannot use directory exemptions or hide broken resolution",
    () {
      for (final patch in [
        {"source": "lib/features/library/*"},
        {"coverage": <String>[]},
        {"reason": ""},
        {"rule": "unresolved"},
        {"removeIn": "someday"},
      ]) {
        expect(
          () => LegacyDependency.fromJson({..._exceptionRecord(), ...patch}),
          throwsFormatException,
        );
      }
    },
  );
}
