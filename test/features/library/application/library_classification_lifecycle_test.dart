import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("attachment failure remains visible after a successful open", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.session.failForeground = true;
    fixture.owner.startInBackground();
    fixture.service.opened.complete(fixture.session);
    await tester.pump();
    expect(fixture.owner.error, isStateError);
    await fixture.owner.close();
    fixture.dispose();
  });

  testWidgets(
    "failed startup rejects enablement while settings remain readable",
    (tester) async {
      final fixture = _Fixture();
      final backend = _SetupConfiguration();
      final configuration = LibraryClassificationConfiguration(
        backend,
        fixture.owner,
      );
      fixture.owner.startInBackground();
      fixture.service.opened.complete(null);
      await tester.pump();
      final change = configuration.enable(
        operationId: "startup-failure",
        observed: backend.settings,
      );
      final checked = expectLater(change, throwsStateError);
      await tester.pump();
      fixture.service.reopened.completeError(StateError("startup failed"));
      await tester.pump();
      await checked;
      expect(fixture.owner.error, isStateError);
      expect(await configuration.load(), same(backend.settings));
      await expectLater(fixture.owner.close(), throwsStateError);
      fixture.dispose();
    },
  );

  testWidgets("successful reconfiguration clears only the runtime failure", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.owner.startInBackground();
    fixture.service.opened.completeError(StateError("startup failed"));
    await tester.pump();
    expect(fixture.owner.error, isStateError);
    final change = fixture.owner.changeConfiguration(() async {});
    fixture.service.reopened.complete(fixture.session);
    await tester.pump();
    await change;
    expect(fixture.owner.error, isNull);
    final environmentFailure = StateError("power observation unavailable");
    fixture.owner.reportEnvironmentError(
      environmentFailure,
      StackTrace.current,
    );
    await fixture.owner.changeConfiguration(() async {});
    expect(fixture.owner.error, same(environmentFailure));
    await fixture.owner.close();
    fixture.dispose();
  });

  testWidgets(
    "closing during startup retires the late handle without running",
    (tester) async {
      final fixture = _Fixture();
      fixture.owner.startInBackground();
      final closed = fixture.owner.close();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      await closed;
      expect(fixture.session.runs, 0);
      expect(fixture.session.closed, isTrue);
      fixture.dispose();
    },
  );

  testWidgets("unconfigured startup remains inactive and closes normally", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.owner.startInBackground();
    fixture.owner.startInBackground();
    fixture.service.opened.complete(null);
    await tester.pump();
    await fixture.owner.close();
    expect(fixture.service.opens, 1);
    expect(fixture.session.runs, 0);
    fixture.dispose();
  });

  testWidgets(
    "late startup receives current reads, import, and suspend before running",
    (tester) async {
      final fixture = _Fixture();
      fixture.owner.startInBackground();
      fixture.update(1, roots: {"root": BigInt.one});
      fixture.update(2, busy: true, roots: {"root": BigInt.two});
      final read = fixture.reads.acquire();
      fixture.owner.setSuspended(true);
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      expect(fixture.session.activityAtRun, (true, true, true, false));
      expect(fixture.session.recovered, [("root", BigInt.two)]);
      read.close();
      fixture.owner.setSuspended(false);
      fixture.update(3);
      await tester.pump();
      expect(fixture.session.activity, (false, false, false, false));
      await fixture.owner.close();
      fixture.dispose();
    },
  );

  testWidgets("close waits for the running task and in-flight recovery", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.session.autoFinish = false;
    fixture.session.recoveryGate = Completer<void>();
    fixture.update(1, roots: {"root": BigInt.one});
    fixture.owner.startInBackground();
    fixture.service.opened.complete(fixture.session);
    await tester.pump();
    var completed = false;
    final closed = fixture.owner.close().then((_) => completed = true);
    await tester.pump();
    expect(fixture.session.closeRequested, isTrue);
    expect(completed, isFalse);
    fixture.session.finished.complete();
    await tester.pump();
    expect(completed, isFalse);
    fixture.session.recoveryGate!.complete();
    await tester.pump();
    await closed;
    expect(fixture.session.closed, isTrue);
    fixture.dispose();
  });

  testWidgets(
    "catalog updates coalesce and returned roots recover without restarting work",
    (tester) async {
      final fixture = _Fixture();
      fixture.update(1, roots: {"root": BigInt.one});
      fixture.owner.startInBackground();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      final notifications = fixture.session.catalogNotifications;
      fixture.update(1, roots: {"root": BigInt.one});
      fixture.owner.browsingActivity();
      expect(fixture.session.catalogNotifications, notifications);
      fixture.update(2);
      fixture.update(3, roots: {"root": BigInt.one});
      await tester.pump();
      expect(fixture.session.recovered, [
        ("root", BigInt.one),
        ("root", BigInt.one),
      ]);
      expect(fixture.session.runs, 1);
      expect(fixture.session.closeRequested, isFalse);
      await fixture.owner.close();
      fixture.dispose();
    },
  );

  testWidgets("pressure releases resources and retains a bounded cooldown", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.update(1);
    fixture.owner.startInBackground();
    fixture.service.opened.complete(fixture.session);
    await tester.pump();
    fixture.owner.memoryPressure();
    expect(fixture.session.activity.$4, isTrue);
    await tester.pump(const Duration(seconds: 59));
    expect(fixture.session.activity.$4, isTrue);
    await tester.pump(const Duration(seconds: 1));
    expect(fixture.session.activity.$4, isFalse);
    expect(fixture.session.runs, 1);
    await fixture.owner.close();
    fixture.dispose();
  });

  testWidgets("enabling after inactive startup opens a new session", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.owner.startInBackground();
    fixture.service.opened.complete(null);
    await tester.pump();
    var saved = false;
    final change = fixture.owner.changeConfiguration(() async {
      saved = true;
    });
    await tester.pump();
    expect(saved, isTrue);
    expect(fixture.service.opens, 2);
    fixture.service.reopened.complete(fixture.session);
    await tester.pump();
    await change;
    expect(fixture.session.runs, 1);
    await fixture.owner.close();
    fixture.dispose();
  });

  testWidgets("configuration waits for execution and handle retirement", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.session.autoFinish = false;
    fixture.session.closeGate = Completer<void>();
    fixture.owner.startInBackground();
    fixture.service.opened.complete(fixture.session);
    await tester.pump();
    var saved = false;
    var retiredAtSave = false;
    final change = fixture.owner.changeConfiguration(() async {
      retiredAtSave = fixture.session.closed;
      saved = true;
    });
    await tester.pump();
    expect(fixture.session.closeRequested, isTrue);
    expect(saved, isFalse);
    fixture.session.finished.complete();
    await tester.pump();
    expect(saved, isFalse);
    fixture.session.closeGate!.complete();
    fixture.service.reopened.complete(null);
    await tester.pump();
    await change;
    expect(saved, isTrue);
    expect(retiredAtSave, isTrue);
    await fixture.owner.close();
    fixture.dispose();
  });

  testWidgets(
    "managed installation commits after retirement and preserves observed settings",
    (tester) async {
      final fixture = _Fixture();
      fixture.session.autoFinish = false;
      fixture.session.closeGate = Completer<void>();
      fixture.owner.startInBackground();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      final backend = _SetupConfiguration();
      final configuration = LibraryClassificationConfiguration(
        backend,
        fixture.owner,
      );
      final observed = backend.settings;
      var retiredAtCommit = false;
      final change = configuration.enableManagedInstallation(
        operationId: "managed-installation",
        observed: observed,
        commitInstallation: () async {
          retiredAtCommit = fixture.session.closed;
          backend.actions.add("install");
        },
      );
      await tester.pump();
      expect(fixture.session.closeRequested, isTrue);
      expect(backend.actions, isEmpty);
      fixture.session.finished.complete();
      await tester.pump();
      expect(backend.actions, isEmpty);
      fixture.session.closeGate!.complete();
      fixture.service.reopened.complete(null);
      await tester.pump();
      await change;
      expect(backend.actions, ["install", "enable"]);
      expect(retiredAtCommit, isTrue);
      expect(
        backend.savedAction,
        LibraryClassificationConfigurationAction.enable,
      );
      expect(identical(backend.savedObserved, observed), isTrue);
      expect(identical(backend.savedCategories, observed.categories), isTrue);
      expect(backend.initialLoads, 0);
      expect(fixture.service.opens, 2);
      await fixture.owner.close();
      fixture.dispose();
    },
  );

  testWidgets("failed managed publication leaves classification disabled", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.owner.startInBackground();
    fixture.service.opened.complete(null);
    await tester.pump();
    final backend = _SetupConfiguration(configured: false);
    final configuration = LibraryClassificationConfiguration(
      backend,
      fixture.owner,
    );
    final change = configuration.enableManagedInstallation(
      operationId: "failed-installation",
      observed: backend.settings,
      commitInstallation: () async => throw StateError("Installation changed"),
    );
    final check = expectLater(change, throwsStateError);
    fixture.service.reopened.complete(null);
    await tester.pump();
    await check;
    expect(backend.actions, isEmpty);
    expect(backend.initialLoads, 1);
    await fixture.owner.close();
    fixture.dispose();
  });

  testWidgets("late opening during reconfiguration never starts the old task", (
    tester,
  ) async {
    final fixture = _Fixture();
    fixture.owner.startInBackground();
    var saved = false;
    final change = fixture.owner.changeConfiguration(() async {
      saved = true;
    });
    await tester.pump();
    expect(saved, isFalse);
    fixture.service.opened.complete(fixture.session);
    fixture.service.reopened.complete(null);
    await tester.pump();
    await change;
    expect(fixture.session.runs, 0);
    expect(fixture.session.closed, isTrue);
    expect(saved, isTrue);
    await fixture.owner.close();
    fixture.dispose();
  });

  testWidgets("closing joins a configuration write and prevents restarting", (
    tester,
  ) async {
    final fixture = _Fixture();
    final saved = Completer<void>();
    final change = fixture.owner.changeConfiguration(() => saved.future);
    await tester.pump();
    var closed = false;
    final closing = fixture.owner.close().then((_) {
      closed = true;
    });
    await tester.pump();
    expect(closed, isFalse);
    saved.complete();
    await tester.pump();
    await change;
    await closing;
    expect(fixture.service.opens, 0);
    fixture.dispose();
  });

  testWidgets(
    "failed configuration reopens the saved setup and reports the error",
    (tester) async {
      final fixture = _Fixture();
      fixture.owner.startInBackground();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      final result = fixture.owner.changeConfiguration<void>(() async {
        throw StateError("save failed");
      });
      final observed = expectLater(result, throwsStateError);
      final replacement = _Session();
      fixture.service.reopened.complete(replacement);
      await tester.pump();
      await observed;
      expect(fixture.session.closed, isTrue);
      expect(replacement.runs, 1);
      await fixture.owner.close();
      fixture.dispose();
    },
  );

  testWidgets("a second configuration cannot overlap an admitted change", (
    tester,
  ) async {
    final fixture = _Fixture();
    final saved = Completer<void>();
    final first = fixture.owner.changeConfiguration(() => saved.future);
    var secondRan = false;
    await expectLater(
      fixture.owner.changeConfiguration(() async {
        secondRan = true;
      }),
      throwsStateError,
    );
    await tester.pump();
    final closed = fixture.owner.close();
    saved.complete();
    await tester.pump();
    await first;
    await closed;
    expect(secondRan, isFalse);
    fixture.dispose();
  });

  testWidgets(
    "maintenance joins recovery and retirement before commit and reopen",
    (tester) async {
      final fixture = _Fixture();
      final plan = _Maintenance();
      fixture.session.maintenance = plan;
      fixture.session.recoveryGate = Completer<void>();
      fixture.session.closeGate = Completer<void>();
      fixture.update(1, roots: {"root": BigInt.one});
      fixture.owner.startInBackground();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      fixture.session.finished.complete();
      await tester.pump();
      expect(plan.commits, 0);
      expect(fixture.service.opens, 1);
      fixture.session.recoveryGate!.complete();
      await tester.pump();
      expect(plan.commits, 0);
      fixture.session.closeGate!.complete();
      await tester.pump();
      expect(fixture.session.closed, isTrue);
      expect(plan.commits, 1);
      expect(fixture.service.opens, 1);
      fixture.owner.memoryPressure();
      final replacement = _Session();
      fixture.service.reopened.complete(replacement);
      plan.finished.complete();
      await tester.pump();
      expect(plan.disposed, isTrue);
      expect(replacement.runs, 1);
      expect(replacement.activityAtRun, (false, false, false, true));
      expect(replacement.recovered, [("root", BigInt.one)]);
      await fixture.owner.close();
      fixture.dispose();
    },
  );

  testWidgets(
    "explicit settings discard a prepared plan while retirement is pending",
    (tester) async {
      final fixture = _Fixture();
      final plan = _Maintenance();
      fixture.session.maintenance = plan;
      fixture.session.closeGate = Completer<void>();
      fixture.owner.startInBackground();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      fixture.session.finished.complete();
      await tester.pump();
      var saved = false;
      final change = fixture.owner.changeConfiguration(() async {
        saved = true;
      });
      await tester.pump();
      expect(plan.cancelled, isTrue);
      expect(saved, isFalse);
      fixture.service.reopened.complete(null);
      fixture.session.closeGate!.complete();
      await tester.pump();
      await change;
      expect(saved, isTrue);
      expect(plan.commits, 0);
      expect(plan.disposed, isTrue);
      await fixture.owner.close();
      fixture.dispose();
    },
  );

  testWidgets(
    "shutdown joins an admitted maintenance commit and prevents reopening",
    (tester) async {
      final fixture = _Fixture();
      final plan = _Maintenance();
      fixture.session.maintenance = plan;
      fixture.owner.startInBackground();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      fixture.session.finished.complete();
      await tester.pump();
      expect(plan.commits, 1);
      var closed = false;
      final closing = fixture.owner.close().then((_) {
        closed = true;
      });
      await tester.pump();
      expect(plan.cancelled, isTrue);
      expect(closed, isFalse);
      plan.finished.complete();
      await tester.pump();
      await closing;
      expect(plan.disposed, isTrue);
      expect(fixture.service.opens, 1);
      fixture.dispose();
    },
  );

  testWidgets(
    "explicit configuration waits for an admitted maintenance commit",
    (tester) async {
      final fixture = _Fixture();
      final plan = _Maintenance();
      fixture.session.maintenance = plan;
      fixture.owner.startInBackground();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      fixture.session.finished.complete();
      await tester.pump();
      var saved = false;
      final change = fixture.owner.changeConfiguration(() async {
        saved = true;
      });
      await tester.pump();
      expect(plan.cancelled, isTrue);
      expect(saved, isFalse);
      fixture.service.reopened.complete(null);
      plan.finished.complete();
      await tester.pump();
      await change;
      expect(saved, isTrue);
      expect(plan.disposed, isTrue);
      expect(fixture.service.opens, 2);
      await fixture.owner.close();
      fixture.dispose();
    },
  );

  for (final failure in ["close request", "retirement", "commit"]) {
    testWidgets("failed $failure releases maintenance without reopening", (
      tester,
    ) async {
      final fixture = _Fixture();
      final plan = _Maintenance();
      fixture.session.maintenance = plan;
      fixture.session.failCloseRequest = failure == "close request";
      fixture.session.closeGate = Completer<void>();
      fixture.owner.startInBackground();
      fixture.service.opened.complete(fixture.session);
      await tester.pump();
      fixture.session.finished.complete();
      await tester.pump();
      if (failure == "close request") {
        fixture.session.closeGate!.complete();
      } else if (failure == "retirement") {
        fixture.session.closeGate!.completeError(
          StateError("retirement failed"),
        );
      } else {
        fixture.session.closeGate!.complete();
        await tester.pump();
        plan.finished.completeError(StateError("commit failed"));
      }
      await tester.pump();
      expect(fixture.owner.error, isStateError);
      expect(plan.commits, failure == "commit" ? 1 : 0);
      expect(plan.disposed, isTrue);
      expect(fixture.service.opens, 1);
      await expectLater(fixture.owner.close(), throwsStateError);
      fixture.dispose();
    });
  }
}

