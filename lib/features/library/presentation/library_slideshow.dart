import "dart:async";

import "package:flutter/foundation.dart";

/// A slide owns its dwell timer. Late image frames cannot advance another slide.
class LibrarySlideshow extends ChangeNotifier {
  LibrarySlideshow({this.interval = const Duration(seconds: 3)});

  final Duration interval;
  Timer? _timer;
  Object? _slide;
  Future<bool> Function()? _advance;
  bool _isReady = false;
  bool _isPlaying = false;
  bool _isDisposed = false;
  bool _hasFailed = false;
  int _playGeneration = 0;

  bool get isPlaying => _isPlaying;
  bool get canPlay => !_hasFailed && _advance != null;

  void show(Object slide, Future<bool> Function()? advance) {
    _advance = advance;
    if (_slide == slide) return;
    _timer?.cancel();
    _slide = slide;
    _isReady = false;
    _hasFailed = false;
  }

  void ready(Object slide) {
    if (_isDisposed || _slide != slide || _isReady) return;
    _isReady = true;
    final wasFailed = _hasFailed;
    _hasFailed = false;
    if (wasFailed) notifyListeners();
    _schedule();
  }

  void failed(Object slide) {
    if (_slide != slide || _isDisposed) return;
    _hasFailed = true;
    _isReady = false;
    pause();
    notifyListeners();
  }

  void toggle() {
    if (_isDisposed) return;
    if (_isPlaying) {
      pause();
      return;
    }
    if (!canPlay) return;
    _isPlaying = true;
    _playGeneration++;
    notifyListeners();
    _schedule();
  }

  void pause() {
    _timer?.cancel();
    _playGeneration++;
    if (!_isPlaying || _isDisposed) return;
    _isPlaying = false;
    notifyListeners();
  }

  void _schedule() {
    if (!_isPlaying || !_isReady || _isDisposed) return;
    _timer?.cancel();
    if (_advance == null) {
      pause();
      return;
    }
    final slide = _slide;
    final generation = _playGeneration;
    _timer = Timer(interval, () async {
      if (_isDisposed || !_isPlaying || _slide != slide) return;
      // The next selection and decoded frame start the next timer, never paging latency.
      _isReady = false;
      final advance = _advance;
      if (advance == null) {
        pause();
      } else {
        try {
          final moved = await advance();
          if (!moved && generation == _playGeneration) pause();
        } on Object {
          if (generation == _playGeneration) pause();
        }
      }
    });
  }

  @override
  void dispose() {
    _isDisposed = true;
    _timer?.cancel();
    super.dispose();
  }
}
