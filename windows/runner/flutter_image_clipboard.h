#ifndef RUNNER_FLUTTER_IMAGE_CLIPBOARD_H_
#define RUNNER_FLUTTER_IMAGE_CLIPBOARD_H_

#include <flutter/binary_messenger.h>
#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <windows.h>

class FlutterImageClipboard {
 public:
  FlutterImageClipboard(HWND window, flutter::BinaryMessenger *messenger);
  ~FlutterImageClipboard();

 private:
  flutter::MethodChannel<flutter::EncodableValue> channel_;
};

#endif  // RUNNER_FLUTTER_IMAGE_CLIPBOARD_H_