class _Maintenance implements LibraryClassificationMaintenance {
  final finished = Completer<void>();
  int commits = 0;
  bool cancelled = false;
  bool disposed = false;

  @override
  Future<void> commit() {
    commits++;
    return finished.future;
  }

  @override
  void cancel() => cancelled = true;
  @override
  void dispose() => disposed = true;
}

class _SetupConfiguration implements LibraryClassificationConfigurationService {
  _SetupConfiguration({this.configured = true});
  final bool configured;
  final actions = <String>[];
  int initialLoads = 0;
  LibraryClassificationSettings? savedObserved;
  LibraryClassificationConfigurationAction? savedAction;
  List<LibraryClassificationCategory>? savedCategories;
  static const categories = [
    LibraryClassificationCategory(
      id: "drawing",
      label: "绘画",
      descriptions: ["a drawing"],
    ),
  ];
  late final settings = LibraryClassificationSettings(
    revision: BigInt.from(17),
    dispatchRevision: BigInt.from(29),
    mode: LibraryClassificationMode.disabled,
    categories: configured ? categories : [],
    runId: null,
    installationSelected: false,
  );
  @override
  Future<LibraryClassificationSettings> load() async => settings;
  @override
  Future<List<LibraryClassificationCategory>> initialCategories() async {
    initialLoads += 1;
    return categories;
  }

