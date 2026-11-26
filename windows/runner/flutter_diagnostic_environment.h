#ifndef RUNNER_FLUTTER_DIAGNOSTIC_ENVIRONMENT_H_
#define RUNNER_FLUTTER_DIAGNOSTIC_ENVIRONMENT_H_

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>

#include <atomic>
#include <thread>

class FlutterDiagnosticEnvironment {
 public:
  explicit FlutterDiagnosticEnvironment(flutter::BinaryMessenger* messenger);
  ~FlutterDiagnosticEnvironment();

  FlutterDiagnosticEnvironment(const FlutterDiagnosticEnvironment&) = delete;
  FlutterDiagnosticEnvironment& operator=(const FlutterDiagnosticEnvironment&) = delete;

 private:
  flutter::MethodChannel<flutter::EncodableValue> channel_;
  std::atomic<bool> collecting_{false};
  std::thread worker_;
};

#endif  // RUNNER_FLUTTER_DIAGNOSTIC_ENVIRONMENT_H_
