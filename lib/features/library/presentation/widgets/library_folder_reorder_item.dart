import "package:flutter/gestures.dart";
import "package:material_ui/material_ui.dart";

import "library_folder_disclosure.dart";

class LibraryFolderReorderItem extends StatefulWidget {
  const LibraryFolderReorderItem({
    required this.index,
    required this.enabled,
    required this.header,
    required this.child,
    required this.onCollapse,
    super.key,
  });

  final int index;
  final bool enabled;
  final Widget header;
  final Widget? child;
  final VoidCallback onCollapse;

  @override
  State<LibraryFolderReorderItem> createState() =>
      _LibraryFolderReorderItemState();
}

class _LibraryFolderReorderItemState extends State<LibraryFolderReorderItem> {
  bool _preparingDrag = false;

  void _prepareDrag() {
    setState(() => _preparingDrag = true);
    widget.onCollapse();
  }

  void _finishDrag() {
    if (mounted) setState(() => _preparingDrag = false);
  }

  @override
  Widget build(BuildContext context) => Column(
    children: [
      _FolderDragStartListener(
        index: widget.index,
        enabled: widget.enabled,
        prepare: _prepareDrag,
        finish: _finishDrag,
        child: widget.header,
      ),
      LibraryFolderDisclosure(
        animate: !_preparingDrag,
        child: _preparingDrag ? null : widget.child,
      ),
    ],
  );
}

class _FolderDragStartListener extends ReorderableDragStartListener {
  const _FolderDragStartListener({
    required super.index,
    required super.enabled,
    required super.child,
    required this.prepare,
    required this.finish,
  });

  final VoidCallback prepare;
  final VoidCallback finish;

  @override
  MultiDragGestureRecognizer createRecognizer() =>
      _FolderDragRecognizer(prepare, finish);
}

class _FolderDragRecognizer extends ImmediateMultiDragGestureRecognizer {
  _FolderDragRecognizer(this.prepare, this.finish);

  final VoidCallback prepare;
  final VoidCallback finish;
  _PreparedFolderDrag? _pending;

  @override
  MultiDragPointerState createNewPointerState(PointerDownEvent event) =>
      _FolderPointerState(event.position, event.kind, gestureSettings);

  @override
  set onStart(GestureMultiDragStartCallback? start) {
    super.onStart = start == null
        ? null
        : (position) {
            prepare();
            final drag = _pending = _PreparedFolderDrag(finish);
            // ReorderableList measures the item when its drag starts. Let the
            // collapsed row finish layout before handing ownership to it.
            WidgetsBinding.instance.addPostFrameCallback((_) {
              drag.start(() => start(position));
            });
            return drag;
          };
  }

  @override
  void dispose() {
    _pending?.retire();
    super.dispose();
  }
}

class _FolderPointerState extends MultiDragPointerState {
  _FolderPointerState(super.initialPosition, super.kind, super.gestureSettings);

  GestureMultiDragStartCallback? _acceptedStart;

  // Match Material taps: a mouse's one-pixel drag slop is too small for a row
  // that contains buttons and collapses when sorting starts.
  bool get _passedClickTolerance =>
      pendingDelta!.distance > (gestureSettings?.touchSlop ?? kTouchSlop);

  @override
  void checkForResolutionAfterMove() {
    if (!_passedClickTolerance) return;
    final start = _acceptedStart;
    if (start == null) {
      resolve(GestureDisposition.accepted);
    } else {
      _acceptedStart = null;
      start(initialPosition);
    }
  }

  @override
  void accepted(GestureMultiDragStartCallback starter) {
    if (_passedClickTolerance) {
      starter(initialPosition);
    } else {
      // Winning an otherwise empty arena does not establish drag intent.
      _acceptedStart = starter;
    }
  }
}

class _PreparedFolderDrag extends Drag {
  _PreparedFolderDrag(this.finish);

  final VoidCallback finish;
  Drag? _delegate;
  DragUpdateDetails? _pendingUpdate;
  bool _retired = false;

  void retire() {
    if (_retired) return;
    _retired = true;
    _pendingUpdate = null;
    // The list disposes its drag before its recognizer. Retire only our
    // deferred handoff here; canceling that framework drag would dispose twice.
    WidgetsBinding.instance.addPostFrameCallback((_) => finish());
  }

  void start(Drag? Function() create) {
    if (_retired) return;
    _delegate = create();
    if (_pendingUpdate case final update?) _delegate?.update(update);
    _pendingUpdate = null;
  }

  @override
  void update(DragUpdateDetails details) {
    if (_retired) return;
    if (_delegate case final delegate?) {
      delegate.update(details);
    } else {
      _pendingUpdate = DragUpdateDetails(
        delta: (_pendingUpdate?.delta ?? Offset.zero) + details.delta,
        globalPosition: details.globalPosition,
        localPosition: details.localPosition,
        sourceTimeStamp: details.sourceTimeStamp,
      );
    }
  }

  @override
  void end(DragEndDetails details) {
    if (_retired) return;
    _retired = true;
    _delegate?.end(details);
    finish();
  }

  @override
  void cancel() {
    if (_retired) return;
    _retired = true;
    _delegate?.cancel();
    finish();
  }
}
