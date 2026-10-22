#include "flutter_io_thread_priority.h"

#include <tlhelp32.h>

#include <memory>
#include <vector>

namespace {

struct HandleCloser {
  void operator()(HANDLE handle) const { CloseHandle(handle); }
};

using OwnedHandle = std::unique_ptr<void, HandleCloser>;

DWORD ReadProcessThreads(std::set<DWORD>& threads) {
  const HANDLE raw = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
  if (raw == INVALID_HANDLE_VALUE) {
    return GetLastError();
  }
  const OwnedHandle snapshot(raw);
  THREADENTRY32 entry{};
  entry.dwSize = sizeof(entry);
  if (!Thread32First(snapshot.get(), &entry)) {
    return GetLastError();
  }
  do {
    if (entry.th32OwnerProcessID == GetCurrentProcessId()) {
      threads.insert(entry.th32ThreadID);
    }
    entry.dwSize = sizeof(entry);
  } while (Thread32Next(snapshot.get(), &entry));
  const DWORD error = GetLastError();
  if (error != ERROR_NO_MORE_FILES) {
    return error;
  }
  return ERROR_SUCCESS;
}

}  // namespace

FlutterIoThreadPriority::FlutterIoThreadPriority()
    : snapshot_error_(ReadProcessThreads(existing_threads_)) {}

FlutterIoPriorityResult FlutterIoThreadPriority::Apply() const {
  if (snapshot_error_ != ERROR_SUCCESS) {
    return {FlutterIoPriorityStatus::kFailed, 0, snapshot_error_};
  }
  std::set<DWORD> current_threads;
  const DWORD error = ReadProcessThreads(current_threads);
  if (error != ERROR_SUCCESS) {
    return {FlutterIoPriorityStatus::kFailed, 0, error};
  }

  std::vector<OwnedHandle> candidates;
  for (const DWORD id : current_threads) {
    if (existing_threads_.count(id) != 0) {
      continue;
    }
    OwnedHandle thread(OpenThread(THREAD_QUERY_LIMITED_INFORMATION |
                                      THREAD_SET_INFORMATION | SYNCHRONIZE,
                                  FALSE, id));
    if (!thread) {
      const DWORD open_error = GetLastError();
      if (open_error == ERROR_INVALID_PARAMETER) {
        continue;  // A short-lived startup worker already exited.
      }
      return {FlutterIoPriorityStatus::kFailed, 0, open_error};
    }
    if (GetProcessIdOfThread(thread.get()) != GetCurrentProcessId() ||
        WaitForSingleObject(thread.get(), 0) == WAIT_OBJECT_0) {
      continue;
    }
    const int priority = GetThreadPriority(thread.get());
    if (priority == THREAD_PRIORITY_ERROR_RETURN) {
      return {FlutterIoPriorityStatus::kFailed, 0, GetLastError()};
    }
    if (priority == THREAD_PRIORITY_BELOW_NORMAL) {
      candidates.push_back(std::move(thread));
    }
  }
  if (candidates.empty()) {
    return {FlutterIoPriorityStatus::kNoMatch};
  }
  if (candidates.size() != 1) {
    return {FlutterIoPriorityStatus::kAmbiguous};
  }

  const HANDLE thread = candidates.front().get();
  if (WaitForSingleObject(thread, 0) != WAIT_TIMEOUT ||
      GetThreadPriority(thread) != THREAD_PRIORITY_BELOW_NORMAL) {
    return {FlutterIoPriorityStatus::kNoMatch};
  }
  const DWORD id = GetThreadId(thread);
  if (!SetThreadPriority(thread, THREAD_PRIORITY_NORMAL)) {
    return {FlutterIoPriorityStatus::kFailed, id, GetLastError()};
  }
  if (GetThreadPriority(thread) != THREAD_PRIORITY_NORMAL) {
    return {FlutterIoPriorityStatus::kFailed, id, ERROR_INVALID_STATE};
  }
  return {FlutterIoPriorityStatus::kAdjusted, id};
}
