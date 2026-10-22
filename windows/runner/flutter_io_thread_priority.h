#ifndef RUNNER_FLUTTER_IO_THREAD_PRIORITY_H_
#define RUNNER_FLUTTER_IO_THREAD_PRIORITY_H_

#include <windows.h>

#include <set>

enum class FlutterIoPriorityStatus { kAdjusted, kNoMatch, kAmbiguous, kFailed };

struct FlutterIoPriorityResult {
  FlutterIoPriorityStatus status;
  DWORD thread_id = 0;
  DWORD error = ERROR_SUCCESS;
};

// Scope this observation around synchronous Flutter engine construction, before
// plugin registration. The pinned Windows engine creates exactly one new
// BELOW_NORMAL thread in this interval: its image I/O runner.
class FlutterIoThreadPriority {
 public:
  FlutterIoThreadPriority();
  FlutterIoPriorityResult Apply() const;

 private:
  std::set<DWORD> existing_threads_;
  DWORD snapshot_error_ = ERROR_SUCCESS;
};

#endif  // RUNNER_FLUTTER_IO_THREAD_PRIORITY_H_
