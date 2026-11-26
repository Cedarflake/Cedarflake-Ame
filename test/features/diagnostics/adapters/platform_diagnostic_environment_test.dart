import "dart:async";

import "package:cedarflake_ame/features/diagnostics/adapters/platform/platform_diagnostic_environment.dart";
import "package:cedarflake_ame/features/diagnostics/domain/diagnostic_report.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  final binding = TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel("cedarflake_ame/diagnostics");
  const environment = PlatformDiagnosticEnvironment();

  tearDown(() {
    binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, null);
  });

  testWidgets("keeps GPU drivers paired and samples current window scaling", (
    tester,
  ) async {
    var cpuModel = "  Intel(R) Core(TM) CPU @ 2.40GHz  ";
    binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (_) async {
      return {
        "cpuModel": cpuModel,
        "physicalMemoryBytes": 34359738368,
        "graphicsAdapterState": "available",
        "graphicsAdapters": [
          {"model": "Integrated Graphics", "driverVersion": "31.0.101.5534"},
          {"model": "Discrete Graphics", "driverVersion": "32.0.15.6094"},
        ],
        "machineName": "private-device-name",
        "userPath": r"C:\Users\Alice",
      };
    });
    tester.view.devicePixelRatio = 1.25;
    addTearDown(tester.view.resetDevicePixelRatio);
    final first = await environment.collect();
    final facts = {for (final fact in first.facts) fact.field: fact.value};
    expect(facts[DiagnosticField.cpuModel], "Intel(R) Core(TM) CPU @ 2.40GHz");
    expect(facts[DiagnosticField.physicalMemoryBytes], "34359738368");
    expect(facts[DiagnosticField.displayScalePercent], "125.00");
    expect(
      first.graphicsAdapters.map(
        (adapter) => (adapter.model, adapter.driverVersion),
      ),
      [
        ("Integrated Graphics", "31.0.101.5534"),
        ("Discrete Graphics", "32.0.15.6094"),
      ],
    );
    expect(facts.values.join(), isNot(contains("private-device-name")));
    expect(facts.values.join(), isNot(contains("Alice")));
    cpuModel = "AMD Ryzen 7 7840HS w/ Radeon 780M Graphics";
    tester.view.devicePixelRatio = 1.5;
    final second = await environment.collect();
    expect(
      second.facts
          .singleWhere((fact) => fact.field == DiagnosticField.cpuModel)
          .value,
      "AMD Ryzen 7 7840HS w/ Radeon 780M Graphics",
    );
    expect(
      second.facts
          .singleWhere(
            (fact) => fact.field == DiagnosticField.displayScalePercent,
          )
          .value,
      "150.00",
    );
  });

  test("unavailable hardware preserves OS and framework facts", () async {
    for (final failure in [
      MissingPluginException(),
      PlatformException(code: "diagnostic_environment_unavailable"),
    ]) {
      binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
        _,
      ) async {
        throw failure;
      });
      final snapshot = await environment.collect();
      final facts = {for (final fact in snapshot.facts) fact.field: fact.value};
      expect(facts[DiagnosticField.cpuModel], "unavailable");
      expect(facts[DiagnosticField.physicalMemoryBytes], "unavailable");
      expect(facts[DiagnosticField.graphicsAdapterState], "unavailable");
      expect(facts[DiagnosticField.operatingSystem], isNot("unavailable"));
      expect(facts, contains(DiagnosticField.flutterEngineRevision));
      expect(snapshot.graphicsAdapters, isEmpty);
    }
  });

  testWidgets("slow hardware cannot hold export collection indefinitely", (
    tester,
  ) async {
    final pending = Completer<Map<String, Object?>>();
    binding.defaultBinaryMessenger.setMockMethodCallHandler(
      channel,
      (_) => pending.future,
    );
    var completed = false;
    final reading = environment.collect().then((value) {
      completed = true;
      return value;
    });
    await tester.pump();
    await tester.pump(const Duration(seconds: 2));
    expect(completed, isTrue);
    final snapshot = await reading;
    expect(
      snapshot.facts
          .singleWhere((fact) => fact.field == DiagnosticField.cpuModel)
          .value,
      "unavailable",
    );
    pending.complete({"cpuModel": "Late processor"});
    await tester.pump();
    expect(
      snapshot.facts
          .singleWhere((fact) => fact.field == DiagnosticField.cpuModel)
          .value,
      "unavailable",
    );
  });

  test(
    "rejects private or malformed values without losing other hardware",
    () async {
      binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
        _,
      ) async {
        return {
          "cpuModel": r"C:\Users\Alice\processor.txt",
          "physicalMemoryBytes": -1,
          "graphicsAdapterState": "partial",
          "graphicsAdapters": [
            {"model": "Microsoft 基本显示适配器", "driverVersion": "invalid\nprivate"},
            {"model": "x" * 161, "driverVersion": "31.0.0.1"},
          ],
        };
      });
      final snapshot = await environment.collect();
      final facts = {for (final fact in snapshot.facts) fact.field: fact.value};
      expect(facts[DiagnosticField.cpuModel], "unavailable");
      expect(facts[DiagnosticField.physicalMemoryBytes], "unavailable");
      expect(snapshot.graphicsAdapters.first.model, "Microsoft 基本显示适配器");
      expect(snapshot.graphicsAdapters.first.driverVersion, "unavailable");
      expect(snapshot.graphicsAdapters.last.model, "unavailable");
      expect(snapshot.graphicsAdapters.last.driverVersion, "31.0.0.1");
    },
  );
}
