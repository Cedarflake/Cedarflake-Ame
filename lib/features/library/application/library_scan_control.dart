import "library_scanner.dart";
import "../domain/library_models.dart";

enum LibraryScanCommand { suspend, pause, cancel }

/// A false native result before Started is not acknowledgement of user intent.
class LibraryScanControl {
  LibraryScanControl({required this.scanId, required this._scanner});

  final String scanId;
  final LibraryScanner _scanner;
  LibraryScanCommand? _command;
  bool _isApplied = false;
  bool _isStarted = false;
  bool _isRetired = false;
  LibraryScanFailure? _failure;

  LibraryScanCommand? get command => _isRetired ? null : _command;
  LibraryScanFailure? get failure => _isRetired ? null : _failure;

  bool request(LibraryScanCommand command) {
    if (_isRetired ||
        (_command != null && command.index < _command!.index) ||
        (command == _command && _failure == null)) {
      return false;
    }
    _command = command;
    _isApplied = _send(command);
    return true;
  }

  void started() {
    if (_isRetired || _isStarted) {
      return;
    }
    _isStarted = true;
    final command = _command;
    if (command != null && !_isApplied) {
      _isApplied = _send(command);
    }
  }

  void retire() => _isRetired = true;

  bool _send(LibraryScanCommand command) {
    try {
      final applied = switch (command) {
        LibraryScanCommand.cancel => _scanner.cancel(scanId),
        LibraryScanCommand.pause => _scanner.pause(scanId),
        LibraryScanCommand.suspend => _scanner.suspend(scanId),
      };
      _failure = null;
      return applied;
    } on Object catch (error) {
      _failure = LibraryScanFailure(
        code: "bridge_scan_control_failed",
        message: error.toString(),
      );
      return false;
    }
  }
}
