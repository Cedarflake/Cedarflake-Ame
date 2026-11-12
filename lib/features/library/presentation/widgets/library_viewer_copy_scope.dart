import "package:material_ui/material_ui.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_feedback_style.dart";
import "../../../../app/presentation/ame_notification_lifetime.dart";
import "../../application/library_image_clipboard.dart";
import "../coordination/library_viewer_clipboard.dart";
import "library_viewer_notice.dart";

class LibraryViewerCopyScope extends ConsumerStatefulWidget {
  const LibraryViewerCopyScope({
    required this.locationId,
    required this.quarterTurns,
    required this.child,
    super.key,
  });

  final String locationId;
  final int quarterTurns;
  final Widget child;

  static LibraryViewerCopyData of(BuildContext context) => maybeOf(context)!;

  static LibraryViewerCopyData? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LibraryViewerCopyData>();

  @override
  ConsumerState<LibraryViewerCopyScope> createState() =>
      _LibraryViewerCopyScopeState();
}

class _LibraryViewerCopyScopeState
    extends ConsumerState<LibraryViewerCopyScope> {
  final _clipboard = LibraryViewerClipboard();
  AmeNotificationEntry? _notice;

  @override
  void didUpdateWidget(LibraryViewerCopyScope oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.locationId != widget.locationId) {
      _clipboard.clear();
      _notice = null;
    }
  }

  @override
  void dispose() {
    _clipboard.clear();
    super.dispose();
  }

  Future<void> _copy() async {
    final location = widget.locationId;
    try {
      final copied = await _clipboard.copy(
        ref.read(libraryImageClipboardProvider),
        quarterTurns: widget.quarterTurns,
      );
      if (copied && mounted && location == widget.locationId) {
        _show("图片已复制到剪贴板", AmeNotificationSeverity.success);
      }
    } on Object catch (error) {
      if (!mounted || location != widget.locationId) return;
      _show(
        error is ViewerClipboardImageTooLarge ? "图片过大，无法复制到剪贴板" : "无法复制图片，请重试",
        AmeNotificationSeverity.error,
        canRetry: error is! ViewerClipboardImageTooLarge,
      );
    }
  }

  void _show(
    String message,
    AmeNotificationSeverity severity, {
    bool canRetry = false,
  }) {
    final now = DateTime.now();
    setState(
      () => _notice = AmeNotificationEntry(
        id: "viewer-copy-${now.microsecondsSinceEpoch}",
        title: message,
        message: "",
        severity: severity,
        occurredAt: now,
        isUnread: false,
        isActive: true,
        isPersistent: severity == AmeNotificationSeverity.error,
        actionLabel: severity == AmeNotificationSeverity.error
            ? (canRetry ? "重试" : "知道了")
            : null,
      ),
    );
  }

  void _dismiss(String id) {
    if (mounted && _notice?.id == id) setState(() => _notice = null);
  }

  @override
  Widget build(BuildContext context) {
    final notice = _notice;
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(
          LogicalKeyboardKey.keyC,
          control: true,
          includeRepeats: false,
        ): _copy,
      },
      child: LibraryViewerCopyData(
        clipboard: _clipboard,
        copy: _copy,
        notification: notice,
        dismiss: _dismiss,
        notice: notice == null
            ? null
            : AmeNotificationLifetime(
                notification: notice,
                onDismiss: _dismiss,
                child: LibraryViewerNotice(
                  icon: ameNotificationIcon(notice.severity),
                  message: notice.title,
                  actionLabel: notice.actionLabel,
                  onAction: notice.actionLabel == "重试"
                      ? _copy
                      : () => _dismiss(notice.id),
                ),
              ),
        child: widget.child,
      ),
    );
  }
}

class LibraryViewerCopyData extends InheritedWidget {
  const LibraryViewerCopyData({
    required this.clipboard,
    required this.copy,
    required this.notification,
    required this.dismiss,
    required this.notice,
    required super.child,
    super.key,
  });

  final LibraryViewerClipboard clipboard;
  final VoidCallback copy;
  final AmeNotificationEntry? notification;
  final ValueChanged<String> dismiss;
  final Widget? notice;

  @override
  bool updateShouldNotify(LibraryViewerCopyData oldWidget) =>
      notice != oldWidget.notice;
}