  @override
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationSettings observed,
    required List<LibraryClassificationCategory> categories,
    required LibraryClassificationConfigurationAction action,
  }) async {
    savedAction = action;
    actions.add("enable");
    savedObserved = observed;
    savedCategories = categories;
    return settings;
  }

  @override
  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  }) => throw UnimplementedError();
}

class _Fixture {
  final service = _Service();
  final session = _Session();
  final reads = LibraryForegroundReads();
  late final owner = LibraryClassificationLifecycle(
    service,
    foregroundReads: reads,
  );

  void update(
    int revision, {
    bool busy = false,
    Map<String, BigInt> roots = const {},
  }) => owner.updateCatalog(
    revision: BigInt.from(revision),
    availableRoots: roots,
    isBusy: busy,
  );

  void dispose() {
    owner.dispose();
    reads.dispose();
  }
}

class _Service implements LibraryClassification {
  final opened = Completer<LibraryClassificationSession?>();
  final reopened = Completer<LibraryClassificationSession?>();
  int opens = 0;
  @override
  Future<LibraryClassificationSession?> open() {
    opens++;
    return opens == 1 ? opened.future : reopened.future;
  }
}

class _Session implements LibraryClassificationSession {
  final finished = Completer<void>();
  Completer<void>? recoveryGate;
  Completer<void>? closeGate;
  bool autoFinish = true;
  bool closeRequested = false;
  bool closed = false;
  bool failCloseRequest = false;
  bool failForeground = false;
  LibraryClassificationMaintenance? maintenance;
  int runs = 0;
  int catalogNotifications = 0;
  final recovered = <(String, BigInt)>[];
  (bool, bool, bool, bool) activity = (false, false, true, false);
  (bool, bool, bool, bool)? activityAtRun;

  @override
  Future<LibraryClassificationMaintenance?> run() async {
    runs++;
    activityAtRun = activity;
    await finished.future;
    return maintenance;
  }

  @override
  void foregroundChanged({
    required bool readingSources,
    required bool importing,
    required bool suspended,
    required bool memoryPressure,
  }) {
    if (failForeground) throw StateError("foreground signal failed");
    activity = (readingSources, importing, suspended, memoryPressure);
  }

  @override
  void browsingActivity() {}
  @override
  void catalogChanged() => catalogNotifications++;
  @override
  Future<void> rootAvailable(String rootId, BigInt generation) async {
    recovered.add((rootId, generation));
    await recoveryGate?.future;
  }

  @override
  void requestClose() {
    closeRequested = true;
    if (failCloseRequest) {
      throw StateError("close request failed");
    }
    if (autoFinish && !finished.isCompleted) {
      finished.complete();
    }
  }

  @override
  Future<void> close() async {
    await closeGate?.future;
    closed = true;
  }
}
