#include "user_data_cleanup.h"

#include <sddl.h>
#include <shlobj.h>
#include <wtsapi32.h>

#include <stdexcept>

namespace ame::installer {
namespace {

FileHandle CurrentToken() {
  HANDLE raw = nullptr;
  constexpr DWORD rights = TOKEN_QUERY | TOKEN_IMPERSONATE | TOKEN_DUPLICATE;
  if (!OpenThreadToken(GetCurrentThread(), rights, TRUE, &raw)) {
    if (GetLastError() != ERROR_NO_TOKEN ||
        !OpenProcessToken(GetCurrentProcess(), rights, &raw)) {
      throw std::runtime_error("Cannot identify the account requesting data removal");
    }
  }
  return FileHandle(raw);
}

LocalAllocation ParseUserSid(const std::wstring& text) {
  PSID sid = nullptr;
  if (text.empty() || text.size() > 184 || !ConvertStringSidToSidW(text.c_str(), &sid)) {
    throw std::runtime_error("The data-removal account identity is invalid");
  }
  LocalAllocation allocation(sid);
  if (IsWellKnownSid(sid, WinLocalSystemSid) || IsWellKnownSid(sid, WinLocalServiceSid) ||
      IsWellKnownSid(sid, WinNetworkServiceSid)) {
    throw std::runtime_error("Data removal requires a signed-in user account");
  }
  return allocation;
}

std::wstring CanonicalUserSid(PSID sid) {
  LPWSTR raw = nullptr;
  if (!ConvertSidToStringSidW(sid, &raw)) {
    throw std::runtime_error("Cannot normalize the data-removal account identity");
  }
  const LocalAllocation allocation(raw);
  return std::wstring(raw);
}

std::wstring SessionAccountPart(DWORD session_id, WTS_INFO_CLASS field) {
  LPWSTR raw = nullptr;
  DWORD bytes = 0;
  const BOOL result = WTSQuerySessionInformationW(WTS_CURRENT_SERVER_HANDLE, session_id,
                                                 field, &raw, &bytes);
  const std::unique_ptr<wchar_t, decltype(&WTSFreeMemory)> text(raw, WTSFreeMemory);
  if (!result || !text || bytes < sizeof(wchar_t) || bytes > 65536 ||
      bytes % sizeof(wchar_t) != 0 || text.get()[bytes / sizeof(wchar_t) - 1] != L'\0') {
    throw std::runtime_error("Cannot identify the signed-in account for data removal");
  }
  return std::wstring(text.get());
}

void VerifySessionAccount(DWORD session_id, PSID expected) {
  const auto user = SessionAccountPart(session_id, WTSUserName);
  const auto domain = SessionAccountPart(session_id, WTSDomainName);
  if (user.empty()) throw std::runtime_error("Data removal requires a signed-in user account");
  const auto account = domain.empty() ? user : domain + L"\\" + user;
  DWORD sid_size = 0;
  DWORD domain_size = 0;
  SID_NAME_USE kind{};
  LookupAccountNameW(nullptr, account.c_str(), nullptr, &sid_size, nullptr, &domain_size, &kind);
  if (GetLastError() != ERROR_INSUFFICIENT_BUFFER || sid_size == 0 || sid_size > 65536 ||
      domain_size > 32768) {
    throw std::runtime_error("Cannot resolve the signed-in account for data removal");
  }
  std::vector<BYTE> sid(sid_size);
  std::vector<wchar_t> resolved_domain(static_cast<size_t>(domain_size) + 1);
  if (!LookupAccountNameW(nullptr, account.c_str(), sid.data(), &sid_size,
                         resolved_domain.data(), &domain_size, &kind) ||
      kind != SidTypeUser || !EqualSid(sid.data(), expected)) {
    throw std::runtime_error("Data removal cannot use another account's elevation credentials");
  }
}

}  // namespace

bool ShouldRemoveUserData(const UserDataRemovalRequest& request) {
  return request.installed && request.remove == L"ALL" && !request.upgrading &&
      request.choice == L"1";
}

CurrentUserData ResolveCurrentUserData(const std::wstring& expected_sid) {
  const auto expected = ParseUserSid(expected_sid);
  const auto token = CurrentToken();
  DWORD required = 0;
  GetTokenInformation(token.get(), TokenUser, nullptr, 0, &required);
  if (GetLastError() != ERROR_INSUFFICIENT_BUFFER || required == 0 || required > 65536) {
    throw std::runtime_error("Cannot read the data-removal account token");
  }
  std::vector<BYTE> bytes(required);
  if (!GetTokenInformation(token.get(), TokenUser, bytes.data(), required, &required)) {
    throw std::runtime_error("Cannot read the data-removal account token");
  }
  const auto user = reinterpret_cast<const TOKEN_USER*>(bytes.data());
  if (!EqualSid(user->User.Sid, expected.get())) {
    throw std::runtime_error("The data-removal account changed during elevation");
  }
  DWORD session_id = 0;
  if (!GetTokenInformation(token.get(), TokenSessionId, &session_id, sizeof(session_id),
                           &required) || session_id == 0) {
    throw std::runtime_error("Data removal requires an interactive user session");
  }
  // Burn can elevate before starting MSI; UserSID alone can then name the alternate administrator.
  VerifySessionAccount(session_id, user->User.Sid);
  PWSTR raw = nullptr;
  const HRESULT result = SHGetKnownFolderPath(FOLDERID_LocalAppData,
                                              KF_FLAG_DONT_VERIFY | KF_FLAG_NO_PACKAGE_REDIRECTION,
                                              token.get(), &raw);
  const std::unique_ptr<wchar_t, decltype(&CoTaskMemFree)> folder(raw, CoTaskMemFree);
  if (FAILED(result) || !folder || !*folder) {
    throw std::runtime_error("Cannot resolve the current account's local application data");
  }
  return {CanonicalUserSid(user->User.Sid), std::filesystem::path(folder.get())};
}

FileHandle AcquireUserDataCleanupLease(const std::wstring& sid) {
  const auto identity = ParseUserSid(sid);
  const auto name = L"Global\\Cedarflake.Ame.SingleInstance." + CanonicalUserSid(identity.get());
  SetLastError(ERROR_SUCCESS);
  FileHandle mutex(CreateMutexW(nullptr, FALSE, name.c_str()));
  const DWORD error = GetLastError();
  if (!mutex || error == ERROR_ALREADY_EXISTS) {
    throw std::runtime_error("Close all Ame windows before removing Ame data");
  }
  return mutex;
}

}  // namespace ame::installer
