#ifndef RUNNER_SINGLE_INSTANCE_LOCK_H_
#define RUNNER_SINGLE_INSTANCE_LOCK_H_

#include <windows.h>

enum class SingleInstanceLockResult {
  kAcquired,
  kAlreadyRunning,
  kFailed,
};

class SingleInstanceLock {
 public:
  SingleInstanceLock();
  ~SingleInstanceLock();

  SingleInstanceLock(const SingleInstanceLock&) = delete;
  SingleInstanceLock& operator=(const SingleInstanceLock&) = delete;

  SingleInstanceLockResult Acquire();
  bool RetainForHandoff();
  void Release();

 private:
  HANDLE mutex_ = nullptr;
  bool owns_mutex_ = false;
};

#endif  // RUNNER_SINGLE_INSTANCE_LOCK_H_
