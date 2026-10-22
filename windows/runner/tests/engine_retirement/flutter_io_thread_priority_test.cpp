#include "flutter_io_thread_priority.h"

#include <cstdio>
#include <cstdlib>

namespace {

void Require(bool condition, const char* message) {
  if (!condition) {
    std::fprintf(stderr, "FAIL: %s\n", message);
    std::exit(EXIT_FAILURE);
  }
}

class WaitingThread {
 public:
  explicit WaitingThread(int priority) {
    stop_ = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    Require(stop_ != nullptr, "Thread stop event must be created");
    thread_ = CreateThread(nullptr, 0, Wait, stop_, CREATE_SUSPENDED, &id_);
    Require(thread_ != nullptr, "Fixture thread must be created");
    Require(SetThreadPriority(thread_, priority) != FALSE,
            "Fixture thread priority must be set");
    Require(ResumeThread(thread_) != static_cast<DWORD>(-1),
            "Fixture thread must start");
  }

  ~WaitingThread() {
    Require(SetEvent(stop_) != FALSE &&
                WaitForSingleObject(thread_, 5000) == WAIT_OBJECT_0,
            "Fixture thread must retire before its handles close");
    CloseHandle(thread_);
    CloseHandle(stop_);
  }

  DWORD id() const { return id_; }
  int priority() const { return GetThreadPriority(thread_); }

 private:
  static DWORD WINAPI Wait(void* stop) {
    return WaitForSingleObject(stop, INFINITE) == WAIT_OBJECT_0 ? 0 : 1;
  }

  HANDLE stop_ = nullptr;
  HANDLE thread_ = nullptr;
  DWORD id_ = 0;
};

}  // namespace

void VerifyFlutterIoThreadPriorityPolicy() {
  const WaitingThread existing_background(THREAD_PRIORITY_BELOW_NORMAL);
  {
    const FlutterIoThreadPriority observation;
    const WaitingThread image_io(THREAD_PRIORITY_BELOW_NORMAL);
    const WaitingThread raster(THREAD_PRIORITY_ABOVE_NORMAL);
    const WaitingThread ordinary(THREAD_PRIORITY_NORMAL);
    const auto result = observation.Apply();
    Require(result.status == FlutterIoPriorityStatus::kAdjusted &&
                result.thread_id == image_io.id() &&
                image_io.priority() == THREAD_PRIORITY_NORMAL,
            "Only the new background runner must be normalized");
    Require(existing_background.priority() == THREAD_PRIORITY_BELOW_NORMAL &&
                raster.priority() == THREAD_PRIORITY_ABOVE_NORMAL &&
                ordinary.priority() == THREAD_PRIORITY_NORMAL,
            "Existing workers and other priorities must remain unchanged");
    Require(observation.Apply().status == FlutterIoPriorityStatus::kNoMatch,
            "Repeating application must not raise another priority");
  }
  {
    const FlutterIoThreadPriority observation;
    const WaitingThread first(THREAD_PRIORITY_BELOW_NORMAL);
    const WaitingThread second(THREAD_PRIORITY_BELOW_NORMAL);
    Require(observation.Apply().status == FlutterIoPriorityStatus::kAmbiguous &&
                first.priority() == THREAD_PRIORITY_BELOW_NORMAL &&
                second.priority() == THREAD_PRIORITY_BELOW_NORMAL,
            "Ambiguous startup threads must not be changed");
  }
  {
    const FlutterIoThreadPriority observation;
    { const WaitingThread retired(THREAD_PRIORITY_BELOW_NORMAL); }
    Require(observation.Apply().status == FlutterIoPriorityStatus::kNoMatch,
            "A retired startup thread must not be adjusted");
  }
  Require(existing_background.priority() == THREAD_PRIORITY_BELOW_NORMAL,
          "Existing background work must retain its priority");
  std::printf("IO_PRIORITY_POLICY new_only=1 ambiguous_unchanged=1 retired_ignored=1\n");
}
