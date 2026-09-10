#include "single_instance_lock.h"

#include <sddl.h>

#include <string>
#include <vector>

namespace {

std::wstring CurrentUserSid() {
  HANDLE token = nullptr;
  if (!::OpenProcessToken(::GetCurrentProcess(), TOKEN_QUERY, &token)) {
    return {};
  }

  DWORD token_user_size = 0;
  ::GetTokenInformation(token, TokenUser, nullptr, 0, &token_user_size);
  if (::GetLastError() != ERROR_INSUFFICIENT_BUFFER || token_user_size == 0) {
    ::CloseHandle(token);
    return {};
  }

  std::vector<BYTE> token_user_buffer(token_user_size);
  if (!::GetTokenInformation(token, TokenUser, token_user_buffer.data(),
                             token_user_size, &token_user_size)) {
    ::CloseHandle(token);
    return {};
  }
  ::CloseHandle(token);

  const auto* token_user =
      reinterpret_cast<const TOKEN_USER*>(token_user_buffer.data());
  LPWSTR sid_text = nullptr;
  if (!::ConvertSidToStringSidW(token_user->User.Sid, &sid_text)) {
    return {};
  }
  std::wstring result(sid_text);
  ::LocalFree(sid_text);
  return result;
}

}  // namespace

SingleInstanceLock::SingleInstanceLock() = default;

SingleInstanceLock::~SingleInstanceLock() {
  if (owns_mutex_) {
    ::ReleaseMutex(mutex_);
  }
  if (mutex_ != nullptr) {
    ::CloseHandle(mutex_);
  }
}

SingleInstanceLockResult SingleInstanceLock::Acquire() {
  if (mutex_ != nullptr) {
    return SingleInstanceLockResult::kFailed;
  }

  const std::wstring user_sid = CurrentUserSid();
  if (user_sid.empty()) {
    return SingleInstanceLockResult::kFailed;
  }
  const std::wstring mutex_name =
      L"Global\\Cedarflake.Ame.SingleInstance." + user_sid;

  ::SetLastError(ERROR_SUCCESS);
  HANDLE mutex = ::CreateMutexW(nullptr, TRUE, mutex_name.c_str());
  if (mutex == nullptr) {
    return SingleInstanceLockResult::kFailed;
  }
  if (::GetLastError() == ERROR_ALREADY_EXISTS) {
    ::CloseHandle(mutex);
    return SingleInstanceLockResult::kAlreadyRunning;
  }

  mutex_ = mutex;
  owns_mutex_ = true;
  return SingleInstanceLockResult::kAcquired;
}
