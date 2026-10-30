#include "flutter_power_channel.h"

#include <flutter/standard_method_codec.h>

#include <memory>

FlutterPowerChannel::FlutterPowerChannel(
    HWND window, flutter::BinaryMessenger* messenger)
    : channel_(messenger, "cedarflake_ame/system_power",
               &flutter::StandardMethodCodec::GetInstance()) {
  registration_ =
      RegisterSuspendResumeNotification(window, DEVICE_NOTIFY_WINDOW_HANDLE);
  if (!registration_) {
    registration_error_ = GetLastError();
  }
  channel_.SetMethodCallHandler(
      [this](const flutter::MethodCall<flutter::EncodableValue>& call,
             std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
        if (call.method_name() != "getState") {
          result->NotImplemented();
        } else if (!registration_) {
          result->Error(
              "system_power_unavailable", "Windows power registration failed",
              flutter::EncodableValue(static_cast<std::int64_t>(registration_error_)));
        } else {
          result->Success(Snapshot());
        }
      });
}

FlutterPowerChannel::~FlutterPowerChannel() {
  channel_.SetMethodCallHandler(nullptr);
  if (registration_) {
    UnregisterSuspendResumeNotification(registration_);
  }
}

bool FlutterPowerChannel::HandlePowerBroadcast(WPARAM event) {
  switch (event) {
    case PBT_APMSUSPEND:
      if (!suspended_) {
        Publish(true);
      }
      return true;
    case PBT_APMRESUMEAUTOMATIC:
      // A wake notification can arrive without a prior suspend notification.
      Publish(false);
      return true;
    case PBT_APMRESUMESUSPEND:
      if (suspended_) {
        Publish(false);
      }
      return true;
    default:
      return false;
  }
}

flutter::EncodableValue FlutterPowerChannel::Snapshot() const {
  return flutter::EncodableValue(flutter::EncodableMap{
      {flutter::EncodableValue("revision"), flutter::EncodableValue(revision_)},
      {flutter::EncodableValue("suspended"), flutter::EncodableValue(suspended_)},
  });
}

void FlutterPowerChannel::Publish(bool suspended) {
  suspended_ = suspended;
  ++revision_;
  channel_.InvokeMethod("stateChanged",
                        std::make_unique<flutter::EncodableValue>(Snapshot()));
}
