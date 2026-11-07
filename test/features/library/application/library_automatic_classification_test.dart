import "package:cedarflake_ame/features/library/application/library_automatic_classification.dart";
import "package:cedarflake_ame/features/library/application/ports/library_classification.dart";
import "package:cedarflake_ame/features/library/application/ports/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("missing models never activate or open analysis", () async {
    final fixture = _Fixture()..configuration.installed = false;
    expect(await fixture.owner.open(), isNull);
    expect(fixture.events, ["load"]);
  });

  test("automatic setup opens without replacing its run", () async {
    final fixture = _Fixture();
    await fixture.owner.open();
    expect(fixture.events, ["load", "open"]);
  });

  test("legacy pause resumes the observed run before opening", () async {
    final fixture = _Fixture();
    fixture.configuration.mode = LibraryClassificationMode.paused;
    await fixture.owner.open();
    expect(fixture.events, ["load", "resume", "open"]);
    expect(fixture.configuration.observedDispatch, BigInt.from(7));
    expect(fixture.configuration.activationId, isNull);
    await fixture.owner.open();
    expect(fixture.events, ["load", "resume", "open", "load", "open"]);
  });

  for (final mode in [
    LibraryClassificationMode.disabled,
    LibraryClassificationMode.stopped,
  ]) {
    test(
      "legacy $mode activates retained categories once before opening",
      () async {
        final fixture = _Fixture();
        fixture.configuration.mode = mode;
        await fixture.owner.open();
        expect(fixture.events, ["load", "enable", "open"]);
        expect(
          fixture.configuration.savedCategories,
          same(_Configuration.categories),
        );
        expect(fixture.configuration.observed?.revision, BigInt.from(3));
        expect(
          fixture.configuration.activationId,
          matches(RegExp(r"^[0-9a-f]{32}$")),
        );
        await fixture.owner.open();
        expect(fixture.events, ["load", "enable", "open", "load", "open"]);
      },
    );
  }

  test(
    "an installed setup without definitions uses admitted initial categories",
    () async {
      final fixture = _Fixture();
      fixture.configuration.mode = LibraryClassificationMode.disabled;
      fixture.configuration.hasCategories = false;
      await fixture.owner.open();
      expect(fixture.events, ["load", "initial categories", "enable", "open"]);
      expect(
        fixture.configuration.savedCategories,
        same(_Configuration.categories),
      );
    },
  );

  for (final mode in [
    LibraryClassificationMode.paused,
    LibraryClassificationMode.stopped,
  ]) {
    test("failed legacy $mode transition cannot open a worker", () async {
      final fixture = _Fixture();
      fixture.configuration.mode = mode;
      fixture.configuration.fail = true;
      await expectLater(fixture.owner.open(), throwsStateError);
      expect(fixture.events, isNot(contains("open")));
      expect(fixture.configuration.mode, mode);
    });
  }
}

class _Fixture {
  final events = <String>[];
  late final configuration = _Configuration(events);
  late final owner = LibraryAutomaticClassification(
    _Runner(events),
    configuration,
  );
}

class _Runner implements LibraryClassification {
  _Runner(this.events);
  final List<String> events;

  @override
  Future<LibraryClassificationSession?> open() async {
    events.add("open");
    return null;
  }
}

class _Configuration implements LibraryClassificationConfigurationService {
  @override
  Future<LibraryClassificationAcceleration> loadAcceleration() async =>
      const LibraryClassificationAcceleration(gpuEnabled: true);

  @override
  Future<void> saveAcceleration({required bool gpuEnabled}) async {
    throw UnsupportedError("Acceleration is outside this fixture");
  }

  _Configuration(this.events);
  final List<String> events;
  bool installed = true;
  bool hasCategories = true;
  bool fail = false;
  LibraryClassificationMode mode = LibraryClassificationMode.automatic;
  LibraryClassificationSettings? observed;
  BigInt? observedDispatch;
  String? activationId;
  List<LibraryClassificationCategory>? savedCategories;
  static const categories = [
    LibraryClassificationCategory(
      id: "retained",
      label: "Existing category",
      descriptions: ["retained description"],
    ),
  ];

  LibraryClassificationSettings get settings => LibraryClassificationSettings(
    revision: BigInt.from(3),
    dispatchRevision: BigInt.from(7),
    mode: mode,
    categories: hasCategories ? categories : [],
    runId: "legacy-run",
    installationSelected: installed,
  );

  @override
  Future<LibraryClassificationSettings> load() async {
    events.add("load");
    return settings;
  }

  @override
  Future<LibraryClassificationSettings> loadNavigation() => load();

  @override
  Future<List<LibraryClassificationCategory>> initialCategories() async {
    events.add("initial categories");
    return categories;
  }

  @override
  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  }) async {
    expect(action, LibraryClassificationControl.resume);
    events.add("resume");
    if (fail) throw StateError("configuration changed");
    observedDispatch = expectedRevision;
    mode = LibraryClassificationMode.automatic;
    return settings;
  }

  @override
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationSettings observed,
    required List<LibraryClassificationCategory> categories,
    required LibraryClassificationConfigurationAction action,
  }) async {
    expect(action, LibraryClassificationConfigurationAction.enable);
    events.add("enable");
    if (fail) throw StateError("configuration changed");
    this.observed = observed;
    activationId = operationId;
    savedCategories = categories;
    mode = LibraryClassificationMode.automatic;
    return settings;
  }
}
