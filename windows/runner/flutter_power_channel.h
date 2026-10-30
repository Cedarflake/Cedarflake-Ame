#ifndef RUNNER_FLUTTER_POWER_CHANNEL_H_
#define RUNNER_FLUTTER_POWER_CHANNEL_H_

#include <windows.h>

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>

#include <cstdint>

class FlutterPowerChannel {
 public:
  FlutterPowerChannel(HWND window, flutter::BinaryMessenger* messenger);
  ~FlutterPowerChannel();
  FlutterPowerChannel(const FlutterPowerChannel&) = delete;
  FlutterPowerChannel& operator=(const FlutterPowerChannel&) = delete;

  bool HandlePowerBroadcast(WPARAM event);

 private:
  flutter::EncodableValue Snapshot() const;
  void Publish(bool suspended);

  flutter::MethodChannel<flutter::EncodableValue> channel_;
  HPOWERNOTIFY registration_ = nullptr;
  DWORD registration_error_ = ERROR_SUCCESS;
  std::int64_t revision_ = 0;
  bool suspended_ = false;
};

#endif  // RUNNER_FLUTTER_POWER_CHANNEL_H_
