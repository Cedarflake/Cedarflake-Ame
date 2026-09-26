import "dart:async";

import "package:flutter/material.dart";

class LibraryTaskLiveRegion extends StatefulWidget {
  const LibraryTaskLiveRegion({
    required this.message,
    required this.announcementScope,
    required this.child,
    this.throttle = const Duration(milliseconds: 800),
    super.key,
  });

  final String message;
  final Object announcementScope;
  final Duration throttle;
  final Widget child;

  @override
  State<LibraryTaskLiveRegion> createState() => _LibraryTaskLiveRegionState();
}

class _LibraryTaskLiveRegionState extends State<LibraryTaskLiveRegion> {
  late String _announcedMessage;
  Timer? _announcementTimer;
  String? _pendingMessage;

  @override
  void initState() {
    super.initState();
    _announcedMessage = widget.message;
  }

  @override
  void didUpdateWidget(covariant LibraryTaskLiveRegion oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.announcementScope != widget.announcementScope) {
      _announcementTimer?.cancel();
      _announcementTimer = null;
      _pendingMessage = null;
      _announcedMessage = widget.message;
      return;
    }
    if (_announcedMessage == widget.message) {
      _pendingMessage = null;
      return;
    }
    _pendingMessage = widget.message;
    _announcementTimer ??= Timer(widget.throttle, _flushPendingMessage);
  }

  @override
  void dispose() {
    _announcementTimer?.cancel();
    super.dispose();
  }

  void _flushPendingMessage() {
    _announcementTimer = null;
    final message = _pendingMessage;
    _pendingMessage = null;
    if (!mounted || message == null || message == _announcedMessage) {
      return;
    }
    setState(() => _announcedMessage = message);
  }

  @override
  Widget build(BuildContext context) {
    return Semantics(
      key: const Key("library-task-live-region"),
      container: true,
      explicitChildNodes: true,
      liveRegion: true,
      label: _announcedMessage,
      child: widget.child,
    );
  }
}
