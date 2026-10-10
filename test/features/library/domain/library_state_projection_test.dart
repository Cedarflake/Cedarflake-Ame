import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("removal owns status and kind even during scan and query work", () {
    const state = LibraryState(
      status: LibraryStatus.removing,
      taskKind: LibraryTaskKind.remove,
      queryActivity: _loading,
      primaryScan: LibraryPrimaryScanSnapshot(
        status: LibraryStatus.scanning,
        taskKind: LibraryTaskKind.import,
        scanId: "scan",
      ),
    );
    _expectProjection(
      state,
      status: LibraryStatus.removing,
      kind: LibraryTaskKind.remove,
      processing: true,
      taskProcessing: true,
      busy: true,
    );
    _expectProjection(
      state.copyWith(status: LibraryStatus.failed, queryActivity: _idle),
      status: LibraryStatus.failed,
      kind: LibraryTaskKind.remove,
    );
  });

  test("committed removal keeps admission busy after task processing ends", () {
    final state = const LibraryState(
      status: LibraryStatus.completed,
      taskKind: LibraryTaskKind.remove,
      isRemovalCommitted: true,
    ).copyWith(roots: const []);
    _expectProjection(
      state,
      status: LibraryStatus.completed,
      kind: LibraryTaskKind.remove,
      busy: true,
    );
    expect(state.isCommittedRemovalReloadPending, isTrue);
    expect(state.copyWith(isRemovalCommitted: false).isBusy, isFalse);
  });

  test("an absent primary scan retains the legacy status and task kind", () {
    const state = LibraryState(
      status: LibraryStatus.paused,
      taskKind: LibraryTaskKind.update,
    );
    _expectProjection(
      state,
      status: LibraryStatus.paused,
      kind: LibraryTaskKind.update,
    );
    _expectProjection(
      state.copyWith(queryActivity: _loading),
      status: LibraryStatus.refreshing,
      kind: LibraryTaskKind.update,
      processing: true,
      taskProcessing: true,
      busy: true,
    );
  });

  for (final scanStatus in [LibraryStatus.empty, LibraryStatus.completed]) {
    for (final hasRoots in [false, true]) {
      test(
        "feedback-free $scanStatus derives catalog state with roots=$hasRoots",
        () {
          final state = LibraryState(
            status: LibraryStatus.failed,
            roots: hasRoots ? const [_root] : const [],
            taskKind: LibraryTaskKind.update,
            primaryScan: LibraryPrimaryScanSnapshot(status: scanStatus),
          );
          _expectProjection(
            state,
            status: hasRoots ? LibraryStatus.completed : LibraryStatus.empty,
            kind: null,
          );
          _expectProjection(
            state.copyWith(queryActivity: _loading),
            status: LibraryStatus.refreshing,
            kind: null,
            processing: true,
            taskProcessing: true,
            busy: true,
          );
        },
      );
    }
  }

  for (final feedback in [
    const LibraryPrimaryScanSnapshot(
      status: LibraryStatus.paused,
      scanId: "scan",
    ),
    const LibraryPrimaryScanSnapshot(
      status: LibraryStatus.paused,
      taskKind: LibraryTaskKind.import,
    ),
  ]) {
    test(
      "scan feedback overrides refreshing: id=${feedback.scanId}, kind=${feedback.taskKind}",
      () {
        final state = LibraryState(
          status: LibraryStatus.completed,
          taskKind: LibraryTaskKind.update,
          queryActivity: _loading,
          primaryScan: feedback,
        );
        _expectProjection(
          state,
          status: LibraryStatus.paused,
          kind: feedback.taskKind,
          processing: true,
          busy: true,
        );
      },
    );
  }

  test("completed feedback remains completed without catalog roots", () {
    const state = LibraryState(
      queryActivity: _loading,
      primaryScan: LibraryPrimaryScanSnapshot(
        status: LibraryStatus.completed,
        scanId: "completed",
      ),
    );
    expect(state.status, LibraryStatus.completed);
    expect(state.taskKind, isNull);
  });

  test("empty feedback remains empty with catalog roots", () {
    const state = LibraryState(
      roots: [_root],
      primaryScan: LibraryPrimaryScanSnapshot(
        status: LibraryStatus.empty,
        taskKind: LibraryTaskKind.import,
      ),
    );
    expect(state.status, LibraryStatus.empty);
    expect(state.taskKind, LibraryTaskKind.import);
  });

  for (final expected in const [
    _PrimaryStatusCase(
      LibraryStatus.choosingDirectory,
      processing: true,
      taskProcessing: true,
    ),
    _PrimaryStatusCase(
      LibraryStatus.scanning,
      scanning: true,
      processing: true,
      taskProcessing: true,
    ),
    _PrimaryStatusCase(
      LibraryStatus.pausing,
      scanning: true,
      processing: true,
      taskProcessing: true,
    ),
    _PrimaryStatusCase(
      LibraryStatus.cancelling,
      scanning: true,
      processing: true,
      taskProcessing: true,
    ),
    _PrimaryStatusCase(LibraryStatus.discarding, taskProcessing: true),
    _PrimaryStatusCase(
      LibraryStatus.refreshing,
      processing: true,
      taskProcessing: true,
    ),
    _PrimaryStatusCase(LibraryStatus.paused),
    _PrimaryStatusCase(LibraryStatus.failed),
    _PrimaryStatusCase(LibraryStatus.cancelled),
    _PrimaryStatusCase(LibraryStatus.stale),
    _PrimaryStatusCase(LibraryStatus.empty),
    _PrimaryStatusCase(LibraryStatus.completed),
  ]) {
    test(
      "primary ${expected.status} preserves its dependent admission flags",
      () {
        final state = LibraryState(
          roots: const [_root],
          primaryScan: LibraryPrimaryScanSnapshot(
            status: expected.status,
            taskKind: LibraryTaskKind.import,
          ),
        );
        _expectProjection(
          state,
          status: expected.status,
          kind: LibraryTaskKind.import,
          scanning: expected.scanning,
          processing: expected.processing,
          taskProcessing: expected.taskProcessing,
          busy: expected.processing,
        );
      },
    );
  }

  test("feedback-free nonterminal scan state is not derived from roots", () {
    const state = LibraryState(
      roots: [_root],
      primaryScan: LibraryPrimaryScanSnapshot(status: LibraryStatus.paused),
    );
    _expectProjection(state, status: LibraryStatus.paused, kind: null);
    expect(
      state.copyWith(queryActivity: _loading).status,
      LibraryStatus.refreshing,
    );
    expect(state.copyWith(isLoadingTimeAnchor: true).isBusy, isTrue);
  });

  test(
    "catalog copy cannot replace primary feedback and explicit replacement can",
    () {
      const scan = LibraryPrimaryScanSnapshot(
        status: LibraryStatus.paused,
        taskKind: LibraryTaskKind.import,
        scanId: "retained",
      );
      const state = LibraryState(primaryScan: scan);
      final copied = state.copyWith(
        roots: const [_root],
        status: LibraryStatus.completed,
        taskKind: LibraryTaskKind.update,
      );
      _expectProjection(
        copied,
        status: LibraryStatus.paused,
        kind: LibraryTaskKind.import,
      );
      expect(copied.primaryScan, same(scan));
      _expectProjection(
        copied.copyWith(
          primaryScan: scan.copyWith(status: LibraryStatus.failed),
        ),
        status: LibraryStatus.failed,
        kind: LibraryTaskKind.import,
      );
      _expectProjection(
        copied.copyWith(
          primaryScan: null,
          status: LibraryStatus.completed,
          taskKind: null,
        ),
        status: LibraryStatus.completed,
        kind: null,
      );
    },
  );
}

class _PrimaryStatusCase {
  const _PrimaryStatusCase(
    this.status, {
    this.scanning = false,
    this.processing = false,
    this.taskProcessing = false,
  });

  final LibraryStatus status;
  final bool scanning;
  final bool processing;
  final bool taskProcessing;
}

void _expectProjection(
  LibraryState state, {
  required LibraryStatus status,
  required LibraryTaskKind? kind,
  bool scanning = false,
  bool processing = false,
  bool taskProcessing = false,
  bool busy = false,
}) {
  expect(state.status, status);
  expect(state.taskKind, kind);
  expect(state.isScanning, scanning);
  expect(state.isProcessing, processing);
  expect(state.isTaskProcessing, taskProcessing);
  expect(state.isBusy, busy);
}

const _loading = LibraryQueryLoading(LibraryGalleryQuery());
const _idle = LibraryQueryIdle();
const _root = LibraryRoot(
  id: "root",
  path: "C:\\Fixture",
  displayPath: "C:\\Fixture",
  createdUnixMs: 1,
  assetCount: 1,
  issueCount: 0,
  availability: LibraryRootAvailability.available,
);
