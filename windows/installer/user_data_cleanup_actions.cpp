#include "user_data_cleanup.h"

#include <msiquery.h>

#include <stdexcept>

namespace {

std::wstring Property(MSIHANDLE session, const wchar_t* name) {
  DWORD required = 0;
  const UINT result = MsiGetPropertyW(session, name, L"", &required);
  if ((result != ERROR_MORE_DATA && result != ERROR_SUCCESS) || required > 1024) {
    throw std::runtime_error("Cannot read the uninstall data-removal request");
  }
  std::vector<wchar_t> text(static_cast<size_t>(required) + 1);
  DWORD capacity = static_cast<DWORD>(text.size());
  if (MsiGetPropertyW(session, name, text.data(), &capacity) != ERROR_SUCCESS) {
    throw std::runtime_error("Cannot read the uninstall data-removal request");
  }
  return std::wstring(text.data(), capacity);
}

void Report(MSIHANDLE session, INSTALLMESSAGE type, int message,
            const char* detail) noexcept {
  const MSIHANDLE record = MsiCreateRecord(2);
  if (record) {
    MsiRecordSetInteger(record, 1, message);
    MsiRecordSetStringA(record, 2, detail);
    MsiProcessMessage(session, type, record);
    MsiCloseHandle(record);
  }
}

void BeginCommittedCleanup(MSIHANDLE session) {
  const MSIHANDLE record = MsiCreateRecord(2);
  if (!record) throw std::runtime_error("Cannot update uninstall completion progress");
  MsiRecordSetInteger(record, 1, 2);
  MsiRecordSetInteger(record, 2, 0);
  const int result = MsiProcessMessage(session, INSTALLMESSAGE_COMMONDATA, record);
  MsiCloseHandle(record);
  if (result == IDCANCEL || result == -1) {
    throw std::runtime_error("Data removal did not start after the uninstall transaction");
  }
}

}  // namespace

extern "C" __declspec(dllexport) UINT __stdcall AmePrepareUserDataRemoval(MSIHANDLE session) {
  try {
    using namespace ame::installer;
    if (MsiSetPropertyW(session, L"AmeRemoveUserData", L"") != ERROR_SUCCESS) {
      throw std::runtime_error("Cannot initialize the uninstall data-removal request");
    }
    const UserDataRemovalRequest request{!Property(session, L"Installed").empty(),
        Property(session, L"REMOVE"), !Property(session, L"UPGRADINGPRODUCTCODE").empty(),
        Property(session, L"AME_REMOVE_USER_DATA")};
    if (!ShouldRemoveUserData(request)) return ERROR_SUCCESS;
    const auto user = ResolveCurrentUserData(Property(session, L"UserSID"));
    const auto lease = AcquireUserDataCleanupLease(user.sid);
    AmeUserDataDirectory(user.local_app_data);
    if (MsiSetPropertyW(session, L"AmeRemoveUserData", user.sid.c_str()) != ERROR_SUCCESS) {
      throw std::runtime_error("Cannot retain the data-removal account identity");
    }
    return ERROR_SUCCESS;
  } catch (const std::exception& error) {
    Report(session, INSTALLMESSAGE_ERROR,
        25002,
        error.what());
    return ERROR_INSTALL_FAILURE;
  } catch (...) {
    return ERROR_INSTALL_FAILURE;
  }
}

extern "C" __declspec(dllexport) UINT __stdcall AmeRemoveUserData(MSIHANDLE session) {
  try {
    if (!MsiGetMode(session, MSIRUNMODE_COMMIT)) {
      throw std::runtime_error("User data can only be removed after uninstall has committed");
    }
    const auto user = ame::installer::ResolveCurrentUserData(Property(session, L"CustomActionData"));
    const auto lease = ame::installer::AcquireUserDataCleanupLease(user.sid);
    BeginCommittedCleanup(session);
    ame::installer::RemoveAmeUserData(user.local_app_data);
  } catch (const std::exception& error) {
    Report(session, INSTALLMESSAGE_WARNING,
        25003, error.what());
  } catch (...) {
    Report(session, INSTALLMESSAGE_WARNING,
        25003, "Unexpected cleanup failure");
  }
  // Cleanup cannot restore deleted user data; never roll back a completed uninstall on its failure.
  return ERROR_SUCCESS;
}
